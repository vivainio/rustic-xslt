#!/usr/bin/env python3
"""Cross-check real Peppol Schematron with SaxonC and rust-xslt."""
import argparse
from collections import Counter
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from saxonche import PySaxonProcessor
from run import ROOT, invoke, resources, revision

SVRL = '{http://purl.oclc.org/dsdl/svrl}'


def signature(report):
    root = ET.fromstring(report)
    if root.tag != SVRL + 'schematron-output':
        raise ValueError('validator did not return SVRL')
    fired = Counter(node.get('context') for node in root.findall('.//' + SVRL + 'fired-rule'))
    def assertions(kind):
        return Counter((node.get('id'), node.get('test'), node.get('location'),
                        ' '.join(''.join(node.itertext()).split()))
                       for node in root.findall('.//' + SVRL + kind))
    return fired, assertions('failed-assert'), assertions('successful-report')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', required=True, type=Path)
    parser.add_argument('--peppol', required=True, type=Path)
    parser.add_argument('--release', action='store_true', help='use an optimized driver')
    parser.add_argument('--max-steps', type=int, default=500_000_000)
    parser.add_argument('--timeout', type=int, default=300)
    args = parser.parse_args()
    sch, peppol = args.schxslt2.resolve(), args.peppol.resolve()
    print('SchXslt2 revision:', revision(sch), flush=True)
    print('Peppol revision:', revision(peppol), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    transpiler = sch / 'src/main/resources/content/transpile.xsl'
    schema = peppol / 'rules/sch/PEPPOL-EN16931-UBL.sch'
    invoice = peppol / 'rules/examples/base-example.xml'
    source = invoice.read_text()
    invalid, replaced = re.subn(r'<cbc:ProfileID\b[^>]*>.*?</cbc:ProfileID>', '', source, flags=re.DOTALL)
    if replaced != 1:
        raise ValueError('upstream sample must contain exactly one cbc:ProfileID')
    with tempfile.TemporaryDirectory(prefix='peppol-reference-') as directory, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        xslt = processor.new_xslt30_processor()
        saxon_transpiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        rust_result = invoke(driver, transpiler, schema.read_text(), resources(sch), schema.resolve().as_uri(), max_steps=args.max_steps, timeout=args.timeout)
        if rust_result['diagnostics']:
            raise RuntimeError(rust_result['diagnostics'][0])
        validators = {
            'rust': rust_result['output'],
            'saxon': saxon_transpiler.transform_to_string(source_file=str(schema)),
        }
        baseline = {}
        for producer, generated in validators.items():
            path = Path(directory) / (producer + '.xsl')
            path.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(path))
            for label, xml in [('upstream-example', source), ('missing-profile', invalid)]:
                saxon_report = saxon_validator.transform_to_string(xdm_node=processor.parse_xml(xml_text=xml))
                rust_report = invoke(driver, path, xml, {}, invoice.resolve().as_uri(), max_steps=args.max_steps, timeout=args.timeout)
                if rust_report['diagnostics']:
                    raise RuntimeError(rust_report['diagnostics'][0])
                for executor, report in [('saxon', saxon_report), ('rust', rust_report['output'])]:
                    result = signature(report)
                    if not result[0]:
                        raise AssertionError('no rules fired')
                    if label == 'upstream-example' and result[1]:
                        raise AssertionError('upstream example has failed assertions')
                    if label == 'missing-profile' and not any(key[0] == 'PEPPOL-EN16931-R001' for key in result[1]):
                        raise AssertionError('missing expected profile assertion')
                    if label in baseline and result != baseline[label]:
                        raise AssertionError(f'{producer} transpiler / {executor} validator disagrees on {label}')
                    baseline[label] = result
                    print('PASS', producer, 'transpiler /', executor, 'validator:', label,
                          'fired:', sum(result[0].values()), 'failed:', sum(result[1].values()), flush=True)


if __name__ == '__main__':
    main()
