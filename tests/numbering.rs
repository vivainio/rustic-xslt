use rust_xslt::Stylesheet;
fn run(body: &str, source: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform(source)
}
const DOC: &str = "<d><s><p/><p/><t/><p/></s><s><p/><s><p/><p/></s></s></d>";
#[test]
fn value_attribute_formats_numbers() {
    for (format, value, expected) in [
        ("1", "7", "7"),
        ("001", "7", "007"),
        ("a", "27", "aa"),
        ("A", "3", "C"),
        ("i", "1994", "mcmxciv"),
        ("I", "4", "IV"),
        ("w", "42", "forty-two"),
        ("W", "101", "ONE HUNDRED AND ONE"),
        ("Ww", "21", "Twenty-One"),
        ("(1)", "5", "(5)"),
        ("1.", "5", "5."),
        ("a)", "2", "b)"),
    ] {
        let body = format!(r#"<xsl:number value="{value}" format="{format}"/>"#);
        assert_eq!(run(&body, "<d/>").unwrap(), expected, "{format} {value}");
    }
    assert_eq!(
        run(
            r#"<xsl:number value="3" format="1" ordinal="yes"/>"#,
            "<d/>"
        )
        .unwrap(),
        "3rd"
    );
    assert_eq!(
        run(
            r#"<xsl:number value="21" format="w" ordinal="yes"/>"#,
            "<d/>"
        )
        .unwrap(),
        "twenty-first"
    );
    assert_eq!(
        run(
            r#"<xsl:number value="1234567" grouping-separator="," grouping-size="3"/>"#,
            "<d/>"
        )
        .unwrap(),
        "1,234,567"
    );
    assert_eq!(
        run(r#"<xsl:number value="(1, 2, 3)" format="1.a.i"/>"#, "<d/>").unwrap(),
        "1.b.iii"
    );
    assert_eq!(run(r#"<xsl:number value="2.5"/>"#, "<d/>").unwrap(), "3");
    assert_eq!(run(r#"<xsl:number value="0"/>"#, "<d/>").unwrap(), "0");
    assert_eq!(
        run(r#"<xsl:number value="-1"/>"#, "<d/>").unwrap_err().code,
        "XTDE0980"
    );
}
#[test]
fn single_level_counts_preceding_siblings() {
    let body = r#"<xsl:for-each select="//p"><xsl:number/><xsl:text> </xsl:text></xsl:for-each>"#;
    assert_eq!(run(body, DOC).unwrap(), "1 2 3 1 1 2 ");
    let counted = r#"<xsl:for-each select="//p"><xsl:number count="p|t"/>,</xsl:for-each>"#;
    assert_eq!(run(counted, DOC).unwrap(), "1,2,4,1,1,2,");
}
#[test]
fn multiple_level_builds_hierarchical_numbers() {
    let body = r#"<xsl:for-each select="//p"><xsl:number level="multiple" count="s|p" format="1.1"/>;</xsl:for-each>"#;
    assert_eq!(run(body, DOC).unwrap(), "1.1;1.2;1.3;2.1;2.2.1;2.2.2;");
}
#[test]
fn any_level_counts_in_document_order() {
    let body = r#"<xsl:for-each select="//p"><xsl:number level="any"/>,</xsl:for-each>"#;
    assert_eq!(run(body, DOC).unwrap(), "1,2,3,4,5,6,");
    let from = r#"<xsl:for-each select="//p"><xsl:number level="any" from="s"/>,</xsl:for-each>"#;
    assert_eq!(run(from, DOC).unwrap(), "1,2,3,1,1,2,");
    let select = r#"<xsl:number select="d/s[2]/s/p[2]" level="any" count="p"/>"#;
    assert_eq!(run(select, DOC).unwrap(), "6");
}
#[test]
fn number_rejects_conflicting_attributes_and_non_nodes() {
    assert_eq!(
        run(r#"<xsl:number value="1" count="p"/>"#, DOC)
            .unwrap_err()
            .code,
        "XTSE0975"
    );
    assert!(run(r#"<xsl:number level="deep"/>"#, DOC).is_err());
    assert!(run(r#"<xsl:number select="1"/>"#, DOC).is_err());
}
