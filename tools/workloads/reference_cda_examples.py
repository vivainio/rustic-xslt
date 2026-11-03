#!/usr/bin/env python3
"""Compare an HL7 C-CDA schema on the upstream clinical example corpus."""
import argparse
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from saxonche import PySaxonProcessor

from reference_peppol import signature
from run import ROOT, revision


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--examples', type=Path, required=True)
    parser.add_argument('--nemsis', type=Path,
                        help='checkout containing the ISO compiler (required unless --schxslt2 is given)')
    parser.add_argument('--schxslt2', type=Path,
                        help='use SchXslt2 instead of the NEMSIS ISO compiler')
    parser.add_argument('--version', choices=('2.1', '3.0', '4.0'), default='3.0')
    parser.add_argument('--phase', choices=('errors', 'warnings'), default='errors')
    parser.add_argument('--vocabulary', type=Path,
                        help='voc.xml required for the 2.1 schema')
    parser.add_argument('--allow-compiler-messages', action='store_true',
                        help='continue after compiler messages; inspect stderr for incomplete rules')
    parser.add_argument('--iso-fix-attribute-contexts', action='store_true',
                        help='apply the recognized ISO skeleton attribute compatibility fix')
    parser.add_argument('--schxslt-xslt1-message-values', action='store_true',
                        help='use XPath 1.0 compatibility for SchXslt2 message value-of expressions')
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=200_000_000)
    parser.add_argument('--max-depth', type=int, default=2048)
    parser.add_argument('--expected-count', type=int, default=137,
                        help='expected number of examples (default: pinned corpus size, 137)')
    args = parser.parse_args()
    if args.version == '2.1' and args.vocabulary is None:
        parser.error('--version 2.1 requires --vocabulary pointing to voc.xml')
    if not args.schxslt2 and not args.nemsis:
        parser.error('one of --nemsis and --schxslt2 is required')
    if args.iso_fix_attribute_contexts and args.schxslt2:
        parser.error('--iso-fix-attribute-contexts applies only to the ISO compiler')
    if args.schxslt_xslt1_message_values and not args.schxslt2:
        parser.error('--schxslt-xslt1-message-values requires --schxslt2')
    examples = args.examples.resolve()
    print('HL7 examples revision:', revision(examples), flush=True)
    schema = examples / f'ccda-{args.version}.sch'
    if args.schxslt2:
        schxslt2 = args.schxslt2.resolve()
        print('SchXslt2 revision:', revision(schxslt2), flush=True)
        compiler = schxslt2 / 'src/main/resources/content/transpile.xsl'
    else:
        nemsis = args.nemsis.resolve()
        print('ISO compiler (NEMSIS) revision:', revision(nemsis), flush=True)
        compiler = nemsis / 'Schematron/DevelopmentKit/Schematron/utilities/iso-schematron-xslt2/iso_svrl_for_xslt2.xsl'
    inputs = sorted((examples / 'input/examples').glob('*.xml'))
    if len(inputs) != args.expected_count or not inputs:
        raise ValueError(f'expected {args.expected_count} clinical examples, found {len(inputs)}')
    build = ['cargo', 'build', '--example', 'schematron']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/schematron'
    with tempfile.TemporaryDirectory(prefix='cda-examples-') as directory, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        if args.version == '2.1':
            local_schema = Path(directory) / schema.name
            shutil.copyfile(schema, local_schema)
            shutil.copyfile(args.vocabulary, Path(directory) / 'voc.xml')
            schema = local_schema
        reference_compiler = compiler
        if args.iso_fix_attribute_contexts:
            local_compiler = Path(directory) / 'iso-compiler'
            shutil.copytree(compiler.parent, local_compiler)
            skeleton = local_compiler / 'iso_schematron_skeleton_for_saxon.xsl'
            source = skeleton.read_text()
            changes = [
                ("$attributes='true' and parent::node()", "$attributes='true'"),
                ('<axsl:template match="*" mode="schematron-select-full-path">',
                 '<axsl:template match="*|@*" mode="schematron-select-full-path">'),
                ('<!-- XSLT1 syntax -->\n\t\t<axsl:text>/</axsl:text>',
                 '<!-- XSLT1 syntax -->\n<axsl:apply-templates select="parent::*" mode="schematron-get-full-path"/>\n\t\t<axsl:text>/</axsl:text>'),
            ]
            for before, after in changes:
                if source.count(before) != 1:
                    raise ValueError('ISO attribute fix does not match this compiler version')
                source = source.replace(before, after, 1)
            skeleton.write_text(source)
            reference_compiler = local_compiler / compiler.name
        if args.schxslt_xslt1_message_values:
            local_compiler = Path(directory) / 'schxslt2-compiler'
            shutil.copytree(compiler.parent, local_compiler)
            reference_compiler = local_compiler / compiler.name
            source = reference_compiler.read_text()
            before = '<alias:value-of select="{@select}"/>'
            if source.count(before) != 1:
                raise ValueError('SchXslt2 message compatibility fix does not match this compiler version')
            reference_compiler.write_text(source.replace(before, '<alias:value-of version="1.0" select="{@select}"/>', 1))
        xslt = processor.new_xslt30_processor()
        compiler_xslt = xslt.compile_stylesheet(stylesheet_file=str(reference_compiler))
        phase_parameter = '{http://dmaus.name/ns/2023/schxslt}phase' if args.schxslt2 else 'phase'
        compiler_xslt.set_parameter(phase_parameter, processor.make_string_value(args.phase))
        if not args.schxslt2:
            compiler_xslt.set_parameter('allow-foreign', processor.make_string_value('true'))
        validator_file = Path(directory) / 'validator.xsl'
        validator_file.write_text(compiler_xslt.transform_to_string(source_file=str(schema)))
        try:
            validator = xslt.compile_stylesheet(stylesheet_file=str(validator_file))
        except Exception as error:
            # The pinned 3.0 warnings phase contains an XPath 1.0 boolean
            # arithmetic idiom that SchXslt2 places in an XSLT 3 expression.
            if (args.schxslt2 and not args.schxslt_xslt1_message_values
                    and args.version == '3.0' and args.phase == 'warnings'
                    and 'XPTY0004' in str(error) and 'xs:boolean' in str(error)):
                case = examples / 'input/examples/header-direct-address.xml'
                rust = subprocess.run([str(driver), str(compiler), str(schema), '--phase', args.phase,
                                       '--max-steps', str(args.max_steps), '--max-depth', str(args.max_depth), str(case)],
                                      cwd=ROOT, text=True, capture_output=True, timeout=300)
                if (rust.returncode == 0 or 'XPTY0004' not in rust.stderr
                        or 'XPath 1.0-bound schema' not in rust.stderr):
                    raise AssertionError(f'expected a Rust XPath 1.0 compatibility diagnostic: {rust.stderr}')
                print('LIMITATION: SchXslt2 emits an XPath 1.0 boolean arithmetic expression in XSLT 3; '
                      'Saxon rejects the validator and Rust reports XPTY0004 when it runs.', flush=True)
                return 1
            raise
        command = [str(driver), str(compiler), str(schema), '--phase', args.phase,
                   '--max-steps', str(args.max_steps), '--max-depth', str(args.max_depth)]
        if not args.schxslt2:
            command.append('--iso')
        if args.iso_fix_attribute_contexts:
            command.append('--iso-fix-attribute-contexts')
        if args.schxslt_xslt1_message_values:
            command.append('--schxslt-xslt1-message-values')
        if args.allow_compiler_messages:
            command.append('--allow-compiler-messages')
        rust = subprocess.run([*command, *(str(path) for path in inputs)], cwd=ROOT, text=True,
                              capture_output=True, timeout=600)
        if rust.returncode:
            raise RuntimeError(f'rust-xslt failed: {rust.stderr.strip()}')
        warning = 'this ISO compiler can skip attribute-context rules'
        if args.version == '3.0' and args.phase == 'errors':
            if (warning in rust.stderr) != (not args.schxslt2 and not args.iso_fix_attribute_contexts):
                raise AssertionError('the ISO attribute-context warning was missing or applied to SchXslt2')
            if args.schxslt2 and 'XPath 1.0-bound schema' not in rust.stderr:
                raise AssertionError('SchXslt2 XPath 1.0 compatibility warning was missing')
        if args.iso_fix_attribute_contexts and 'using ISO compiler attribute-context compatibility fix' not in rust.stderr:
            raise AssertionError('the ISO compatibility fix was not reported')
        if args.schxslt_xslt1_message_values and 'for message value-of expressions only' not in rust.stderr:
            raise AssertionError('the SchXslt2 message compatibility fix was not reported')
        for line in rust.stderr.splitlines():
            if line.startswith('Schematron compiler: '):
                print('COMPILER MESSAGE:', line, flush=True)
        reports = re.split(r'(?=<\?xml\s)', rust.stdout.strip())
        reports = [report for report in reports if report.strip()]
        if len(reports) != len(inputs):
            raise AssertionError(f'expected {len(inputs)} SVRL documents, got {len(reports)}')
        fired_total = failed_total = empty = 0
        observed = {}
        for path, report in zip(inputs, reports):
            actual = signature(report)
            reference = signature(validator.transform_to_string(source_file=str(path)))
            if actual != reference:
                raise AssertionError(f'{path.name}: report differs from Saxon')
            observed[path.name] = actual
            fired, failures, _ = actual
            if path.name == 'header-direct-address.xml' and args.version == '3.0' and args.phase == 'errors':
                telecom_failures = sum(count for (_, test, _, _), count in failures.items()
                                       if test == 'contains($TelecomUseUSRealmHeader, .)')
                if telecom_failures != (2 if args.schxslt2 or args.iso_fix_attribute_contexts else 0):
                    raise AssertionError(f'{path.name}: unexpected telecom-use findings: {telecom_failures}')
                if args.iso_fix_attribute_contexts:
                    locations = [location for (_, test, location, _), count in failures.items()
                                 if test == 'contains($TelecomUseUSRealmHeader, .)' for _ in range(count)]
                    if any(not location or 'ClinicalDocument' not in location or not location.endswith('/@use')
                           for location in locations) or len(set(locations)) != 2:
                        raise AssertionError(f'{path.name}: invalid attribute locations: {locations}')
                    xpath = processor.new_xpath_processor()
                    xpath.set_context(xdm_item=processor.parse_xml(xml_file_name=str(path)))
                    selected = [xpath.evaluate(location) for location in locations]
                    if any(result is None or result.size != 1 for result in selected):
                        raise AssertionError(f'{path.name}: attribute locations do not select one node each')
                    if {result.item_at(0).string_value for result in selected} != {'DIR', 'PUB'}:
                        raise AssertionError(f'{path.name}: attribute locations select the wrong values')
            fired_total += sum(fired.values())
            failed_total += sum(failures.values())
            empty += not bool(fired)
            print('PASS', path.name, 'fired:', sum(fired.values()), 'failed:', sum(failures.values()), flush=True)
        if not fired_total:
            raise AssertionError('no rules fired across the corpus')
        print('PASS total:', len(inputs), 'documents,', fired_total, 'rules fired,',
              failed_total, 'failed assertions,', empty, 'documents without fired rules', flush=True)
        if args.version == '3.0' and args.phase == 'errors':
            names = ['general-narrative-reference-act.xml', 'medications-med-relative-dose-iv-drug.xml',
                     'medical-equipment-multiple-implants.xml']
            cases = [examples / 'input/examples' / name for name in names]
            if observed[names[0]][0] or not observed[names[1]][0] or not observed[names[2]][1]:
                raise AssertionError('expected unmatched, clean and finding cases in the pinned corpus')
            flagged = subprocess.run([*command, '--fail-on-no-rules', *map(str, cases)], cwd=ROOT,
                                     text=True, capture_output=True, timeout=300)
            flagged_reports = [part for part in re.split(r'(?=<\?xml\s)', flagged.stdout) if part.strip()]
            if (flagged.returncode != 1 or len(flagged_reports) != len(cases)
                    or 'no Schematron rules fired' not in flagged.stderr
                    or any(signature(report) != observed[name] for name, report in zip(names, flagged_reports))):
                raise AssertionError('fail-on-no-rules did not preserve all reports and return status 1')
            clean = subprocess.run([*command, '--fail-on-no-rules', *map(str, cases[1:])], cwd=ROOT,
                                   text=True, capture_output=True, timeout=300)
            if clean.returncode != 0:
                raise AssertionError(f'fail-on-no-rules treated a finding as an unmatched input: {clean.stderr}')
            combined = subprocess.run([*command, '--fail-on-no-rules', '--fail-on-findings', *map(str, cases[1:])],
                                      cwd=ROOT, text=True, capture_output=True, timeout=300)
            if combined.returncode != 1:
                raise AssertionError('combined findings and no-rules flags did not return status 1')
            print('PASS no-rules exit status, complete reports, and independent findings policy', flush=True)


if __name__ == '__main__':
    raise SystemExit(main())
