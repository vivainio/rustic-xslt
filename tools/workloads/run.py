#!/usr/bin/env python3
"""Exercise unmodified, locally installed real-world XSLT stylesheets."""
import argparse
import json
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
XML_BASE = '{http://www.w3.org/XML/1998/namespace}base'
DB = '{http://docbook.org/ns/docbook}'


def revision(path):
    return subprocess.check_output(['git', '-C', str(path), 'rev-parse', 'HEAD'], text=True).strip()


def resources(path):
    return {p.resolve().as_uri(): p.read_text() for p in path.rglob('*.xsl')}


def invoke(driver, stylesheet, source, modules, source_uri=None, params=None):
    request = {'stylesheet': stylesheet.read_text(), 'stylesheetURI': stylesheet.resolve().as_uri(),
               'source': source, 'resources': modules}
    if source_uri:
        request['sourceURI'] = source_uri
    if params:
        request['params'] = params
    started = time.monotonic()
    run = subprocess.run([str(driver)], input=json.dumps(request) + '\n', text=True,
                         capture_output=True, check=True, timeout=120)
    result = json.loads(run.stdout)
    result['seconds'] = round(time.monotonic() - started, 3)
    return result


def same_tree(left, right):
    def signature(node):
        return (node.tag, sorted(node.attrib.items()), node.text, node.tail,
                [signature(child) for child in node])
    return signature(left) == signature(right)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--docbook-release', required=True, type=Path, help='published release xslt/ directory')
    parser.add_argument('--docbook-source', required=True, type=Path, help='DocBook xslTNG source checkout')
    parser.add_argument('--schxslt2', required=True, type=Path, help='SchXslt2 source checkout')
    parser.add_argument('--report', type=Path, help='write machine-readable JSON report')
    args = parser.parse_args()
    subprocess.run(['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance'], cwd=ROOT, check=True)
    driver = ROOT / 'target/debug/examples/reference_driver'
    doc = args.docbook_release.resolve()
    modules = resources(doc)
    source_path = args.docbook_source / 'src/main/samples/article.xml'
    source = source_path.read_text()
    cases = []

    def record(name, result, check=None):
        if result.get('diagnostics'):
            status = 'BLOCKED'
            detail = result['diagnostics'][0]
        else:
            try:
                status, detail = check(result) if check else ('UNVERIFIED', 'transformation completed; no output oracle for this stage')
            except (AssertionError, ET.ParseError, KeyError) as error:
                status, detail = 'FAIL', str(error) or 'output did not match expected behavior'
        case = {'case': name, 'status': status, 'detail': detail, 'seconds': result['seconds']}
        cases.append(case)
        print(f"{status:8} {name} ({result['seconds']}s): {detail}", flush=True)

    def check_identity(result, add_base=False):
        original, actual = ET.fromstring(source), ET.fromstring(result['output'])
        if add_base and XML_BASE not in original.attrib:
            assert XML_BASE in actual.attrib, 'missing added xml:base'
            del actual.attrib[XML_BASE]
        assert same_tree(original, actual), 'output differs from source tree'
        if result['messages']:
            return 'DEGRADED', 'identity fallback: ' + '; '.join(map(str, result['messages']))
        return 'PASS', 'source tree preserved'

    record('docbook/full', invoke(driver, doc / 'docbook.xsl', source, modules, source_path.resolve().as_uri()))
    for stage in sorted((doc / 'transforms').glob('*.xsl')):
        check = (lambda r: check_identity(r, True)) if stage.name.startswith('00-') else (
            (lambda r: check_identity(r)) if stage.name.startswith('10-') else None)
        record('docbook/' + stage.stem, invoke(driver, stage, source, modules, source_path.resolve().as_uri()), check)
    entity_source = f'<article xmlns="{DB[1:-1]}"><mediaobject><imageobject><imagedata entityref="missing"/></imageobject></mediaobject></article>'
    def check_entity(result):
        root = ET.fromstring(result['output'])
        image = root.find('.//' + DB + 'imagedata')
        assert image is not None and image.attrib == {'fileref': ''}, 'entity reference was not converted to an empty fileref'
        return 'PASS', 'missing entity reference converted to empty fileref'
    record('docbook/00-logstruct-entityref', invoke(driver, doc / 'transforms/00-logstruct.xsl', entity_source, modules), check_entity)
    sch = args.schxslt2.resolve()
    fixture = (sch / 'src/test/bugs/rule-context-root.xml').read_text()
    schema = re.search(r'<sch:schema\b.*?</sch:schema>', fixture, re.DOTALL).group()
    record('schxslt2/transpile', invoke(driver, sch / 'src/main/resources/content/transpile.xsl', schema, resources(sch)))
    report = {'docbook_source_revision': revision(args.docbook_source), 'schxslt2_revision': revision(sch),
              'docbook_release': str(doc), 'cases': cases}
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + '\n')
    counts = {status: sum(c['status'] == status for c in cases) for status in ('PASS', 'DEGRADED', 'UNVERIFIED', 'BLOCKED', 'FAIL')}
    print('Results:', ', '.join(f'{k}={v}' for k, v in counts.items()))
    return 1 if any(c['status'] in ('UNVERIFIED', 'BLOCKED', 'FAIL') for c in cases) else 0


if __name__ == '__main__':
    sys.exit(main())
