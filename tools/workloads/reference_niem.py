#!/usr/bin/env python3
"""Exercise NIEM's Schematron-in-XSD rules against SaxonC and rust-xslt."""
import argparse
import subprocess
import tempfile
from pathlib import Path
from saxonche import PySaxonApiError, PySaxonProcessor
from reference_peppol import signature
from run import ROOT, invoke, resources, revision

VALID = '''<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
 xmlns:six="http://release.niem.gov/niem/schematron-in-xsd/4.0/"
 xmlns:ct="http://release.niem.gov/niem/conformanceTargets/3.0/"
 six:queryBinding="xslt2"
 ct:conformanceTargets="http://reference.niem.gov/niem/specification/schematron-in-xsd/4.0/#XSDWithSchematron">
 <xs:element name="foo" type="xs:string"/>
</xs:schema>'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--schxslt2', type=Path, required=True)
    parser.add_argument('--niem', type=Path, required=True)
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--max-steps', type=int, default=20_000_000)
    args = parser.parse_args()
    sch, niem = args.schxslt2.resolve(), args.niem.resolve()
    print('SchXslt2 revision:', revision(sch), flush=True)
    print('NIEM revision:', revision(niem), flush=True)
    build = ['cargo', 'build', '--example', 'reference_driver', '--features', 'conformance']
    if args.release:
        build.append('--release')
    subprocess.run(build, cwd=ROOT, check=True)
    driver = ROOT / 'target' / ('release' if args.release else 'debug') / 'examples/reference_driver'
    transpiler = sch / 'src/main/resources/content/transpile.xsl'
    schema = niem / 'schematron-in-xsd-rules-for-xsd.sch'
    xsd = niem / 'schematron-in-xsd.xsd'
    modules = resources(sch)

    def run(style, source, uri=None, available=None):
        result = invoke(driver, style, source, available or {}, uri, max_steps=args.max_steps)
        if result['diagnostics']:
            raise RuntimeError(result['diagnostics'][0])
        return result['output']

    with tempfile.TemporaryDirectory(prefix='niem-reference-') as tmp, PySaxonProcessor(license=False) as processor:
        print('Reference:', processor.version, flush=True)
        directory = Path(tmp)
        xslt = processor.new_xslt30_processor()
        saxon_transpiler = xslt.compile_stylesheet(stylesheet_file=str(transpiler))
        original = schema.read_text()
        unmodified = directory / 'upstream.xsl'
        unmodified.write_text(run(transpiler, original, schema.as_uri(), modules))
        result = signature(run(unmodified, xsd.read_text(), xsd.as_uri()))
        if not result[0] or len(result[1]) != 2:
            raise AssertionError('unexpected result for the unmodified schema')
        try:
            xslt.compile_stylesheet(stylesheet_file=str(unmodified))
        except PySaxonApiError as error:
            if 'XPST0081' not in str(error) or "prefix 'xs'" not in str(error):
                raise
            print('LIMITATION unmodified schema: generated stylesheet omits xs namespace; Saxon rejects it', flush=True)
        else:
            raise AssertionError('upstream generated stylesheet unexpectedly declared the xs prefix')

        marker = '<sch:ns prefix="ct"'
        if original.count(marker) != 1:
            raise ValueError('upstream schema changed: cannot add xs namespace for comparison')
        patched = original.replace(marker, '<sch:ns prefix="xs" uri="http://www.w3.org/2001/XMLSchema"/>' + marker, 1)
        validators = {
            'rust': run(transpiler, patched, schema.as_uri(), modules),
            'saxon': saxon_transpiler.transform_to_string(xdm_node=processor.parse_xml(xml_text=patched)),
        }
        invalid = VALID.replace('xslt2', 'xslt9').replace('<xs:element', '<xs:element xmlns:q="urn:extra"')
        cases = [('upstream-xsd', xsd.read_text()), ('valid-xsd', VALID), ('invalid-xsd', invalid)]
        baseline = {}
        for producer, generated in validators.items():
            validator = directory / (producer + '.xsl')
            validator.write_text(generated)
            saxon_validator = xslt.compile_stylesheet(stylesheet_file=str(validator))
            for name, source in cases:
                outputs = {
                    'rust': run(validator, source),
                    'saxon': saxon_validator.transform_to_string(xdm_node=processor.parse_xml(xml_text=source)),
                }
                for executor, output in outputs.items():
                    result = signature(output)
                    messages = {item[3] for item in result[1]}
                    if not result[0] or (name == 'valid-xsd' and result[1]):
                        raise AssertionError(f'unexpected result for {name}: {result}')
                    if name == 'upstream-xsd' and len(result[1]) != 2:
                        raise AssertionError('expected two upstream XSD findings')
                    if name == 'invalid-xsd' and not all(any(f'Rule {rule}:' in msg for msg in messages) for rule in ('3-8', '3-10')):
                        raise AssertionError(f'missing expected findings: {messages}')
                    if name in baseline and result != baseline[name]:
                        raise AssertionError(f'{producer} transpiler / {executor} validator disagrees on {name}: {result} vs {baseline[name]}')
                    baseline[name] = result
                    print('PASS', producer, 'transpiler /', executor, 'validator:', name,
                          'fired:', sum(result[0].values()), 'failed:', sum(result[1].values()), flush=True)


if __name__ == '__main__':
    main()
