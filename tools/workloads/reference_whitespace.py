#!/usr/bin/env python3
"""Cross-check the SchXslt2 whitespace case with SaxonC and rust-xslt."""
import argparse
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from saxonche import PySaxonProcessor
from run import ROOT, invoke, resources, revision

FIXTURES = ROOT / 'tools/workloads/schematron'
SVRL = '{http://purl.oclc.org/dsdl/svrl}'


def message(report):
    root = ET.fromstring(report)
    text = root.find('.//' + SVRL + 'failed-assert/' + SVRL + 'text')
    if text is None:
        raise ValueError('expected failed assertion message')
    return ''.join(text.itertext())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', required=True, type=Path)
    args = parser.parse_args()
    sch = args.schxslt2.resolve()
    print('SchXslt2 revision:', revision(sch), flush=True)
    subprocess.run(['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance'], cwd=ROOT, check=True)
    driver = ROOT / 'target/debug/examples/reference_driver'
    transpiler = sch / 'src/main/resources/content/transpile.xsl'
    schema_path = FIXTURES / 'typical.sch'
    schema = schema_path.read_text().replace('<name/> named <value-of', '<name/> <value-of')
    invoice = FIXTURES / 'typical-invalid.xml'
    modules = resources(sch)
    with tempfile.TemporaryDirectory(prefix='whitespace-reference-') as directory, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        xslt = processor.new_xslt30_processor()
        saxon_transpiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        for preserve in (False, True):
            source = schema.replace('<assert test="$quantity ge $minimum"', '<assert xml:space="preserve" test="$quantity ge $minimum"') if preserve else schema
            rust_result = invoke(driver, transpiler, source, modules, schema_path.resolve().as_uri())
            if rust_result['diagnostics']:
                raise RuntimeError(rust_result['diagnostics'][0])
            validators = {
                'rust': rust_result['output'],
                'saxon': saxon_transpiler.transform_to_string(xdm_node=processor.parse_xml(xml_text=source)),
            }
            observed = {}
            for producer, generated in validators.items():
                path = Path(directory) / (producer + '.xsl')
                path.write_text(generated)
                rust_report = invoke(driver, path, invoice.read_text(), {}, invoice.resolve().as_uri())
                if rust_report['diagnostics']:
                    raise RuntimeError(rust_report['diagnostics'][0])
                observed[(producer, 'rust')] = message(rust_report['output'])
                saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(path))
                observed[(producer, 'saxon')] = message(saxon_validator.transform_to_string(source_file=str(invoice)))
            expected = 'Quantity for item bad must be at least two.' if preserve else 'Quantity for itembad must be at least two.'
            for (producer, executor), actual in observed.items():
                if actual != expected:
                    raise AssertionError(f'{producer} transpiler / {executor} validator: {actual!r} != {expected!r}')
                print(('PRESERVED' if preserve else 'LIMITATION'), producer, 'transpiler /', executor, 'validator:', repr(actual), flush=True)


if __name__ == '__main__':
    main()
