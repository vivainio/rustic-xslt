#!/usr/bin/env python3
"""Cross-check HL7 C-CDA Schematron and its vocabulary against SaxonC."""
import argparse
import subprocess
import tempfile
from pathlib import Path

from saxonche import PySaxonProcessor

from reference_peppol import signature
from run import ROOT, revision


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ccda', type=Path, required=True)
    parser.add_argument('--nemsis', type=Path, required=True,
                        help='checkout containing the ISO Schematron compiler')
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=200_000_000)
    parser.add_argument('--max-depth', type=int, default=2048)
    args = parser.parse_args()
    ccda, nemsis = args.ccda.resolve(), args.nemsis.resolve()
    print('C-CDA revision:', revision(ccda), flush=True)
    print('ISO compiler (NEMSIS) revision:', revision(nemsis), flush=True)
    compiler = nemsis / 'Schematron/DevelopmentKit/Schematron/utilities/iso-schematron-xslt2/iso_svrl_for_xslt2.xsl'
    schema = ccda / 'validation/Consolidated CDA Templates for Clinical Notes (US Realm) DSTU R2.1.sch'
    vocabulary = ccda / 'validation/voc.xml'
    sample = ccda / 'examples/C-CDA_R2-1_CCD.xml'
    build = ['cargo', 'build', '--example', 'schematron']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/schematron'
    template = ('<supply xmlns="urn:hl7-org:v3" classCode="SPLY" moodCode="{}">'
                '<templateId root="2.16.840.1.113883.10.20.22.4.50" extension="2014-06-09"/>'
                '<id root="1.2.3"/><statusCode code="completed"/></supply>')
    with tempfile.TemporaryDirectory(prefix='ccda-reference-') as directory, PySaxonProcessor(license=False) as processor:
        tmp = Path(directory)
        print('Reference:', processor.version, flush=True)
        (tmp / 'voc.xml').symlink_to(vocabulary)
        inputs = [('upstream', sample, set())]
        for value, expected in [('EVN', set()), ('INVALID', {'a-1098-8746'})]:
            path = tmp / (value + '.xml')
            path.write_text(template.format(value))
            inputs.append((value, path, expected))
        limited = subprocess.run([str(driver), str(compiler), str(schema), '--iso', '--phase', 'errors',
                                  '--max-steps', '1', str(sample)], cwd=ROOT, text=True,
                                 capture_output=True, timeout=300)
        if limited.returncode == 0 or 'Schematron compiler:' not in limited.stderr or 'XPDY0130' not in limited.stderr:
            raise AssertionError(f'compiler limit did not identify its stage: {limited.stderr}')
        print('PASS compiler work-limit diagnostic identifies the stage', flush=True)
        xslt = processor.new_xslt30_processor()
        compiler_xslt = xslt.compile_stylesheet(stylesheet_file=str(compiler))
        compiler_xslt.set_parameter('phase', processor.make_string_value('errors'))
        compiler_xslt.set_parameter('allow-foreign', processor.make_string_value('true'))
        validator_file = tmp / 'saxon-validator.xsl'
        validator_file.write_text(compiler_xslt.transform_to_string(source_file=str(schema)))
        validator = xslt.compile_stylesheet(stylesheet_file=str(validator_file))
        for name, path, expected in inputs:
            rust = subprocess.run([str(driver), str(compiler), str(schema), '--iso', '--phase', 'errors',
                                   '--max-steps', str(args.max_steps), '--max-depth', str(args.max_depth), str(path)],
                                  cwd=ROOT, text=True, capture_output=True, check=True, timeout=300)
            actual = signature(rust.stdout)
            reference = signature(validator.transform_to_string(source_file=str(path)))
            failed = {entry[0] for entry in actual[1]}
            if not actual[0] or failed != expected or actual != reference:
                raise AssertionError(f'{name}: fired={sum(actual[0].values())}, failed={failed}, '
                                     f'expected={expected}, Saxon agrees={actual == reference}')
            print('PASS', name, 'fired:', sum(actual[0].values()), 'failed IDs:', sorted(failed), flush=True)


if __name__ == '__main__':
    main()
