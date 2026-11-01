#!/usr/bin/env python3
"""Run NEMSIS's unmodified healthcare schema and samples against SaxonC."""
import argparse
from collections import Counter
import subprocess
import tempfile
from pathlib import Path
from saxonche import PySaxonProcessor
from reference_peppol import signature
from run import ROOT, invoke, resources, revision


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
        validator = Path(tmp) / 'nemsis-iso-validator.xsl'
        validator.write_text(iso_compiler.transform_to_string(source_file=str(schema)))
        saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
        for name in ('DEMDataSet--Base.xml', 'DEMDataSet-nemSch_d001_A.xml'):
            source, expected = suite / 'xml' / name, suite / 'svrl' / name
            rust = signature(invoke_checked(validator, source.read_text(), source.as_uri()))
            saxon = signature(saxon_validator.transform_to_string(source_file=str(source)))
            official = signature(expected.read_text())
            def ids(result):
                return sum((Counter({item[0]: count}) for item, count in result[1].items()), Counter())
            if not rust[0] or rust != saxon or ids(rust) != ids(official):
                raise AssertionError(f'{name}: Rust {rust}, Saxon {saxon}, expected IDs {ids(official)}')
            print('PASS', name, 'fired:', sum(rust[0].values()), 'failures:', dict(ids(rust)), flush=True)

        iso_result = invoke(driver, compiler, schema.read_text(), resources(compiler_dir), schema.as_uri(),
                            {'allow-foreign': 'true'}, max_steps=args.max_steps)
        if iso_result['diagnostics']:
            print('LIMITATION Rust cannot compile NEMSIS ISO compiler:', iso_result['diagnostics'][0], flush=True)
        else:
            print('PASS Rust compiled the NEMSIS ISO compiler', flush=True)
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
