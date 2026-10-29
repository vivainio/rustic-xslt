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
