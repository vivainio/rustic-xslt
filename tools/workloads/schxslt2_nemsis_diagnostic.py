#!/usr/bin/env python3
"""Investigate SchXslt2/NEMSIS incompatibilities using temporary modifications."""
import argparse
from collections import Counter
import subprocess
import tempfile
from pathlib import Path
from saxonche import PySaxonProcessor
from reference_nemsis import comparable_signature
from reference_peppol import signature
from run import ROOT, invoke, revision


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise ValueError(f'expected exactly one occurrence of {old!r}')
    return text.replace(old, new, 1)


def ids(result):
    return sum((Counter({entry[0]: count}) for entry, count in result[1].items()), Counter())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--nemsis', type=Path, required=True)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--release', action='store_true')
    args = parser.parse_args()
    nemsis, schxslt = args.nemsis.resolve(), args.schxslt2.resolve()
    print('NEMSIS revision:', revision(nemsis), flush=True)
    print('SchXslt2 revision:', revision(schxslt), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    source = schxslt / 'src/main/resources/content/transpile.xsl'
    schema = nemsis / 'Schematron/rules/DEMDataSet.sch'
    suite = nemsis / 'SampleData/Schematron/DEM'

    # Experimental compatibility changes are applied only to temporary files.
    original = "{($schxslt:location-function, 'path')[1]}({($assertion/@subject, $assertion/../@subject, '$Q{http://dmaus.name/ns/2023/schxslt}rule-context')[1]})"
    corrected = "{($schxslt:location-function, 'path')[1]}(({($assertion/@subject, $assertion/../@subject, '$Q{http://dmaus.name/ns/2023/schxslt}rule-context')[1]})[1])"
    compiler = replace_once(source.read_text(), original, corrected)
    original = 'Q{http://www.w3.org/1999/XSL/Transform}accumulator | Q{http://www.w3.org/1999/XSL/Transform}function |'
    compiler = replace_once(compiler, original, 'Q{http://www.w3.org/1999/XSL/Transform}variable | ' + original)
    marker = '<sch:ns prefix="nem"'
    schema_text = replace_once(schema.read_text(), marker,
                               '<sch:ns prefix="xs" uri="http://www.w3.org/2001/XMLSchema"/>' + marker)
    with tempfile.TemporaryDirectory(prefix='schxslt2-nemsis-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        directory = Path(tmp)
        stylesheet = directory / 'compiler.xsl'
        stylesheet.write_text(compiler)
        adjusted_schema = directory / 'schema.sch'
        adjusted_schema.write_text(schema_text)
        xslt = processor.new_xslt30_processor()
        saxon_compiler = xslt.compile_stylesheet(stylesheet_file=str(stylesheet))
        result = invoke(driver, stylesheet, schema_text, {}, schema.as_uri(), max_steps=100_000_000)
        if result['diagnostics']:
            raise RuntimeError(result['diagnostics'][0])
        validators = {'rust': result['output'], 'saxon': saxon_compiler.transform_to_string(source_file=str(adjusted_schema))}
        baseline = {}
        for producer, generated in validators.items():
            validator = directory / (producer + '.xsl')
            validator.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
            for name in ('DEMDataSet--Base.xml', 'DEMDataSet-nemSch_d006_A.xml', 'DEMDataSet-nemSch_d007_A.xml'):
                doc = suite / 'xml' / name
                result = invoke(driver, validator, doc.read_text(), {}, doc.as_uri(), max_steps=100_000_000)
                if result['diagnostics']:
                    raise RuntimeError(result['diagnostics'][0])
                outputs = {'rust': result['output'], 'saxon': saxon_validator.transform_to_string(source_file=str(doc))}
                expected = signature((suite / 'svrl' / name).read_text())
                for executor, output in outputs.items():
                    actual = comparable_signature(output)
                    if not actual[0] or ids(actual) != ids(expected) or (name in baseline and actual != baseline[name]):
                        raise AssertionError(f'{producer}/{executor} {name}: {actual}, expected IDs {ids(expected)}')
                    expected_subject = {
                        'DEMDataSet-nemSch_d006_A.xml': '/Q{http://www.nemsis.org}dAgency.06[2]',
                        'DEMDataSet-nemSch_d007_A.xml': '/Q{http://www.nemsis.org}dAgency.07[1]',
                    }.get(name)
                    if expected_subject and not any(entry[2].endswith(expected_subject) for entry in actual[1]):
                        raise AssertionError(f'{name}: the first subject was not used for the location')
                    baseline[name] = actual
                    print('PASS adjusted', producer, 'compiler /', executor, 'validator:', name,
                          'fired:', sum(actual[0].values()), 'failure IDs:', dict(ids(actual)), flush=True)


if __name__ == '__main__':
    main()
