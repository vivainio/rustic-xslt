use rust_xslt::Stylesheet;
fn run(body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r>v</r>")
}
#[test]
fn messages_do_not_contribute_to_the_result() {
    assert_eq!(run("a<xsl:message>hello</xsl:message>b").unwrap(), "ab");
    assert_eq!(run("<xsl:message select='r'/>x").unwrap(), "x");
    assert_eq!(
        run("<xsl:message terminate='no'>t</xsl:message>y").unwrap(),
        "y"
    );
}
#[test]
fn terminating_messages_stop_the_transformation_with_their_text() {
    let error = run("a<xsl:message terminate='yes'>stop <xsl:value-of select='r'/></xsl:message>b")
        .unwrap_err();
    assert_eq!(error.code, "XTMM9000");
    assert!(error.message.contains("stop v"), "{}", error.message);
    assert!(run("<xsl:message terminate='maybe'/>").is_err());
}
#[test]
fn messages_are_collected_in_order_for_callers_that_ask() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:message>first</xsl:message>x<xsl:message select="1 + 1"/><xsl:message terminate="no">third <xsl:value-of select="'!'"/></xsl:message></xsl:template></xsl:stylesheet>"#).unwrap();
    let result = sheet
        .transform_with_documents("<r/>", &rust_xslt::TransformOptions::default())
        .unwrap();
    assert_eq!(result.principal, "x");
    assert_eq!(result.messages, vec!["first", "2", "third !"]);
}
#[test]
fn assertions_fail_with_their_message_and_pass_silently() {
    assert_eq!(
        run("a<xsl:assert test='1 = 1'>never</xsl:assert>b").unwrap(),
        "ab"
    );
    let failure =
        run("<xsl:assert test='1 = 2'>value was <xsl:value-of select='1 + 1'/></xsl:assert>")
            .unwrap_err();
    assert_eq!(failure.code, "XTMM9001");
    assert!(
        failure.message.contains("value was 2"),
        "{}",
        failure.message
    );
    assert_eq!(
        run("<xsl:assert test=\"false()\" select=\"'plain'\"/>")
            .unwrap_err()
            .code,
        "XTMM9001"
    );
    assert!(run("<xsl:assert/>").is_err());
}
