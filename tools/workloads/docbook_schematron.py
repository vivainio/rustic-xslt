#!/usr/bin/env python3
"""Probe the upstream DocBook glossary Schematron and its XSLT library."""
import argparse
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from saxonche import PySaxonProcessor
from run import ROOT, invoke, resources, revision

SVRL = '{http://purl.oclc.org/dsdl/svrl}'
DB = 'http://docbook.org/ns/docbook'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--docbook', type=Path, required=True)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--release', action='store_true')
    args = parser.parse_args()
    docbook, schxslt = args.docbook.resolve(), args.schxslt2.resolve()
    print('DocBook revision:', revision(docbook), flush=True)
    print('SchXslt2 revision:', revision(schxslt), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    transpiler = schxslt / 'src/main/resources/content/transpile.xsl'
    schema = docbook / 'src/guide/xml/examples/glossary.sch'
    helper = docbook / 'src/main/xslt/standalone-functions.xsl'

    def source(entries):
        glossary = ''.join('<glossentry><glossterm>cat</glossterm><glossdef><para>Feline</para></glossdef></glossentry>' for _ in range(entries))
        return f'<article xmlns="{DB}"><para><firstterm>cat</firstterm></para><glossary>{glossary}</glossary></article>'

    cases = [('missing', source(0), ['error']), ('single', source(1), []),
             ('duplicate', source(2), ['warning'])]
    blocked = False
    with tempfile.TemporaryDirectory(prefix='docbook-schematron-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        directory = Path(tmp)
        # The upstream example uses a relative include. Put its unmodified
        # helper alongside it, as in a deployed stylesheet bundle.
        bundled_schema = directory / 'glossary.sch'
        bundled_schema.write_text(schema.read_text())
        bundled_helper = directory / 'standalone-functions.xsl'
        bundled_helper.write_text(helper.read_text())
        xslt = processor.new_xslt30_processor()
        saxon_compiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        rust = invoke(driver, transpiler, schema.read_text(), resources(schxslt), bundled_schema.as_uri(), max_steps=20_000_000)
        if rust['diagnostics']:
            raise RuntimeError(rust['diagnostics'][0])
        validators = {'rust': rust['output'], 'saxon': saxon_compiler.transform_to_string(source_file=str(bundled_schema))}
        for producer, generated in validators.items():
            validator = directory / (producer + '.xsl')
            validator.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
            for label, xml, expected_roles in cases:
                saxon_report = ET.fromstring(saxon_validator.transform_to_string(xdm_node=processor.parse_xml(xml_text=xml)))
                reports = saxon_report.findall('.//' + SVRL + 'successful-report')
                roles = [node.get('role') for node in reports]
                if not saxon_report.findall('.//' + SVRL + 'fired-rule') or roles != expected_roles:
                    raise AssertionError(f'{producer}/{label}: expected roles {expected_roles}, got {roles}')
                print('PASS', producer, 'compiler / Saxon validator:', label, 'roles:', roles, flush=True)
                result = invoke(driver, validator, xml, {bundled_helper.resolve().as_uri(): bundled_helper.read_text()},
                                max_steps=20_000_000)
                if result['diagnostics']:
                    blocked = True
                    print('BLOCKED', producer, 'compiler / Rust validator:', label, result['diagnostics'][0], flush=True)
                else:
                    rust_report = ET.fromstring(result['output'])
                    rust_roles = [node.get('role') for node in rust_report.findall('.//' + SVRL + 'successful-report')]
                    if not rust_report.findall('.//' + SVRL + 'fired-rule') or rust_roles != expected_roles:
                        raise AssertionError(f'{producer}/{label}: Rust roles {rust_roles}, expected {expected_roles}')
                    print('PASS', producer, 'compiler / Rust validator:', label, 'roles:', rust_roles, flush=True)
    return int(blocked)


if __name__ == '__main__':
    raise SystemExit(main())
