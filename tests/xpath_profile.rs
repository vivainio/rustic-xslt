#![cfg(feature = "conformance")]
use rust_xslt::Stylesheet;

fn sheet(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:fn="http://www.w3.org/2005/xpath-functions"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}

#[test]
fn xpath_test_profile_rejects_xslt_only_functions_and_references() {
    for expression in [
        "current()",
        "fn:current()",
        "current#0",
        "key('k', 1)",
        "system-property('xsl:version')",
        "document('x')",
        "regex-group(1)",
    ] {
        assert_eq!(
            Stylesheet::compile_xpath_test_stylesheet(&sheet(expression))
                .unwrap_err()
                .code,
            "XPST0017",
            "{expression}"
        );
    }
}

#[test]
fn xpath_profile_retains_functions_transferred_from_xslt_to_xpath() {
    let expression = "(generate-id(()) eq '', exists(function-lookup(QName('http://www.w3.org/2005/xpath-functions', 'generate-id'), 1)))";
    assert_eq!(
        Stylesheet::compile_xpath_test_stylesheet(&sheet(expression))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "true true"
    );
}

#[test]
fn failed_profile_compiles_restore_the_xslt_function_library() {
    assert!(Stylesheet::compile_xpath_test_stylesheet(&sheet("current()")).is_err());
    assert_eq!(
        Stylesheet::compile(&sheet("name(current()/*)"))
            .unwrap()
            .transform("<root/>")
            .unwrap(),
        "root"
    );
    assert_eq!(
        Stylesheet::compile_xpath_test_stylesheet(&sheet("1 + 2"))
            .unwrap()
            .transform("<root/>")
            .unwrap(),
        "3"
    );
}

#[test]
fn xpath_function_lookup_uses_the_compiled_profile_at_runtime() {
    let expression =
        "empty(function-lookup(QName('http://www.w3.org/2005/xpath-functions','current'), 0))";
    let xpath = Stylesheet::compile_xpath_test_stylesheet(&sheet(expression)).unwrap();
    let xslt = Stylesheet::compile(&sheet(expression)).unwrap();
    assert_eq!(xpath.transform("<r/>").unwrap(), "true");
    assert_eq!(xslt.transform("<r/>").unwrap(), "false");
    assert_eq!(xpath.transform("<r/>").unwrap(), "true");
}
