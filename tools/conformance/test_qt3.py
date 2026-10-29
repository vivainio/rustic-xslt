import tempfile
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET
from qt3 import NS, HarnessError, ProfileExcluded, assertion, dependencies, environment, preflight, run_case

def node(xml):
    return ET.fromstring(xml.replace('<root>', f'<root xmlns="{NS[1:-1]}">'))[0]

class QT3Assertions(unittest.TestCase):
    def test_false_assertions_remain_engine_failures(self):
        class Driver:
            def call(self, request):
                return {"output": "false", "diagnostics": []}
        self.assertFalse(assertion(node('<root><assert-eq>2</assert-eq></root>'), {}, "1", {"namespaces": {}, "source": ""}, Driver()))

    def test_errors_never_satisfy_negated_non_error_assertions(self):
        self.assertFalse(assertion(node('<root><not><assert-true/></not></root>'), {"diagnostics": [{"code": "XPTY0004"}]}, "bad", {}, None))
        self.assertFalse(assertion(node('<root><error code="*"/></root>'), {"diagnostics": [{"code": "RUST_XSLT_UNSUPPORTED"}]}, "bad", {}, None))

    def test_preflight_does_not_skip_unsupported_assertions_or_attributes(self):
        for xml in ['<any-of><assert-true/><assert-serialization>1,2</assert-serialization></any-of>', '<assert-string-value ignored="yes">x</assert-string-value>', '<assert-string-value normalize-space="maybe">x</assert-string-value>']:
            with self.assertRaises(HarnessError):
                preflight(node('<root>' + xml + '</root>'))

    def test_assertion_evaluator_errors_are_harness_errors(self):
        class Driver:
            def call(self, request):
                return {"diagnostics": [{"code": "XPST0017", "message": "unsupported assertion function"}]}
        with self.assertRaises(HarnessError):
            assertion(node('<root><assert>unsupported()</assert></root>'), {}, "1", {"namespaces": {}, "source": ""}, Driver())

    def test_invalid_deep_equality_responses_are_harness_errors(self):
        class Driver:
            def call(self, request):
                return {"output": "unsupported-node;", "diagnostics": []}
        with self.assertRaises(HarnessError):
            assertion(node('<root><assert-deep-eq>1</assert-deep-eq></root>'), {}, "1", {"namespaces": {}, "source": ""}, Driver())

class QT3Setup(unittest.TestCase):
    def test_spec_dependencies_select_xpath_31_and_respect_negation(self):
        for value in ['XP20 XQ10', 'XP30 XQ30', 'XQ31+']:
            with self.assertRaises(ProfileExcluded):
                dependencies(node(f'<root><test-case><dependency type="spec" value="{value}"/></test-case></root>'))
        for value in ['XP31', 'XP31+ XQ31+', 'XP30+', 'XP20+']:
            dependencies(node(f'<root><test-case><dependency type="spec" value="{value}"/></test-case></root>'))
            with self.assertRaises(ProfileExcluded):
                dependencies(node(f'<root><test-case><dependency type="spec" value="{value}" satisfied="false"/></test-case></root>'))
        dependencies(node('<root><test-case><dependency type="spec" value="XP20" satisfied="false"/></test-case></root>'))

    def test_query_and_assertion_requests_use_the_xpath_function_profile(self):
        requests = []
        class Driver:
            def call(self, request):
                requests.append(request)
                return {"output": "true", "diagnostics": []}
        case = node('<root><test-case name="profile"><test>true()</test><result><assert-true/></result></test-case></root>')
        self.assertEqual(run_case(case, Path(), {}, Driver()), ("pass", ""))
        self.assertEqual(len(requests), 2)
        self.assertTrue(all(request.get("xpathOnly") is True for request in requests))

    def test_source_variables_and_resources_retain_uris_and_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            (base / 'source.xml').write_text('\ufeff<r/>', encoding='utf-8')
            (base / 'data.txt').write_text('resource content', encoding='utf-8')
            setup = node('<root><environment><source role="$doc" file="source.xml" uri="urn:source"/><source role="$other" file="source.xml"/><resource file="data.txt" uri="urn:data"/></environment></root>')
            result = environment(setup, {id(setup): (setup, base)})
            self.assertEqual(result['source'], '')
            self.assertEqual(result['source_variables'], [('doc', 'urn:source'), ('other', 'source.xml')])
            self.assertEqual(result['resources'], {'urn:source': '<r/>', 'source.xml': '<r/>', 'urn:data': 'resource content'})

    def test_only_explicit_alternative_profiles_are_excluded(self):
        with self.assertRaises(ProfileExcluded):
            dependencies(ET.fromstring(f'<test-case xmlns="{NS[1:-1]}"><dependency type="spec" value="XQ30+"/></test-case>'))
        with self.assertRaises(HarnessError):
            dependencies(ET.fromstring(f'<test-case xmlns="{NS[1:-1]}"><dependency type="unknown" value="anything"/></test-case>'))

    def test_environment_references_keep_the_definition_base_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            (base / 'source.xml').write_text('<r/>')
            parent = node('<root><environment name="parent"><source role="." file="source.xml"/><namespace prefix="p" uri="urn:p"/></environment></root>')
            reference = node('<root><environment ref="parent"/></root>')
            registry = {"parent": (parent, base), id(parent): (parent, base), id(reference): (reference, base / 'absent')}
            self.assertEqual(environment(reference, registry), {"source": "<r/>", "namespaces": {"p": "urn:p"}, "decimal_formats": [], "params": [], "resources": {}, "source_variables": []})

if __name__ == '__main__':
    unittest.main()
