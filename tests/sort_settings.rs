use rust_xslt::Stylesheet;
fn run(sort: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:param name="dir" select="'descending'"/><xsl:param name="kind" select="'number'"/><xsl:template match="/"><xsl:for-each select="r/i">{sort}<xsl:value-of select="."/>,</xsl:for-each></xsl:template></xsl:stylesheet>"#))?
        .transform("<r><i>10</i><i>9</i><i>b</i><i>B</i><i>a</i><i>2</i></r>")
}
#[test]
fn sort_settings_may_be_attribute_value_templates() {
    assert_eq!(
        run(r#"<xsl:sort select="." data-type="number" order="{$dir}"/>"#).unwrap(),
        "10,9,2,b,B,a,"
    );
    assert_eq!(
        run(r#"<xsl:sort select="number(.)" data-type="{$kind}" order="descending"/>"#).unwrap(),
        "10,9,2,b,B,a,"
    );
    assert_eq!(
        run(r#"<xsl:sort select="." order="{'asc' || 'ending'}"/>"#).unwrap(),
        "10,2,9,B,a,b,"
    );
    assert_eq!(
        run(r#"<xsl:sort select="." case-order="{'upper-first'}"/>"#).unwrap(),
        "10,2,9,a,B,b,"
    );
    assert_eq!(
        run(r#"<xsl:sort select="." order="{'sideways'}"/>"#)
            .unwrap_err()
            .code,
        "XTDE0030"
    );
}
#[test]
fn sort_collations_lang_and_stable_are_accepted() {
    assert_eq!(
        run(r#"<xsl:sort select="." collation="http://www.w3.org/2005/xpath-functions/collation/html-ascii-case-insensitive" lang="en" stable="yes"/>"#).unwrap(),
        "10,2,9,a,b,B,"
    );
    assert_eq!(
        run(r#"<xsl:sort select="." collation="http://www.w3.org/2013/collation/UCA?strength=primary"/>"#).unwrap(),
        "10,2,9,a,b,B,"
    );
    assert!(run(r#"<xsl:sort select="." collation="urn:unknown"/>"#).is_err());
    assert!(run(r#"<xsl:sort select="." collation="{'urn:unknown'}"/>"#).is_err());
}
