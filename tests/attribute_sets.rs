use rust_xslt::Stylesheet;
fn run(declarations: &str, body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{declarations}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r v='7'/>")
}
const SETS: &str = r#"<xsl:attribute-set name="base"><xsl:attribute name="a">base</xsl:attribute><xsl:attribute name="b">base</xsl:attribute></xsl:attribute-set>
<xsl:attribute-set name="derived" use-attribute-sets="base"><xsl:attribute name="b">derived</xsl:attribute><xsl:attribute name="c" select="/r/@v"/></xsl:attribute-set>
<xsl:attribute-set name="base"><xsl:attribute name="d">merged</xsl:attribute></xsl:attribute-set>"#;
#[test]
fn literal_and_computed_elements_apply_attribute_sets() {
    assert_eq!(
        run(SETS, r#"<out xsl:use-attribute-sets="base"/>"#).unwrap(),
        r#"<out a="base" b="base" d="merged"></out>"#
    );
    assert_eq!(
        run(SETS, r#"<out xsl:use-attribute-sets="derived"/>"#).unwrap(),
        r#"<out a="base" b="derived" d="merged" c="7"></out>"#
    );
    assert_eq!(
        run(SETS, r#"<xsl:element name="e" use-attribute-sets="derived"><xsl:attribute name="a">own</xsl:attribute></xsl:element>"#).unwrap(),
        r#"<e a="own" b="derived" d="merged" c="7"></e>"#
    );
}
#[test]
fn literal_attributes_override_attribute_set_attributes() {
    assert_eq!(
        run(SETS, r#"<out b="literal" xsl:use-attribute-sets="base"/>"#).unwrap(),
        r#"<out a="base" b="literal" d="merged"></out>"#
    );
}
#[test]
fn attribute_set_errors_are_static() {
    assert_eq!(
        run("", r#"<out xsl:use-attribute-sets="nope"/>"#)
            .unwrap_err()
            .code,
        "XTSE0710"
    );
    let cycle = r#"<xsl:attribute-set name="x" use-attribute-sets="y"/><xsl:attribute-set name="y" use-attribute-sets="x"/>"#;
    assert_eq!(run(cycle, "<out/>").unwrap_err().code, "XTSE0720");
    let bad = r#"<xsl:attribute-set name="x"><b/></xsl:attribute-set>"#;
    assert_eq!(run(bad, "<out/>").unwrap_err().code, "XTSE0010");
    let missing = r#"<xsl:attribute-set name="x" use-attribute-sets="nope"/>"#;
    assert_eq!(run(missing, "<out/>").unwrap_err().code, "XTSE0710");
}
