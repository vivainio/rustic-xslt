#!/usr/bin/env python3
"""Run the unmodified SchXslt2 compiler and its generated validators."""
import argparse
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from run import ROOT, invoke, resources, revision

SVRL = '{http://purl.oclc.org/dsdl/svrl}'
FIXTURES = ROOT / 'tools/workloads/schematron'


def check_report(output, case, invalid):
    root = ET.fromstring(output)
    assert root.tag == SVRL + 'schematron-output', 'not an SVRL report'
    failures = root.findall('.//' + SVRL + 'failed-assert')
    reports = root.findall('.//' + SVRL + 'successful-report')
    assert len(failures) == int(invalid and case != 'upstream-accumulator'), 'unexpected failed assertions'
    assert len(reports) == int(invalid and case in ('inventory', 'upstream-accumulator')), 'unexpected successful reports'
    if case == 'upstream-accumulator':
        assert reports[0].get('test') == "accumulator-after('accumulator') eq 2" and reports[0].get('location') == '/Q{}root[1]', 'incorrect accumulator report'
    if case == 'selected-phase':
        assert root.get('phase') == 'alternative', 'selected phase was not applied'
        assert not any(p.get('id') == 'quantities' for p in root.findall('.//' + SVRL + 'active-pattern')), 'unselected pattern ran'
    if case in ('typical', 'whitespace', 'whitespace-preserved'):
        assert root.get('phase') == 'basic', 'incorrect default phase'
        assert not any(p.get('id') == 'inactive' for p in root.findall('.//' + SVRL + 'active-pattern')), 'inactive pattern ran'
    if failures:
        failure = failures[0]
        expected = {'inventory': '@id', 'typical': '$quantity ge $minimum', 'whitespace': '$quantity ge $minimum', 'whitespace-preserved': '$quantity ge $minimum', 'upstream-root': 'false()', 'selected-phase': 'false()'}
        location = '/Q{}root[1]' if case == 'upstream-root' else '/Q{urn:inventory}inventory[1]/Q{urn:inventory}item[1]'
        assert failure.get('test') == expected[case] and failure.get('location') == location, 'incorrect failure test or location'
        text = ''.join(failure.itertext())
        if case == 'inventory':
            assert 'An item needs an id.' in text, 'missing failure message'
            assert reports[0].get('test') == "@discontinued = 'yes'" and reports[0].get('location') == location, 'incorrect report test or location'
            assert 'Discontinued item.' in ''.join(reports[0].itertext()), 'missing report message'
        elif case in ('typical', 'whitespace', 'whitespace-preserved'):
            message = 'Quantity for item bad must be at least two.' if case in ('whitespace', 'whitespace-preserved') else 'Quantity for item named bad must be at least two.'
            assert message in text, 'message whitespace or dynamic content is incorrect'
            diagnostic = failure.find(SVRL + 'diagnostic-reference')
            assert diagnostic is not None and diagnostic.get('diagnostic') == 'quantity-info', 'missing diagnostic reference'
            assert 'Check the item quantity.' in ''.join(diagnostic.itertext()), 'missing diagnostic text'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--generated', type=Path, help='save the generated inventory validator here')
    args = parser.parse_args()
    sch = args.schxslt2.resolve()
    print('SchXslt2 revision:', revision(sch), flush=True)
    subprocess.run(['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance'], cwd=ROOT, check=True)
    driver = ROOT / 'target/debug/examples/reference_driver'
    transpiler = sch / 'src/main/resources/content/transpile.xsl'
    modules = resources(sch)
    upstream = sch / 'src/test/bugs/rule-context-root.xml'
    fixture = upstream.read_text()
    upstream_schema = re.search(r'<sch:schema\b.*?</sch:schema>', fixture, re.DOTALL).group()
    upstream_accumulator = sch / 'src/test/features/xsl-accumulators.xml'
    accumulator_schema = re.search(r'<sch:schema\b.*?</sch:schema>', upstream_accumulator.read_text(), re.DOTALL).group()
    cases = [
        ('inventory', (FIXTURES / 'inventory.sch').read_text(), FIXTURES / 'inventory.sch',
         [('valid', FIXTURES / 'valid.xml', False), ('invalid', FIXTURES / 'invalid.xml', True)]),
        ('typical', (FIXTURES / 'typical.sch').read_text(), FIXTURES / 'typical.sch',
         [('valid', FIXTURES / 'typical-valid.xml', False), ('invalid', FIXTURES / 'typical-invalid.xml', True)]),
        ('whitespace', (FIXTURES / 'typical.sch').read_text().replace('<name/> named <value-of', '<name/> <value-of'), FIXTURES / 'typical.sch',
         [('invalid', FIXTURES / 'typical-invalid.xml', True)]),
        ('whitespace-preserved', (FIXTURES / 'typical.sch').read_text().replace('<name/> named <value-of', '<name/> <value-of').replace('<assert test="$quantity ge $minimum"', '<assert xml:space="preserve" test="$quantity ge $minimum"'), FIXTURES / 'typical.sch',
         [('invalid', FIXTURES / 'typical-invalid.xml', True)]),
        ('selected-phase', (FIXTURES / 'typical.sch').read_text(), FIXTURES / 'typical.sch',
         [('invalid', FIXTURES / 'typical-valid.xml', True)]),
        ('upstream-root', upstream_schema, upstream, [('invalid', None, True)]),
        ('upstream-accumulator', accumulator_schema, upstream_accumulator, [('invalid', None, True)]),
    ]
    blocked = False
    with tempfile.TemporaryDirectory(prefix='schematron-') as directory:
        for case, schema, schema_path, inputs in cases:
            params = {'Q{http://dmaus.name/ns/2023/schxslt}phase': 'alternative'} if case == 'selected-phase' else None
            result = invoke(driver, transpiler, schema, modules, schema_path.resolve().as_uri(), params)
            if result['diagnostics']:
                phase = result['diagnostics'][0]['phase']
                print('BLOCKED', case, 'transpiler compile' if phase == 'compile' else 'transpilation', result['diagnostics'][0])
                blocked = True
                continue
            generated = result['output']
            if args.generated and case == 'inventory':
                args.generated.write_text(generated)
            validator = Path(directory) / (case + '.xsl')
            validator.write_text(generated)
            for name, source_path, invalid in inputs:
                source = source_path.read_text() if source_path else '<root/>'
                source_uri = source_path.resolve().as_uri() if source_path else schema_path.resolve().as_uri()
                result = invoke(driver, validator, source, {}, source_uri)
                if result['diagnostics']:
                    phase = result['diagnostics'][0]['phase']
                    print('BLOCKED', case, name, 'validator compile' if phase == 'compile' else 'validation', result['diagnostics'][0])
                    blocked = True
                    continue
                try:
                    check_report(result['output'], case, invalid)
                except (AssertionError, ET.ParseError) as error:
                    print('FAIL', case, name, str(error))
                    blocked = True
                    continue
                print('PASS', case, name, 'SVRL matches expected validation outcome')
            if case == 'typical':
                items = ''.join(f'<item id="item{i}" quantity="{1 if i % 10 == 0 else 2}"/>' for i in range(500))
                source = '<inventory xmlns="urn:inventory">' + items + '</inventory>'
                result = invoke(driver, validator, source, {}, schema_path.resolve().as_uri())
                if result['diagnostics']:
                    print('BLOCKED', case, '500-items', result['diagnostics'][0])
                    blocked = True
                else:
                    try:
                        report = ET.fromstring(result['output'])
                        failures = report.findall('.//' + SVRL + 'failed-assert')
                        fired = report.findall('.//' + SVRL + 'fired-rule')
                        assert len(fired) == 500 and len(failures) == 50, 'incorrect number of fired rules or failures'
                        text = ''.join(''.join(node.itertext()) for node in failures)
                        assert 'item0' in text and 'item490' in text, 'missing first or last failure'
                        assert failures[0].get('location', '').endswith('/Q{urn:inventory}item[1]'), 'incorrect first failure location'
                        assert failures[-1].get('location', '').endswith('/Q{urn:inventory}item[491]'), 'incorrect last failure location'
                    except (AssertionError, ET.ParseError) as error:
                        print('FAIL', case, '500-items', str(error))
                        blocked = True
                    else:
                        print('PASS', case, '500-items', '500 rules fired and 50 assertions failed')
    return int(blocked)


if __name__ == '__main__':
    sys.exit(main())
