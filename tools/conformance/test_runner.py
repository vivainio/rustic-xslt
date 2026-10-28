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

class ParameterExpressions(unittest.TestCase):
    def test_simple_atomic_parameter_expressions(self):
        for expression, expected in [("123456789012345678901234567890", {"type": "integer", "value": "123456789012345678901234567890"}), ("-1.25", {"type": "decimal", "value": "-1.25"}), ("1e3", {"type": "double", "value": "1e3"}), ("true()", True), ("false()", False), ("()", {"type": "empty"})]:
            self.assertEqual(parameters([node(f'<param name="v" select="{expression}"/>')]), {"v": expected})

    def test_initial_parameters_are_separate_from_globals(self):
        import tempfile
        from run import parse_catalog, run_case
        class RecordingDriver:
            def call(self, request):
                self.request = request
                return {"output": "ok", "method": "text", "diagnostics": []}
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "case.xml"
            path.write_text(f'''<test-case xmlns="{NS[1:-1]}" xmlns:p="urn:p"><test>
                <stylesheet><content>stylesheet</content></stylesheet>
                <param name="p:v" select="'global'"/>
                <initial-template name="p:entry"><param name="p:v" select="42"/></initial-template>
                </test><result><assert-string-value>ok</assert-string-value></result></test-case>''')
            case = parse_catalog(path)
            driver = RecordingDriver()
            self.assertEqual(run_case(case, Path(tmp), {}, driver)[0], "pass")
            self.assertEqual(driver.request["params"], {"Q{urn:p}v": "global"})
            self.assertEqual(driver.request["initialParams"], {"Q{urn:p}v": {"type": "integer", "value": "42"}})
            self.assertEqual(driver.request["initialTemplate"], "Q{urn:p}entry")

    def test_tunnel_initial_values_are_separated(self):
        entries = [node('<param name="ordinary" select="1"/>'), node('<param name="tunnel" select="2" tunnel="yes"/>')]
        self.assertEqual(parameters(entries, allow_tunnel=True), {"ordinary": {"type": "integer", "value": "1"}})
        self.assertEqual(parameters(entries, allow_tunnel=True, tunnel=True), {"tunnel": {"type": "integer", "value": "2"}})

    def test_complex_static_and_tunnel_parameters_remain_harness_errors(self):
        for xml in ['<param name="v" select="1+2"/>', '<param name="v" select="1" static="yes"/>', '<param name="v" select="1" tunnel="yes"/>']:
            with self.assertRaises(HarnessError):
                parameters([node(xml)])

class ComponentNames(unittest.TestCase):
    def catalog(self, xml):
        import tempfile
        from run import parse_catalog
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "catalog.xml"
            path.write_text(xml)
            return parse_catalog(path)

    def test_catalog_names_resolve_at_each_element(self):
        from run import component_name
        root = self.catalog('''<test xmlns="http://www.w3.org/2012/10/xslt-test-catalog" xmlns:p="urn:outer">
            <initial-template name="p:t"/><initial-mode name="p:m" xmlns:p="urn:inner"/>
            <param name="p:v" select="'value'"/><param name="v" select="'bare'"/>
            </test>''')
        self.assertEqual(component_name(root[0]), "Q{urn:outer}t")
        self.assertEqual(component_name(root[1]), "Q{urn:inner}m")
        self.assertEqual(parameters(root[2:]), {"Q{urn:outer}v": "value", "v": "bare"})

    def test_equivalent_parameter_names_are_duplicates(self):
        root = self.catalog('''<test xmlns:p="urn:p" xmlns:q="urn:p">
            <param name="p:v" select="'a'"/><param name="q:v" select="'b'"/>
            </test>''')
        with self.assertRaises(HarnessError):
            parameters(root)

    def test_invalid_or_unbound_catalog_names_are_harness_errors(self):
        from run import component_name
        root = self.catalog('<test><initial-template name="p:t"/><param name="a:b:c"/></test>')
        for entry in root:
            with self.assertRaises(HarnessError):
                component_name(entry)

    def test_uriqualified_names_and_empty_namespace(self):
        from run import component_name
        root = self.catalog('<test><param name="Q{urn:p}v"/><param name="Q{}v"/></test>')
        self.assertEqual(component_name(root[0]), "Q{urn:p}v")
        self.assertEqual(component_name(root[1]), "v")

if __name__ == '__main__':
    unittest.main()
