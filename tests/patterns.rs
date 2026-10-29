use rust_xslt::Stylesheet;
const DOC: &str = "<doc><chap id='c1'><p>a</p><sec><p>b</p><p>c</p></sec></chap><chap id='c2'><q>d</q><p>e</p></chap></doc>";
fn run(templates: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:key name="k" match="chap" use="@id"/>{templates}<xsl:template match="text()"/><xsl:template match="*" priority="-10"/></xsl:stylesheet>"#))?
        .transform(DOC)
}
#[test]
fn descendant_separators_match_any_depth() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="chap//p">[<xsl:value-of select="."/>]</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "[a][b][c][e]");
    let rooted = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="//sec/p">S<xsl:value-of select="."/></xsl:template><xsl:template match="/doc//q">Q</xsl:template>"#;
    assert_eq!(run(rooted).unwrap(), "SbScQ");
    let chain = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="doc//sec//p">D</xsl:template>"#;
    assert_eq!(run(chain).unwrap(), "DD");
}
#[test]
fn parenthesized_unions_expand_into_alternatives() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="chap/(p|q)">X</xsl:template><xsl:template match="(sec|doc)/p">Y</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "XYYXX");
    let nested = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="(chap|sec)//p">Z</xsl:template>"#;
    assert_eq!(run(nested).unwrap(), "ZZZZ");
}
#[test]
fn key_and_id_patterns_match_selected_nodes() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//chap"/></xsl:template><xsl:template match="key('k', 'c2')">second</xsl:template><xsl:template match="chap">other</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "othersecond");
}
#[test]
fn default_priorities_account_for_path_shape() {
    // The compound pattern (priority 0.5) beats the plain name test (priority 0).
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//p"/></xsl:template><xsl:template match="p">plain </xsl:template><xsl:template match="sec//p">deep </xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "plain deep deep plain ");
    assert!(run(r#"<xsl:template match="a/following::b"/>"#).is_err());
    assert!(run(r#"<xsl:template match="a//"/>"#).is_err());
}
