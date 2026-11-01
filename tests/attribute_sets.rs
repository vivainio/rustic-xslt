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
fn copy_applies_attribute_sets_only_to_elements_before_its_body() {
    assert_eq!(run(SETS, r#"<xsl:copy select="/r" use-attribute-sets="derived"><xsl:attribute name="a">override</xsl:attribute></xsl:copy>"#).unwrap(), r#"<r a="override" b="derived" d="merged" c="7"></r>"#);
    assert_eq!(run(SETS, r#"<xsl:copy select="/r" use-attribute-sets="base"><xsl:on-empty><empty/></xsl:on-empty></xsl:copy>"#).unwrap(), r#"<r a="base" b="base" d="merged"><empty></empty></r>"#);
    assert_eq!(
        run(
            SETS,
            r#"<xsl:copy select="/" use-attribute-sets="base"><child/></xsl:copy>"#
        )
        .unwrap(),
        "<child></child>"
    );
    assert_eq!(
        run(SETS, r#"<xsl:copy select="42" use-attribute-sets="base"/>"#).unwrap(),
        "42"
    );
    assert_eq!(
        run(
            "",
            r#"<xsl:copy select="/r" use-attribute-sets="missing"/>"#
        )
        .unwrap_err()
        .code,
        "XTSE0710"
    );
    let declarations = r#"<xsl:attribute-set name="focus"><xsl:attribute name="v" select="@v"/></xsl:attribute-set><xsl:attribute-set name="bad"><xsl:attribute name="x" select="error()"/></xsl:attribute-set>"#;
    let typed = r#"<xsl:variable name="v" as="element()"><xsl:copy select="/r" use-attribute-sets="focus"/></xsl:variable><xsl:copy-of select="$v"/>"#;
    assert_eq!(run(declarations, typed).unwrap(), r#"<r v="7"></r>"#);
    assert_eq!(
        run(
            declarations,
            r#"<xsl:copy select="/" use-attribute-sets="bad"><child/></xsl:copy>"#
        )
        .unwrap(),
        "<child></child>"
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

#[test]
fn attribute_sets_use_global_lexical_scope_with_the_callers_focus() {
    let declarations = r#"<xsl:variable name="v" select="'global'"/><xsl:attribute-set name="a"><xsl:attribute name="from-global" select="$v"/><xsl:attribute name="from-focus" select="@v"/><xsl:attribute name="from-local"><xsl:variable name="v" select="'inside'"/><xsl:value-of select="$v"/></xsl:attribute></xsl:attribute-set>"#;
    let body = r#"<xsl:variable name="v" select="'caller'"/><xsl:copy select="/r" use-attribute-sets="a"/>"#;
    assert_eq!(
        run(declarations, body).unwrap(),
        r#"<r from-global="global" from-focus="7" from-local="inside"></r>"#
    );
    let declarations = r#"<xsl:attribute-set name="a"><xsl:attribute name="x" select="$missing"/></xsl:attribute-set>"#;
    assert_eq!(
        run(
            declarations,
            r#"<xsl:variable name="missing" select="'caller'"/><out xsl:use-attribute-sets="a"/>"#
        )
        .unwrap_err()
        .code,
        "XPST0008"
    );
}

#[test]
fn attribute_sets_retain_current_group_and_regex_groups() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:attribute-set name="g"><xsl:attribute name="key" select="current-grouping-key()"/><xsl:attribute name="size" select="count(current-group())"/></xsl:attribute-set><xsl:attribute-set name="r"><xsl:attribute name="capture" select="regex-group(1)"/></xsl:attribute-set><xsl:template match="/"><out><xsl:for-each-group select="r/i" group-by="@k"><group xsl:use-attribute-sets="g"/></xsl:for-each-group><xsl:analyze-string select="'ab'" regex="(a)b"><xsl:matching-substring><match xsl:use-attribute-sets="r"/></xsl:matching-substring></xsl:analyze-string></out></xsl:template></xsl:stylesheet>"#;
    let result = Stylesheet::compile(source)
        .unwrap()
        .transform("<r><i k='x'/><i k='x'/><i k='y'/></r>")
        .unwrap();
    assert_eq!(
        result,
        r#"<out><group key="x" size="2"></group><group key="y" size="1"></group><match capture="a"></match></out>"#
    );
}
