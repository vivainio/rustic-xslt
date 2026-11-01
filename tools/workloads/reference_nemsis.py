#!/usr/bin/env python3
"""Run NEMSIS's unmodified healthcare schema and samples against SaxonC."""
import argparse
import re
from collections import Counter
import subprocess
import tempfile
from pathlib import Path
from saxonche import PySaxonProcessor
from reference_peppol import signature
from run import ROOT, invoke, resources, revision


def comparable_signature(output):
    fired, failures, reports = signature(output)
    # The d004 diagnostic includes the processor's current local time and zone.
    # Compare all other text and fields while allowing those to differ.
    def normalize(items):
        return Counter({(ident, test, location,
                         re.sub(r'(current time according to this system is )[^)]*', r'\1<time>', text)
                         if ident == 'nemSch_d004' else text): count
                        for (ident, test, location, text), count in items.items()})
    return fired, normalize(failures), normalize(reports)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--nemsis', type=Path, required=True)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=20_000_000)
    args = parser.parse_args()
    nemsis, schxslt = args.nemsis.resolve(), args.schxslt2.resolve()
    print('NEMSIS revision:', revision(nemsis), flush=True)
    print('SchXslt2 revision:', revision(schxslt), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    schema = nemsis / 'Schematron/rules/DEMDataSet.sch'
    suite = nemsis / 'SampleData/Schematron/DEM'
    compiler_dir = nemsis / 'Schematron/DevelopmentKit/Schematron/utilities/iso-schematron-xslt2'
    compiler = compiler_dir / 'iso_svrl_for_xslt2.xsl'
    modern = schxslt / 'src/main/resources/content/transpile.xsl'
    cases = sorted((suite / 'xml').glob('DEMDataSet*.xml'))
    if len(cases) != 20:
        raise ValueError(f'expected 20 pinned NEMSIS sample documents, found {len(cases)}')

    def invoke_checked(style, source, uri=None, available=None, params=None):
        result = invoke(driver, style, source, available or {}, uri, params, max_steps=args.max_steps)
        if result['diagnostics']:
            raise RuntimeError(result['diagnostics'][0])
        return result['output']

    with tempfile.TemporaryDirectory(prefix='nemsis-reference-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        xslt = processor.new_xslt30_processor()
        iso_compiler = xslt.compile_stylesheet(stylesheet_file=str(compiler))
        iso_compiler.set_parameter('allow-foreign', processor.make_string_value('true'))
        validators = {
            'saxon': iso_compiler.transform_to_string(source_file=str(schema)),
            'rust': invoke_checked(compiler, schema.read_text(), schema.as_uri(), resources(compiler_dir),
                                   {'allow-foreign': 'true'}),
        }
        baseline = {}
        for producer, generated in validators.items():
            validator = Path(tmp) / (producer + '-iso-validator.xsl')
            validator.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
            for source in cases:
                name = source.name
                expected = suite / 'svrl' / name
                if not expected.is_file():
                    raise FileNotFoundError(f'missing upstream expected report: {expected}')
                outputs = {
                    'rust': invoke_checked(validator, source.read_text(), source.as_uri()),
                    'saxon': saxon_validator.transform_to_string(source_file=str(source)),
                }
                official = signature(expected.read_text())
                def ids(result):
                    return sum((Counter({item[0]: count}) for item, count in result[1].items()), Counter())
                for executor, output in outputs.items():
                    result = comparable_signature(output)
                    if not result[0] or ids(result) != ids(official) or (name in baseline and result != baseline[name]):
                        raise AssertionError(f'{producer}/{executor} {name}: {result}, expected IDs {ids(official)}, baseline {baseline.get(name)}')
                    baseline[name] = result
                    print('PASS', producer, 'compiler /', executor, 'validator:', name,
                          'fired:', sum(result[0].values()), 'failures:', dict(ids(result)), flush=True)

        modern_output = invoke_checked(modern, schema.read_text(), schema.as_uri(), resources(schxslt))
        modern_validator = Path(tmp) / 'nemsis-schxslt2-validator.xsl'
        modern_validator.write_text(modern_output)
        modern_result = invoke(driver, modern_validator, (suite / 'xml/DEMDataSet--Base.xml').read_text(), {}, max_steps=args.max_steps)
        if modern_result['diagnostics']:
            print('LIMITATION SchXslt2 generated validator:', modern_result['diagnostics'][0], flush=True)
        else:
            print('PASS SchXslt2 generated validator ran', flush=True)


if __name__ == '__main__':
    main()
