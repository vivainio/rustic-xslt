import unittest
from pathlib import Path
import xml.etree.ElementTree as ET
from run import NS, HarnessError, assertion, canonical, environment, fragment, parameters, preflight

def node(xml):
    return ET.fromstring(xml)

class Assertions(unittest.TestCase):
    def test_namespace_attributes_compare_without_python_none_ordering(self):
        a = fragment('<r xmlns:p="urn:p" id="a" p:id="b"/>')
        b = fragment('<r p:id="b" id="a" xmlns:p="urn:p"></r>')
        self.assertEqual(canonical(a), canonical(b))

    def test_xml_comparison_preserves_shape_comments_whitespace_and_prefixes(self):
        cases = [('<r><a/><b/></r>', '<r><a><b/></a></r>'), ('<r> </r>', '<r/>'), ('<r><!--a--></r>', '<r><!--b--></r>'), ('<p:r xmlns:p="u"/>', '<q:r xmlns:q="u"/>')]
        for a,b in cases:
            self.assertNotEqual(canonical(fragment(a)),canonical(fragment(b)))
        self.assertEqual(canonical(fragment(cases[-1][0]),True),canonical(fragment(cases[-1][1]),True))
        self.assertEqual(canonical(fragment('<r>a<![CDATA[b]]>c</r>')),canonical(fragment('<r>abc</r>')))

    def test_error_code_must_match_and_unsupported_is_not_wildcard_success(self):
        a = node('<error code="XTDE0410"/>')
        self.assertFalse(assertion(a,{'diagnostics':[{'code':'XPST0017'}]},Path())[0])
        self.assertTrue(assertion(a,{'diagnostics':[{'code':'XTDE0410'}]},Path())[0])
        self.assertFalse(assertion(node('<error code="*"/>'),{'diagnostics':[{'code':'RUST_XSLT_UNSUPPORTED'}]},Path())[0])

    def test_negation_does_not_turn_engine_errors_into_success(self):
        a = node('<not><assert-xml>&lt;r/&gt;</assert-xml></not>')
        self.assertFalse(assertion(a,{'diagnostics':[{'code':'ERR'}]},Path())[0])

    def test_unknown_assertion_fails_even_under_passing_any_of(self):
        with self.assertRaises(HarnessError):
            preflight(node('<any-of><assert-xml/><unknown-assertion/></any-of>'))

    def test_parameter_expression_is_really_one_string_literal(self):
        self.assertEqual(parameters([node('<param name="x" select="&apos;a&apos;&apos;b&apos;"/>')]),{'x':"a'b"})
        with self.assertRaises(HarnessError):
            parameters([node('<param name="x" select="&apos;a&apos; or &apos;b&apos;"/>')])

    def test_environment_reference_uses_each_definition_base_directory(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); (base/'global').mkdir(); (base/'case').mkdir()
            (base/'global'/'source.xml').write_text('<global/>')
            (base/'case'/'source.xml').write_text('<local/>')
            parent = node(f'<environment xmlns="{NS[1:-1]}" name="parent"><source role="." file="source.xml"/></environment>')
            reference = node(f'<environment xmlns="{NS[1:-1]}" ref="parent"><source role="." file="source.xml"/></environment>')
            registry = {'parent':(parent,base/'global'),id(parent):(parent,base/'global'),id(reference):(reference,base/'case')}
            self.assertEqual(environment(parent,registry)['source'],'<global/>')
            self.assertEqual(environment(reference,registry)['source'],'<local/>')


class XPathAssertions(unittest.TestCase):
    def test_assertion_namespace_scope_is_preserved(self):
        import tempfile
        from run import parse_catalog, assert_stylesheet
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)/'catalog.xml'
            path.write_text(f'<test-set xmlns="{NS[1:-1]}" xmlns:p="urn:outer"><result xmlns:p="urn:inner"><assert>$result/p:out</assert></result></test-set>')
            root = parse_catalog(path)
            a = root.find(NS+'result').find(NS+'assert')
            sheet = assert_stylesheet(a)
            self.assertIn('xmlns:p="urn:inner"',sheet)
            self.assertIn('boolean(($result/p:out))',sheet)

    def test_assertion_evaluation_errors_are_harness_failures(self):
        class BrokenDriver:
            def call(self, request):
                return {'diagnostics':[{'code':'XPST0017','message':'unsupported assertion function'}]}
        with self.assertRaises(HarnessError):
            assertion(node('<assert>unknown()</assert>'),{'method':'xml','output':'<r/>'},Path(),BrokenDriver())

    def test_assertion_evaluation_false_is_an_engine_failure(self):
        class FalseDriver:
            def call(self, request):
                return {'output':'false','diagnostics':[]}
        self.assertFalse(assertion(node('<assert>false()</assert>'),{'method':'xml','output':'<r/>'},Path(),FalseDriver())[0])

if __name__ == '__main__':
    unittest.main()
