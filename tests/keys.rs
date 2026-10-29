use rust_xslt::Stylesheet;
const SOURCE: &str = "<r><item id='a' g='x'>1</item><item id='b' g='y'>2</item><item id='c' g='x'>3</item><grp><item id='d' g='x'>4</item></grp></r>";
fn run(declarations: &str, select: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:k="urn:k"><xsl:output method="text"/>{declarations}<xsl:template match="/"><xsl:sequence select="{select}"/></xsl:template></xsl:stylesheet>"#))?
        .transform(SOURCE)
}
const KEYS: &str = r#"<xsl:key name="by-id" match="item" use="@id"/><xsl:key name="by-group" match="item" use="@g"/><xsl:key name="q" xmlns:k="urn:k" match="item" use="string(.)"/>"#;
#[test]
fn key_returns_matching_nodes_in_document_order() {
    for (select, expected) in [
        ("string(key('by-id', 'b'))", "2"),
        ("string-join(key('by-group', 'x'), ',')", "1,3,4"),
        ("string-join(key('by-group', ('y', 'x')), ',')", "1,2,3,4"),
        ("count(key('by-group', 'none'))", "0"),
        ("count(key('by-group', ()))", "0"),
        ("string(key('by-id', 'a')/@g)", "x"),
        ("string-join(key('by-group', 'x', r/grp), ',')", "1,3,4"),
        ("string-join(r/grp/key('by-group', 'x'), ',')", "1,3,4"),
        ("string(key('q', 3))", "3"),
        ("count(key('by-id', 'a') | key('by-group', 'x'))", "3"),
    ] {
        assert_eq!(run(KEYS, select).unwrap(), expected, "{select}");
    }
}
#[test]
fn key_names_may_be_qualified_and_errors_are_reported() {
    let qualified = r#"<xsl:key name="k:ids" match="item" use="@id"/>"#;
    assert_eq!(run(qualified, "string(key('k:ids', 'c'))").unwrap(), "3");
    assert_eq!(
        run(KEYS, "key('missing', 'a')").unwrap_err().code,
        "XTDE1260"
    );
    assert!(Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:key name="x" match="a"/></xsl:stylesheet>"#).is_err());
}
