#!/usr/bin/env python3
"""Compare an HL7 C-CDA schema on the upstream clinical example corpus."""
import argparse
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from saxonche import PySaxonProcessor

from reference_peppol import signature
from run import ROOT, revision


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--examples', type=Path, required=True)
    parser.add_argument('--nemsis', type=Path, required=True)
    parser.add_argument('--version', choices=('2.1', '3.0', '4.0'), default='3.0')
    parser.add_argument('--phase', choices=('errors', 'warnings'), default='errors')
    parser.add_argument('--vocabulary', type=Path,
                        help='voc.xml required for the 2.1 schema')
    parser.add_argument('--allow-compiler-messages', action='store_true',
                        help='continue after compiler messages; inspect stderr for incomplete rules')
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=200_000_000)
    parser.add_argument('--max-depth', type=int, default=2048)
    parser.add_argument('--expected-count', type=int, default=137,
                        help='expected number of examples (default: pinned corpus size, 137)')
    args = parser.parse_args()
    if args.version == '2.1' and args.vocabulary is None:
        parser.error('--version 2.1 requires --vocabulary pointing to voc.xml')
    examples, nemsis = args.examples.resolve(), args.nemsis.resolve()
    print('HL7 examples revision:', revision(examples), flush=True)
    print('ISO compiler (NEMSIS) revision:', revision(nemsis), flush=True)
    schema = examples / f'ccda-{args.version}.sch'
    compiler = nemsis / 'Schematron/DevelopmentKit/Schematron/utilities/iso-schematron-xslt2/iso_svrl_for_xslt2.xsl'
    inputs = sorted((examples / 'input/examples').glob('*.xml'))
    if len(inputs) != args.expected_count or not inputs:
        raise ValueError(f'expected {args.expected_count} clinical examples, found {len(inputs)}')
    build = ['cargo', 'build', '--example', 'schematron']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/schematron'
    with tempfile.TemporaryDirectory(prefix='cda-examples-') as directory, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        if args.version == '2.1':
            local_schema = Path(directory) / schema.name
            shutil.copyfile(schema, local_schema)
            shutil.copyfile(args.vocabulary, Path(directory) / 'voc.xml')
            schema = local_schema
        xslt = processor.new_xslt30_processor()
        compiler_xslt = xslt.compile_stylesheet(stylesheet_file=str(compiler))
        compiler_xslt.set_parameter('phase', processor.make_string_value(args.phase))
        compiler_xslt.set_parameter('allow-foreign', processor.make_string_value('true'))
        validator_file = Path(directory) / 'validator.xsl'
        validator_file.write_text(compiler_xslt.transform_to_string(source_file=str(schema)))
        validator = xslt.compile_stylesheet(stylesheet_file=str(validator_file))
        command = [str(driver), str(compiler), str(schema), '--iso', '--phase', args.phase,
                   '--max-steps', str(args.max_steps), '--max-depth', str(args.max_depth)]
        if args.allow_compiler_messages:
            command.append('--allow-compiler-messages')
        rust = subprocess.run([*command, *(str(path) for path in inputs)], cwd=ROOT, text=True,
                              capture_output=True, timeout=600)
        if rust.returncode:
            raise RuntimeError(f'rust-xslt failed: {rust.stderr.strip()}')
        for line in rust.stderr.splitlines():
            if line.startswith('Schematron compiler: '):
                print('COMPILER MESSAGE:', line, flush=True)
        reports = re.split(r'(?=<\?xml\s)', rust.stdout.strip())
        reports = [report for report in reports if report.strip()]
        if len(reports) != len(inputs):
            raise AssertionError(f'expected {len(inputs)} SVRL documents, got {len(reports)}')
        fired_total = failed_total = empty = 0
        for path, report in zip(inputs, reports):
            actual = signature(report)
            reference = signature(validator.transform_to_string(source_file=str(path)))
            if actual != reference:
                raise AssertionError(f'{path.name}: report differs from Saxon')
            fired, failures, _ = actual
            fired_total += sum(fired.values())
            failed_total += sum(failures.values())
            empty += not bool(fired)
            print('PASS', path.name, 'fired:', sum(fired.values()), 'failed:', sum(failures.values()), flush=True)
        if not fired_total:
            raise AssertionError('no rules fired across the corpus')
        print('PASS total:', len(inputs), 'documents,', fired_total, 'rules fired,',
              failed_total, 'failed assertions,', empty, 'documents without fired rules', flush=True)


if __name__ == '__main__':
    main()
