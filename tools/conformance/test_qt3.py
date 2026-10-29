import tempfile
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET
from qt3 import NS, HarnessError, ProfileExcluded, assertion, dependencies, environment, preflight

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
            self.assertEqual(environment(reference, registry), {"source": "<r/>", "namespaces": {"p": "urn:p"}})

if __name__ == '__main__':
    unittest.main()
