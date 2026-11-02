#!/usr/bin/env python3
"""Cross-check multi-line Peppol invoices with Rust and SaxonC."""
import argparse
import re
import subprocess
import tempfile
from pathlib import Path
from saxonche import PySaxonProcessor
from reference_peppol import signature
from run import ROOT, invoke, resources, revision

LINE_CONTEXT = 'cac:InvoiceLine | cac:CreditNoteLine'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--peppol', type=Path, required=True)
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=20_000_000)
    args = parser.parse_args()
    schxslt, peppol = args.schxslt2.resolve(), args.peppol.resolve()
    print('SchXslt2 revision:', revision(schxslt), flush=True)
    print('Peppol revision:', revision(peppol), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    transpiler = schxslt / 'src/main/resources/content/transpile.xsl'
    schema = peppol / 'rules/sch/PEPPOL-EN16931-UBL.sch'
    invoice = peppol / 'rules/examples/base-example.xml'
    source = invoice.read_text()
    blocks = list(re.finditer(r'<cac:InvoiceLine>.*?</cac:InvoiceLine>', source, re.DOTALL))
    if len(blocks) != 2 or blocks[0].group().count('<cbc:ID>1</cbc:ID>') != 1:
        raise ValueError('upstream sample no longer has the expected two lines')
    template = blocks[0].group()

    def invoice_with_lines(count):
        lines = '\n'.join(template.replace('<cbc:ID>1</cbc:ID>', f'<cbc:ID>{index}</cbc:ID>', 1)
                          for index in range(1, count + 1))
        result = source[:blocks[0].start()] + lines + source[blocks[-1].end():]
        result, replacements = re.subn(r'<cbc:ProfileID\b[^>]*>.*?</cbc:ProfileID>', '', result, flags=re.DOTALL)
        if replacements != 1:
            raise ValueError('upstream sample must contain exactly one ProfileID')
        return result

    with tempfile.TemporaryDirectory(prefix='peppol-scale-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        xslt = processor.new_xslt30_processor()
        saxon_compiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        rust = invoke(driver, transpiler, schema.read_text(), resources(schxslt), schema.as_uri(),
                      max_steps=args.max_steps, timeout=120)
        if rust['diagnostics']:
            raise RuntimeError(rust['diagnostics'][0])
        validators = {'rust': rust['output'], 'saxon': saxon_compiler.transform_to_string(source_file=str(schema))}
        baseline = {}
        for producer, generated in validators.items():
            path = Path(tmp) / (producer + '.xsl')
            path.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(path))
            for count in (10, 20, 40):
                xml = invoice_with_lines(count)
                rust = invoke(driver, path, xml, {}, invoice.as_uri(), max_steps=args.max_steps, timeout=120)
                if rust['diagnostics']:
                    raise RuntimeError(f'{producer}/{count}: {rust["diagnostics"][0]}')
                reports = {'rust': rust['output'], 'saxon': saxon_validator.transform_to_string(xdm_node=processor.parse_xml(xml_text=xml))}
                for executor, output in reports.items():
                    result = signature(output)
                    failed = {entry[0] for entry in result[1]}
                    if result[0][LINE_CONTEXT] != count or 'PEPPOL-EN16931-R001' not in failed:
                        raise AssertionError(f'{producer}/{executor}/{count}: wrong fired rules or missing expected finding')
                    if count in baseline and result != baseline[count]:
                        raise AssertionError(f'{producer}/{executor}/{count}: report differs from reference')
                    baseline[count] = result
                    print('PASS', producer, 'compiler /', executor, 'validator:', count, 'lines,',
                          sum(result[0].values()), 'rules fired, failed IDs:', sorted(failed), flush=True)


if __name__ == '__main__':
    main()
