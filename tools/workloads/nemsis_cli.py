#!/usr/bin/env python3
"""Check the public Schematron example with NEMSIS schemas and samples."""
import argparse
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from run import ROOT, revision

SVRL = '{http://purl.oclc.org/dsdl/svrl}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--nemsis', type=Path, required=True)
    parser.add_argument('--release', action='store_true')
    args = parser.parse_args()
    nemsis = args.nemsis.resolve()
    print('NEMSIS revision:', revision(nemsis), flush=True)
    build = ['cargo', 'build', '--example', 'schematron']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    cli = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/schematron'
    compiler = nemsis / 'Schematron/DevelopmentKit/Schematron/utilities/iso-schematron-xslt2/iso_svrl_for_xslt2.xsl'
    for dataset, failure in [('DEM', 'nemSch_d001'), ('EMS', 'nemSch_e001')]:
        schema = nemsis / f'Schematron/rules/{dataset}DataSet.sch'
        suite = nemsis / f'SampleData/Schematron/{dataset}'
        names = [f'{dataset}DataSet--Base.xml', f'{dataset}DataSet-{failure}_A.xml']
        inputs = [suite / 'xml' / name for name in names]
        process = subprocess.run([str(cli), str(compiler), str(schema), '--iso', *map(str, inputs)],
                                 text=True, capture_output=True, check=True, timeout=120)
        reports = [chunk for chunk in re.split(r'(?=<\?xml\s)', process.stdout) if chunk.strip()]
        if len(reports) != len(inputs):
            raise AssertionError(f'expected {len(inputs)} SVRL reports; got {len(reports)}')
        for name, report in zip(names, reports):
            root = ET.fromstring(report)
            expected = ET.parse(suite / 'svrl' / name).getroot()
            actual_ids = [node.get('id') for node in root.findall('.//' + SVRL + 'failed-assert')]
            expected_ids = [node.get('id') for node in expected.findall('.//' + SVRL + 'failed-assert')]
            if root.tag != SVRL + 'schematron-output' or not root.findall('.//' + SVRL + 'fired-rule') or actual_ids != expected_ids:
                raise AssertionError(f'{name}: actual failure IDs {actual_ids}, expected {expected_ids}')
            print('PASS', dataset, name, 'failure IDs:', actual_ids, flush=True)

    fixture = ROOT / 'tools/workloads/schematron'
    with tempfile.TemporaryDirectory(prefix='nemsis-cli-') as tmp:
        schema = Path(tmp) / 'phase.sch'
        schema.write_text((fixture / 'typical.sch').read_text().replace('queryBinding="xslt3"', 'queryBinding="xslt2"'))
        process = subprocess.run([str(cli), str(compiler), str(schema), '--iso', '--phase', 'alternative',
                                  str(fixture / 'typical-valid.xml')], text=True, capture_output=True,
                                 check=True, timeout=120)
        root = ET.fromstring(process.stdout)
        failures = root.findall('.//' + SVRL + 'failed-assert')
        if root.get('phase') != 'alternative' or [node.get('test') for node in failures] != ['false()']:
            raise AssertionError('ISO phase selection did not run the alternative pattern')
        print('PASS ISO phase selection', flush=True)

        # The ISO include processor resolves against the schema document URI.
        nested = Path(tmp) / 'nested'
        nested.mkdir()
        (nested / 'rules.sch').write_text((fixture / 'includes/nested/rules.sch').read_text())
        for filename in ('abstract.sch', 'includes/main.sch', 'includes/main-fragment.sch'):
            source = (fixture / filename).read_text().replace('queryBinding="xslt3"', 'queryBinding="xslt2"')
            source = source.replace(' xml:base="nested/"', '').replace('href="rules.sch', 'href="nested/rules.sch')
            schema.write_text(source)
            process = subprocess.run([str(cli), str(compiler), str(schema), '--iso', '--iso-preprocess',
                                      str(fixture / 'valid.xml'), str(fixture / 'invalid.xml')],
                                     text=True, capture_output=True, check=True, timeout=120)
            reports = [ET.fromstring(chunk) for chunk in re.split(r'(?=<\?xml\s)', process.stdout) if chunk.strip()]
            if len(reports) != 2 or any(not report.findall('.//' + SVRL + 'fired-rule') for report in reports):
                raise AssertionError(f'{filename}: expected two reports with fired rules')
            failures = [[node.get('test') for node in report.findall('.//' + SVRL + 'failed-assert')] for report in reports]
            if failures != [[], ['@id']]:
                raise AssertionError(f'{filename}: incorrect assertion results: {failures}')
            print('PASS ISO preprocessing', filename, flush=True)


if __name__ == '__main__':
    main()
