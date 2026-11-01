#!/usr/bin/env python3
"""Compile a real Peppol invoice Schematron and validate two invoices."""
import argparse
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from run import ROOT, invoke, resources, revision

SVRL = '{http://purl.oclc.org/dsdl/svrl}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--peppol', type=Path, required=True)
    parser.add_argument('--max-steps', type=int, default=500_000_000)
    parser.add_argument('--generated', type=Path, help='save generated validator XSLT')
    parser.add_argument('--timeout', type=int, default=300, help='seconds per transformation')
    parser.add_argument('--release', action='store_true', help='use an optimized driver')
    args = parser.parse_args()
    sch, peppol = args.schxslt2.resolve(), args.peppol.resolve()
    print('SchXslt2 revision:', revision(sch), flush=True)
    print('Peppol revision:', revision(peppol), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    profile = 'release' if args.release else 'debug'
    driver = ROOT / 'target' / profile / 'examples/reference_driver'
    transpiler = sch / 'src/main/resources/content/transpile.xsl'
    schema = peppol / 'rules/sch/PEPPOL-EN16931-UBL.sch'
    invoice = peppol / 'rules/examples/base-example.xml'
    source = invoice.read_text()
    invalid, replaced = re.subn(r'<cbc:ProfileID\b[^>]*>.*?</cbc:ProfileID>', '', source, flags=re.DOTALL)
    if replaced != 1:
        raise ValueError('upstream sample must contain exactly one cbc:ProfileID')
    print('Transpiling', schema.name, 'with max steps', args.max_steps, flush=True)
    result = invoke(driver, transpiler, schema.read_text(), resources(sch), schema.resolve().as_uri(), max_steps=args.max_steps, timeout=args.timeout)
    if result['diagnostics']:
        print('BLOCKED transpilation:', result['diagnostics'][0], flush=True)
        return 1
    print('Transpiled in', result['seconds'], 'seconds;', len(result['output']), 'characters', flush=True)
    if args.generated:
        args.generated.write_text(result['output'])
    with tempfile.TemporaryDirectory(prefix='peppol-') as directory:
        validator = Path(directory) / 'validator.xsl'
        validator.write_text(result['output'])
        for label, xml in [('upstream-example', source), ('missing-profile', invalid)]:
            result = invoke(driver, validator, xml, {}, invoice.resolve().as_uri(), max_steps=args.max_steps, timeout=args.timeout)
            if result['diagnostics']:
                print('BLOCKED', label, result['diagnostics'][0], flush=True)
                return 1
            root = ET.fromstring(result['output'])
            if root.tag != SVRL + 'schematron-output':
                raise ValueError('validator did not return SVRL')
            fired = root.findall('.//' + SVRL + 'fired-rule')
            failures = root.findall('.//' + SVRL + 'failed-assert')
            ids = {failure.get('id') for failure in failures}
            if not fired or (label == 'upstream-example' and failures) or (label == 'missing-profile' and 'PEPPOL-EN16931-R001' not in ids):
                print('FAIL', label, 'fired:', len(fired), 'failed IDs:', sorted(str(value) for value in ids), flush=True)
                return 1
            print('PASS', label, 'fired:', len(fired), 'failed IDs:', sorted(str(value) for value in ids), 'seconds:', result['seconds'], flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
