use rust_xslt::Stylesheet;
fn run(body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:variable name="g" select="100"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r><n>1</n><n>2</n></r>")
}
#[test]
fn evaluate_runs_dynamic_xpath_with_context_and_parameters() {
    assert_eq!(run(r#"<xsl:evaluate xpath="'1 + 2'"/>"#).unwrap(), "3");
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'sum(n)'" context-item="r"/>"#).unwrap(),
        "3"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'$a * $b'"><xsl:with-param name="a" select="6"/><xsl:with-param name="b" select="7"/></xsl:evaluate>"#).unwrap(),
        "42"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'$g + 1'"/>"#).unwrap_err().code,
        "XTDE3160"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="&quot;concat('p', 'q')&quot;"/>"#).unwrap(),
        "pq"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'('"/>"#).unwrap_err().code,
        "XTDE3160"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'.'"/>"#).unwrap_err().code,
        "XPDY0002"
    );
    assert!(run(r#"<xsl:evaluate xpath="()"/>"#).is_err());
}
#[test]
fn where_populated_drops_empty_results() {
    assert_eq!(
        run("<out><xsl:where-populated><b/></xsl:where-populated></out>").unwrap(),
        "<out><b></b></out>"
    );
    assert_eq!(
        run("<out><xsl:where-populated><xsl:value-of select=\"()\"/></xsl:where-populated></out>")
            .unwrap(),
        "<out></out>"
    );
    assert_eq!(run("<out><xsl:where-populated><xsl:if test=\"false()\"><b/></xsl:if></xsl:where-populated></out>").unwrap(), "<out></out>");
    assert_eq!(
        run("<out><xsl:where-populated>text</xsl:where-populated></out>").unwrap(),
        "<out>text</out>"
    );
}
#[test]
fn namespace_instruction_declares_namespaces_on_the_current_element() {
    assert_eq!(
        run(r#"<out><xsl:namespace name="p" select="'urn:p'"/><xsl:namespace name="q">urn:q</xsl:namespace></out>"#).unwrap(),
        r#"<out xmlns:p="urn:p" xmlns:q="urn:q"></out>"#
    );
    assert_eq!(
        run(r#"<xsl:namespace name="p" select="'urn:p'"/>"#)
            .unwrap_err()
            .code,
        "XTDE0410"
    );
    assert_eq!(
        run(r#"<out><xsl:namespace name="1bad" select="'u'"/></out>"#)
            .unwrap_err()
            .code,
        "XTDE0920"
    );
}

#[test]
fn evaluate_target_uses_xpath_function_library() {
    for expr in [
        "system-property('xsl:version')",
        "element-available('xsl:text')",
        "current()",
        "system-property#1",
    ] {
        let body = format!(r#"<xsl:evaluate xpath="&quot;{expr}&quot;"/>"#);
        assert_eq!(run(&body).unwrap_err().code, "XTDE3160", "{expr}");
    }
    assert_eq!(
        run(r#"<xsl:evaluate xpath="&quot;concat('a','b')&quot;"/>"#).unwrap(),
        "ab"
    );
    assert_eq!(
        run(r#"<xsl:value-of select="system-property('xsl:version')"/>"#).unwrap(),
        "3.0"
    );
    let stylesheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:function name="f:test"><xsl:sequence select="42"/></xsl:function><xsl:template match="/"><xsl:evaluate xpath="'f:test()'"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(stylesheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "42"
    );
}

#[test]
fn evaluate_checks_target_variable_scope_before_execution() {
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'if (false()) then $g else 1'"/>"#)
            .unwrap_err()
            .code,
        "XTDE3160"
    );
    assert_eq!(run(r#"<xsl:evaluate xpath="'let $v := 4 return $v + $p'"><xsl:with-param name="p" select="3"/></xsl:evaluate>"#).unwrap(), "7");
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:variable name="global" select="41"/><xsl:function name="f:test"><xsl:sequence select="$global + 1"/></xsl:function><xsl:template match="/"><xsl:evaluate xpath="'f:test()'"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "42"
    );
}
