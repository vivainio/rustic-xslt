#!/usr/bin/env python3
"""Probe the upstream DocBook glossary Schematron and its XSLT library."""
import argparse
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from saxonche import PySaxonProcessor
from reference_peppol import signature
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
    build = ['cargo', 'build', '--example', 'reference_driver', '--example', 'schematron', '--features', 'conformance']
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
    def external(term, collections):
        return f'<article xmlns="{DB}"><?db glossary-collection="{collections}"?><para><firstterm>{term}</firstterm></para></article>'
    cases += [('external-single', external('hamburger menu', 'external.xml'), []),
              ('external-missing', external('missing glossary term', 'external.xml'), ['error']),
              ('external-duplicate', external('hamburger menu', 'external.xml second.xml'), ['warning'])]
    blocked = False
    with tempfile.TemporaryDirectory(prefix='docbook-schematron-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        directory = Path(tmp)
        schema_directory = directory / 'schema'
        schema_directory.mkdir()
        # The upstream example uses a relative include. Put its unmodified
        # helper alongside it, as in a deployed stylesheet bundle.
        bundled_schema = schema_directory / 'glossary.sch'
        bundled_schema.write_text(schema.read_text())
        bundled_helper = schema_directory / 'standalone-functions.xsl'
        bundled_helper.write_text(helper.read_text())
        documents = directory / 'documents'
        documents.mkdir()
        external_glossary = (docbook / 'src/guide/xml/glossary.xml').read_text()
        for name in ('external.xml', 'second.xml'):
            (documents / name).write_text(external_glossary)
        for label, xml, _ in cases:
            (documents / (label + '.xml')).write_text(xml)
        modules = {path.resolve().as_uri(): path.read_text() for path in [bundled_helper, documents / 'external.xml', documents / 'second.xml']}
        xslt = processor.new_xslt30_processor()
        saxon_compiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        rust = invoke(driver, transpiler, schema.read_text(), resources(schxslt), bundled_schema.as_uri(), max_steps=20_000_000)
        if rust['diagnostics']:
            raise RuntimeError(rust['diagnostics'][0])
        validators = {'rust': rust['output'], 'saxon': saxon_compiler.transform_to_string(source_file=str(bundled_schema))}
        baseline = {}
        for producer, generated in validators.items():
            validator = schema_directory / (producer + '.xsl')
            validator.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
            for label, xml, expected_roles in cases:
                document = documents / (label + '.xml')
                saxon_output = saxon_validator.transform_to_string(source_file=str(document))
                saxon_report = ET.fromstring(saxon_output)
                reports = saxon_report.findall('.//' + SVRL + 'successful-report')
                roles = [node.get('role') for node in reports]
                saxon_signature = signature(saxon_output)
                if not saxon_report.findall('.//' + SVRL + 'fired-rule') or roles != expected_roles or (label in baseline and baseline[label] != saxon_signature):
                    raise AssertionError(f'{producer}/{label}: expected roles {expected_roles}, got {roles}')
                baseline[label] = saxon_signature
                print('PASS', producer, 'compiler / Saxon validator:', label, 'roles:', roles, flush=True)
                result = invoke(driver, validator, xml, modules, document.resolve().as_uri(), max_steps=20_000_000)
                if result['diagnostics']:
                    blocked = True
                    print('BLOCKED', producer, 'compiler / Rust validator:', label, result['diagnostics'][0], flush=True)
                else:
                    rust_report = ET.fromstring(result['output'])
                    rust_roles = [node.get('role') for node in rust_report.findall('.//' + SVRL + 'successful-report')]
                    if not rust_report.findall('.//' + SVRL + 'fired-rule') or rust_roles != expected_roles or signature(result['output']) != saxon_signature:
                        raise AssertionError(f'{producer}/{label}: Rust roles {rust_roles}, expected {expected_roles}')
                    print('PASS', producer, 'compiler / Rust validator:', label, 'roles:', rust_roles, flush=True)

        cli = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/schematron'
        external_cases = [case for case in cases if case[0].startswith('external-')]
        command = [str(cli), str(transpiler), str(bundled_schema)] + [str(documents / (case[0] + '.xml')) for case in external_cases]
        process = subprocess.run(command, text=True, capture_output=True, check=True, timeout=120)
        reports = [chunk for chunk in re.split(r'(?=<\?xml\s)', process.stdout) if chunk.strip()]
        if len(reports) != len(external_cases):
            raise AssertionError(f'public example returned {len(reports)} reports for {len(external_cases)} documents')
        for (label, _, _), output in zip(external_cases, reports):
            if signature(output) != baseline[label]:
                raise AssertionError(f'public example disagrees with Saxon for {label}')
            print('PASS public example:', label, flush=True)

        isolated = documents / 'isolated'
        isolated.mkdir()
        isolated_source = isolated / 'source.xml'
        isolated_source.write_text(external('hamburger menu', '../external.xml'))
        process = subprocess.run([str(cli), str(transpiler), str(bundled_schema),
                                  str(documents / 'external-single.xml'), str(isolated_source)],
                                 text=True, capture_output=True, check=True, timeout=120)
        reports = [ET.fromstring(chunk) for chunk in re.split(r'(?=<\?xml\s)', process.stdout) if chunk.strip()]
        if len(reports) != 2 or reports[0].findall('.//' + SVRL + 'successful-report'):
            raise AssertionError('the first document could not read its own external glossary')
        denied = reports[1].findall('.//' + SVRL + 'successful-report')
        if [node.get('role') for node in denied] != ['error'] or 'Failed to load glossary:' not in process.stderr:
            raise AssertionError('the second document accessed a resource outside its own allowed roots')
        print('PASS public example: document resource roots remain isolated', flush=True)
    return int(blocked)


if __name__ == '__main__':
    raise SystemExit(main())
