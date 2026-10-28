use rust_xslt::Stylesheet;
fn source(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" xmlns:q="urn:p"><xsl:output method="text"/>{body}</xsl:stylesheet>"#
    )
}
fn evaluate(expr: &str, xml: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&source(&format!(
        r#"<xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template>"#
    )))?
    .transform(xml)
}
#[test]
fn named_node_tests_resolve_qnames_and_occurrence_indicators() {
    assert_eq!(
        evaluate(
            "r/* instance of element(q:n)+",
            "<r xmlns:p='urn:p'><p:n/><p:n/></r>"
        )
        .unwrap(),
        "true"
    );
    assert_eq!(
        evaluate(
            "r/* instance of element(Q{urn:p}n)",
            "<r xmlns:p='urn:p'><p:n/></r>"
        )
        .unwrap(),
        "true"
    );
    assert_eq!(
        evaluate(
            "r/* instance of element(n)",
            "<r xmlns:p='urn:p'><p:n/></r>"
        )
        .unwrap(),
        "false"
    );
    assert_eq!(
        evaluate("r/* instance of element(n)?", "<r/>").unwrap(),
        "true"
    );
}
#[test]
fn named_attribute_types_preserve_node_values() {
    let s = Stylesheet::compile(&source(r#"<xsl:variable name="a" as="attribute(q:v)" select="r/@p:v"/><xsl:template match="/"><xsl:value-of select="$a instance of attribute(Q{urn:p}v)"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r xmlns:p='urn:p' p:v='1'/>").unwrap(), "true");
    assert_eq!(
        evaluate("r/@v instance of attribute(n)", "<r v='1'/>").unwrap(),
        "false"
    );
}
#[test]
fn document_element_types_check_the_document_child_shape() {
    assert_eq!(
        evaluate(
            "/ instance of document-node(element(r))",
            "<!--c--><r/><?p x?>"
        )
        .unwrap(),
        "true"
    );
    assert_eq!(
        evaluate("/ instance of document-node(element(n))", "<r/>").unwrap(),
        "false"
    );
    assert_eq!(
        evaluate("/ instance of document-node(element(*))", "<r/>").unwrap(),
        "true"
    );
    let body = r#"<xsl:variable name="d"><a/><b/></xsl:variable><xsl:template match="/"><xsl:value-of select="$d instance of document-node(element())"/></xsl:template>"#;
    assert_eq!(
        Stylesheet::compile(&source(body))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "false"
    );
}
#[test]
fn named_template_result_types_check_constructed_names() {
    let s = Stylesheet::compile(&source(
        r#"<xsl:template match="/" as="element(q:n)"><p:n>ok</p:n></xsl:template>"#,
    ))
    .unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "ok");
    let s = Stylesheet::compile(&source(
        r#"<xsl:template match="/" as="element(n)"><other/></xsl:template>"#,
    ))
    .unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0505");
}
#[test]
fn named_template_parameter_mismatches_remain_supplied_value_errors() {
    let body = r#"<xsl:template match="/"><xsl:call-template name="t"><xsl:with-param name="n" select="r/*"/></xsl:call-template></xsl:template><xsl:template name="t"><xsl:param name="n" as="element(n)"/></xsl:template>"#;
    let s = Stylesheet::compile(&source(body)).unwrap();
    assert_eq!(s.transform("<r><other/></r>").unwrap_err().code, "XTTE0590");
}
#[test]
fn named_type_treat_as_and_prefix_errors_are_distinct() {
    assert_eq!(
        evaluate("r/* instance of element(a b)", "<r/>")
            .unwrap_err()
            .code,
        "XPST0003"
    );
    assert_eq!(
        evaluate("r/* treat as element(n)", "<r><other/></r>")
            .unwrap_err()
            .code,
        "XPDY0050"
    );
    assert_eq!(
        evaluate("r/* instance of element(z:n)", "<r/>")
            .unwrap_err()
            .code,
        "XPST0081"
    );
}
