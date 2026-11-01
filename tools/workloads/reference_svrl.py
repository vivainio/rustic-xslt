#!/usr/bin/env python3
"""Cross-check the upstream SVRL Schematron on a generated validation report."""
import argparse
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from saxonche import PySaxonProcessor
from run import ROOT, invoke, resources, revision
from reference_peppol import signature

SVRL = '{http://purl.oclc.org/dsdl/svrl}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', required=True, type=Path)
    parser.add_argument('--schema', required=True, type=Path, help='Schematron/schema checkout')
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=20_000_000)
    args = parser.parse_args()
    sch, schema_repo = args.schxslt2.resolve(), args.schema.resolve()
    print('SchXslt2 revision:', revision(sch), flush=True)
    print('Schematron/schema revision:', revision(schema_repo), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    transpiler = sch / 'src/main/resources/content/transpile.xsl'
    modules = resources(sch)
    fixture = ROOT / 'tools/workloads/schematron'

    def run(style, source, source_uri=None, available=None):
        result = invoke(driver, style, source, available or {}, source_uri, max_steps=args.max_steps)
        if result['diagnostics']:
            raise RuntimeError(result['diagnostics'][0])
        return result['output']

    with tempfile.TemporaryDirectory(prefix='svrl-reference-') as directory, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        directory = Path(directory)
        inventory_schema = fixture / 'inventory.sch'
        inventory_validator = directory / 'inventory.xsl'
        inventory_validator.write_text(run(transpiler, inventory_schema.read_text(), inventory_schema.as_uri(), modules))
        report = run(inventory_validator, (fixture / 'invalid.xml').read_text(), (fixture / 'invalid.xml').as_uri())
        root = ET.fromstring(report)
        failure = root.find('.//' + SVRL + 'failed-assert')
        assert failure is not None and failure.get('location'), 'fixture did not generate the expected failure'
        # Exercise an abstract-pattern parameter and an inherited abstract rule.
        del failure.attrib['location']
        active = root.find('.//' + SVRL + 'active-pattern')
        assert active is not None, 'fixture did not generate an active pattern'
        ET.SubElement(active, SVRL + 'text').text = 'unexpected child'
        damaged = ET.tostring(root, encoding='unicode')

        schema = schema_repo / 'svrl.sch'
        xslt = processor.new_xslt30_processor()
        saxon_transpiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        validators = {
            'rust': run(transpiler, schema.read_text(), schema.as_uri(), modules),
            'saxon': saxon_transpiler.transform_to_string(source_file=str(schema)),
        }
        baseline = {}
        for producer, generated in validators.items():
            validator = directory / (producer + '.xsl')
            validator.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
            for name, source in [('generated-report', report), ('damaged-report', damaged)]:
                outputs = {
                    'rust': run(validator, source),
                    'saxon': saxon_validator.transform_to_string(xdm_node=processor.parse_xml(xml_text=source)),
                }
                for executor, output in outputs.items():
                    result = signature(output)
                    if not result[0]:
                        raise AssertionError('no rules fired')
                    if name == 'damaged-report':
                        tests = {item[1] for item in result[1]}
                        if not {'count(*)=0', 'string-length( @location ) > 0'} <= tests:
                            raise AssertionError(f'expected inherited and parameterized assertions, got {tests}')
                    if name in baseline and result != baseline[name]:
                        raise AssertionError(f'{producer} transpiler / {executor} validator disagrees on {name}: {result} vs {baseline[name]}')
                    baseline[name] = result
                    print('PASS', producer, 'transpiler /', executor, 'validator:', name,
                          'fired:', sum(result[0].values()), 'failed:', sum(result[1].values()), flush=True)


        constraints = schema_repo / 'schematron.sch'
        constraint_validators = {
            'rust': run(transpiler, constraints.read_text(), constraints.as_uri(), modules),
            'saxon': saxon_transpiler.transform_to_string(source_file=str(constraints)),
        }
        altered_schema = ET.fromstring(schema.read_text())
        extension = altered_schema.find('.//{http://purl.oclc.org/dsdl/schematron}extends')
        assert extension is not None
        extension.set('rule', 'missing-rule')
        broken_schema = ET.tostring(altered_schema, encoding='unicode')
        baseline = {}
        for producer, generated in constraint_validators.items():
            validator = directory / (producer + '-constraints.xsl')
            validator.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
            for name, source in [('upstream-schema', schema.read_text()), ('missing-abstract-rule', broken_schema)]:
                outputs = {
                    'rust': run(validator, source),
                    'saxon': saxon_validator.transform_to_string(xdm_node=processor.parse_xml(xml_text=source)),
                }
                for executor, output in outputs.items():
                    result = signature(output)
                    expected_test = "//sch:rule[@abstract='true'][@id=current()/@rule]"
                    if not result[0] or (name == 'upstream-schema' and result[1]):
                        raise AssertionError(f'unexpected result for {name}: {result}')
                    if name == 'missing-abstract-rule' and not any(item[1] == expected_test for item in result[1]):
                        raise AssertionError('missing expected abstract-rule reference failure')
                    if name in baseline and result != baseline[name]:
                        raise AssertionError(f'{producer} transpiler / {executor} validator disagrees on {name}: {result} vs {baseline[name]}')
                    baseline[name] = result
                    print('PASS', producer, 'transpiler /', executor, 'validator:', name,
                          'fired:', sum(result[0].values()), 'failed:', sum(result[1].values()), flush=True)


if __name__ == '__main__':
    main()
