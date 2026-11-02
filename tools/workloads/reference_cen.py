#!/usr/bin/env python3
"""Cross-check CEN EN 16931 invoice rules with SaxonC and rust-xslt."""
import argparse
import re
import subprocess
import tempfile
from pathlib import Path
from saxonche import PySaxonProcessor
from reference_peppol import signature
from run import ROOT, invoke, resources, revision

PHASE = 'Q{http://dmaus.name/ns/2023/schxslt}phase'


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise ValueError(f'expected exactly one occurrence of {old!r}')
    return source.replace(old, new, 1)


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
    build = ['cargo', 'build', '--example', 'reference_driver', '--example', 'schematron', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    transpiler = schxslt / 'src/main/resources/content/transpile.xsl'
    schema = peppol / 'rules/sch/CEN-EN16931-UBL.sch'
    invoice = peppol / 'rules/examples/base-example.xml'
    source = invoice.read_text()
    missing, count = re.subn(r'<cbc:CustomizationID\b[^>]*>.*?</cbc:CustomizationID>', '', source, flags=re.DOTALL)
    if count != 1:
        raise ValueError('expected exactly one CustomizationID')
    wrong_total = replace_once(source, '<cbc:LineExtensionAmount currencyID="EUR">1300</cbc:LineExtensionAmount>',
                               '<cbc:LineExtensionAmount currencyID="EUR">1301</cbc:LineExtensionAmount>')
    wrong_code = replace_once(source, '<cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>',
                              '<cbc:InvoiceTypeCode>999</cbc:InvoiceTypeCode>')
    cases = [('valid', source, None), ('missing-customization', missing, 'BR-01'),
             ('incorrect-total', wrong_total, 'BR-CO-10'), ('invalid-code', wrong_code, 'BR-CL-01')]
    modules = resources(schxslt)

    with tempfile.TemporaryDirectory(prefix='cen-reference-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        xslt = processor.new_xslt30_processor()
        saxon_compiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        baselines = {}
        for phase in ('#DEFAULT', 'EN16931model_phase', 'codelist_phase'):
            params = {PHASE: phase} if phase != '#DEFAULT' else None
            result = invoke(driver, transpiler, schema.read_text(), modules, schema.as_uri(), params,
                            max_steps=args.max_steps, timeout=120)
            if result['diagnostics']:
                raise RuntimeError(result['diagnostics'][0])
            saxon_compiler.set_parameter('{http://dmaus.name/ns/2023/schxslt}phase', processor.make_string_value(phase))
            validators = {'rust': result['output'], 'saxon': saxon_compiler.transform_to_string(source_file=str(schema))}
            baseline = {}
            for producer, generated in validators.items():
                validator = Path(tmp) / (producer + '.xsl')
                validator.write_text(generated)
                saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
                for label, xml, expected in cases:
                    rust = invoke(driver, validator, xml, {}, invoice.as_uri(), max_steps=args.max_steps, timeout=120)
                    if rust['diagnostics']:
                        raise RuntimeError(rust['diagnostics'][0])
                    outputs = {'rust': rust['output'], 'saxon': saxon_validator.transform_to_string(xdm_node=processor.parse_xml(xml_text=xml))}
                    for executor, output in outputs.items():
                        outcome = signature(output)
                        failed = {entry[0] for entry in outcome[1]}
                        active = expected is not None and (phase == '#DEFAULT' or
                                 (phase == 'codelist_phase') == expected.startswith('BR-CL-'))
                        if not outcome[0] or (expected is None and failed) or (active and expected not in failed) or (not active and expected in failed):
                            raise AssertionError(f'{phase}/{producer}/{executor}/{label}: unexpected failures {failed}')
                        if label in baseline and outcome != baseline[label]:
                            raise AssertionError(f'{phase}/{producer}/{executor}/{label}: result differs from reference')
                        baseline[label] = outcome
                        print('PASS', phase, producer, 'compiler /', executor, 'validator:', label,
                              'fired:', sum(outcome[0].values()), 'failed:', sorted(failed), flush=True)
            baselines[phase] = baseline

        selected = [cases[0], cases[1], cases[3]]
        paths = []
        for label, xml, _ in selected:
            path = Path(tmp) / (label + '.xml')
            path.write_text(xml)
            paths.append(path)
        cli = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/schematron'
        command = [str(cli), str(transpiler), str(schema), '--phase', 'codelist_phase',
                   '--max-steps', str(args.max_steps), *map(str, paths)]
        process = subprocess.run(command, text=True, capture_output=True, check=True, timeout=120)
        reports = [chunk for chunk in re.split(r'(?=<\?xml\s)', process.stdout) if chunk.strip()]
        if len(reports) != len(selected):
            raise AssertionError(f'public example returned {len(reports)} reports for {len(selected)} inputs')
        for (label, _, _), output in zip(selected, reports):
            if signature(output) != baselines['codelist_phase'][label]:
                raise AssertionError(f'public example disagrees with reference for {label}')
            print('PASS public example: codelist_phase', label, flush=True)


if __name__ == '__main__':
    main()
