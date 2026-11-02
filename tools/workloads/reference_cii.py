#!/usr/bin/env python3
"""Cross-check CEN CII rules on upstream invoices using Rust and SaxonC."""
import argparse
import re
import subprocess
import tempfile
from pathlib import Path
from saxonche import PySaxonProcessor
from reference_cen import PHASE, replace_once
from reference_peppol import signature
from run import ROOT, invoke, resources, revision


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--peppol', type=Path, required=True)
    parser.add_argument('--cen-examples', type=Path, required=True)
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=20_000_000)
    args = parser.parse_args()
    schxslt, peppol, examples = args.schxslt2.resolve(), args.peppol.resolve(), args.cen_examples.resolve()
    for label, repo in [('SchXslt2', schxslt), ('Peppol', peppol), ('CEN examples', examples)]:
        print(label + ' revision:', revision(repo), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--example', 'schematron', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    target = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples'
    driver = target / 'reference_driver'
    transpiler = schxslt / 'src/main/resources/content/transpile.xsl'
    schema = peppol / 'rules/sch/CEN-EN16931-CII.sch'
    files = sorted((examples / 'cii/examples').glob('*.xml'))
    if len(files) != 15:
        raise ValueError(f'expected 15 pinned CII examples, found {len(files)}')
    cases = [(file.name, file.read_text(), None, file.as_uri()) for file in files]
    sample = examples / 'cii/examples/CII_business_example_01.xml'
    source = sample.read_text()
    missing, count = re.subn(r'<ram:GuidelineSpecifiedDocumentContextParameter>.*?</ram:GuidelineSpecifiedDocumentContextParameter>', '', source, flags=re.DOTALL)
    if count != 1:
        raise ValueError('expected one specification identifier')
    wrong_total = replace_once(source, '<ram:LineTotalAmount>1436.5</ram:LineTotalAmount>',
                               '<ram:LineTotalAmount>1436.6</ram:LineTotalAmount>')
    wrong_code = replace_once(source, '<ram:TypeCode>380</ram:TypeCode>', '<ram:TypeCode>999</ram:TypeCode>')
    invalid = [('missing-specification', missing, 'BR-01', sample.as_uri()),
               ('incorrect-total', wrong_total, 'BR-CO-10', sample.as_uri()),
               ('invalid-code', wrong_code, 'BR-CL-01', sample.as_uri())]
    modules = resources(schxslt)
    with tempfile.TemporaryDirectory(prefix='cii-reference-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        xslt = processor.new_xslt30_processor()
        saxon_compiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        baselines = {}
        for phase in ('#DEFAULT', 'EN16931-model-phase', 'codelist_phase'):
            selected = cases + invalid if phase == '#DEFAULT' else [case for case in cases if case[0] == sample.name] + invalid
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
                for label, xml, expected, uri in selected:
                    rust = invoke(driver, validator, xml, {}, uri, max_steps=args.max_steps, timeout=120)
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

        paths = []
        for label, xml, _, _ in invalid:
            path = Path(tmp) / (label + '.xml')
            path.write_text(xml)
            paths.append(path)
        command = [str(target / 'schematron'), str(transpiler), str(schema), '--phase', 'codelist_phase',
                   '--max-steps', str(args.max_steps), *map(str, paths)]
        process = subprocess.run(command, text=True, capture_output=True, check=True, timeout=120)
        reports = [part for part in re.split(r'(?=<\?xml\s)', process.stdout) if part.strip()]
        if len(reports) != len(invalid):
            raise AssertionError('public example returned the wrong number of reports')
        for (label, _, _, _), output in zip(invalid, reports):
            if signature(output) != baselines['codelist_phase'][label]:
                raise AssertionError(f'public example disagrees with reference for {label}')
            print('PASS public example: codelist_phase', label, flush=True)


if __name__ == '__main__':
    main()
