use rust_xslt::Stylesheet;
fn run(body: &str, source: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform(source)
}
#[test]
fn matching_and_non_matching_substrings_are_visited_in_order() {
    let body = r#"<xsl:analyze-string select="r" regex="[0-9]+"><xsl:matching-substring>[<xsl:value-of select="."/>]</xsl:matching-substring><xsl:non-matching-substring>(<xsl:value-of select="."/>)</xsl:non-matching-substring></xsl:analyze-string>"#;
    assert_eq!(run(body, "<r>ab12cd345</r>").unwrap(), "(ab)[12](cd)[345]");
    assert_eq!(run(body, "<r>7</r>").unwrap(), "[7]");
    assert_eq!(run(body, "<r>xyz</r>").unwrap(), "(xyz)");
    assert_eq!(run(body, "<r/>").unwrap(), "");
}
#[test]
fn regex_groups_and_flags_are_available_to_the_matching_body() {
    let body = r#"<xsl:analyze-string select="'k1=v1;K2=v2'" regex="(\w+)=(\w+)" flags="i"><xsl:matching-substring><xsl:value-of select="regex-group(2), regex-group(1)"/>|</xsl:matching-substring></xsl:analyze-string>"#;
    assert_eq!(run(body, "<r/>").unwrap(), "v1 k1|v2 K2|");
    let optional = r#"<xsl:analyze-string select="'ab'" regex="a(x)?b"><xsl:matching-substring>[<xsl:value-of select="regex-group(1)"/>]<xsl:value-of select="regex-group(9)"/></xsl:matching-substring></xsl:analyze-string>"#;
    assert_eq!(run(optional, "<r/>").unwrap(), "[]");
    let avt = r#"<xsl:analyze-string select="'a-b'" regex="{'-'}"><xsl:non-matching-substring><xsl:value-of select="."/>,</xsl:non-matching-substring></xsl:analyze-string>"#;
    assert_eq!(run(avt, "<r/>").unwrap(), "a,b,");
}
#[test]
fn analyze_string_errors() {
    assert_eq!(
        run(r#"<xsl:analyze-string select="'a'" regex="x*"><xsl:matching-substring/></xsl:analyze-string>"#, "<r/>").unwrap_err().code,
        "XTDE1150"
    );
    assert_eq!(
        run(r#"<xsl:analyze-string select="'a'" regex="a"/>"#, "<r/>")
            .unwrap_err()
            .code,
        "XTSE1130"
    );
    assert!(run(r#"<xsl:analyze-string select="'a'" regex="("><xsl:matching-substring/></xsl:analyze-string>"#, "<r/>").is_err());
    assert!(run(r#"<xsl:analyze-string select="(1,2)" regex="a"><xsl:matching-substring/></xsl:analyze-string>"#, "<r/>").is_err());
}
