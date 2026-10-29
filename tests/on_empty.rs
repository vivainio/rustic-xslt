use rust_xslt::Stylesheet;
fn run(body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r><i>1</i></r>")
}
#[test]
fn on_empty_runs_only_when_the_rest_is_empty() {
    assert_eq!(
        run("<out><xsl:for-each select='r/missing'><x/></xsl:for-each><xsl:on-empty><none/></xsl:on-empty></out>").unwrap(),
        "<out><none></none></out>"
    );
    assert_eq!(
        run("<out><xsl:for-each select='r/i'><x/></xsl:for-each><xsl:on-empty><none/></xsl:on-empty></out>").unwrap(),
        "<out><x></x></out>"
    );
    assert_eq!(
        run("<out><xsl:value-of select=\"'t'\"/><xsl:on-empty select=\"'empty'\"/></out>").unwrap(),
        "<out>t</out>"
    );
}
#[test]
fn on_non_empty_follows_non_empty_content() {
    assert_eq!(
        run("<out><xsl:for-each select='r/i'><x/></xsl:for-each><xsl:on-non-empty><done/></xsl:on-non-empty></out>").unwrap(),
        "<out><x></x><done></done></out>"
    );
    assert_eq!(
        run("<out><xsl:if test='false()'><x/></xsl:if><xsl:on-non-empty><done/></xsl:on-non-empty><xsl:on-empty>nothing</xsl:on-empty></out>").unwrap(),
        "<out>nothing</out>"
    );
    assert_eq!(
        run("<out><xsl:on-empty>always</xsl:on-empty></out>").unwrap(),
        "<out>always</out>"
    );
}
