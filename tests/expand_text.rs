use rust_xslt::Stylesheet;
fn run(stylesheet_attrs: &str, body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" {stylesheet_attrs}><xsl:output omit-xml-declaration="yes"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r><n>1</n><n>2</n></r>")
}
#[test]
fn expand_text_turns_text_nodes_into_value_templates() {
    assert_eq!(
        run(
            r#"expand-text="yes""#,
            "<p>total: {sum(r/n)}, items: {r/n}</p>"
        )
        .unwrap(),
        "<p>total: 3, items: 1 2</p>"
    );
    assert_eq!(
        run(r#"expand-text="yes""#, "<p>{{literal}} {1 + 1}</p>").unwrap(),
        "<p>{literal} 2</p>"
    );
    assert_eq!(
        run(
            r#"expand-text="yes""#,
            "<p><xsl:text>{count(r/n)} nodes</xsl:text></p>"
        )
        .unwrap(),
        "<p>2 nodes</p>"
    );
    assert_eq!(
        run(r#"expand-text="yes""#, "<p>{upper-case('x')}</p>").unwrap(),
        "<p>X</p>"
    );
}
#[test]
fn expand_text_is_off_by_default_and_can_be_scoped() {
    assert_eq!(run("", "<p>{1 + 1}</p>").unwrap(), "<p>{1 + 1}</p>");
    assert_eq!(
        run(r#"expand-text="yes""#, r#"<p xsl:expand-text="no">{1}</p>"#).unwrap(),
        "<p>{1}</p>"
    );
    assert_eq!(
        run("", r#"<p xsl:expand-text="yes">{1 + 1}<q>{2 + 2}</q></p>"#).unwrap(),
        "<p>2<q>4</q></p>"
    );
    assert_eq!(
        run(
            "",
            r#"<xsl:if test="true()" expand-text="yes">{3}</xsl:if>"#
        )
        .unwrap(),
        "3"
    );
}
#[test]
fn malformed_value_templates_are_static_errors() {
    assert!(run(r#"expand-text="yes""#, "<p>}</p>").is_err());
    assert!(run(r#"expand-text="yes""#, "<p>{1 +</p>").is_err());
    assert!(run(r#"expand-text="maybe""#, "<p/>").is_err());
}
