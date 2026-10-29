use rust_xslt::Stylesheet;
fn run(output: &str, body: &str) -> String {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output {output}/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))
        .unwrap()
        .transform("<r/>")
        .unwrap()
}
#[test]
fn indent_yes_separates_element_only_content_onto_lines() {
    assert_eq!(
        run(
            r#"indent="yes" omit-xml-declaration="yes""#,
            "<a><b><c/></b><d x='1'>text</d></a>"
        ),
        "<a>\n  <b>\n    <c></c>\n  </b>\n  <d x=\"1\">text</d>\n</a>"
    );
    assert_eq!(
        run(r#"indent="yes""#, "<a><b/></a>"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<a>\n  <b></b>\n</a>"
    );
}
#[test]
fn mixed_content_comments_and_defaults_are_preserved() {
    assert_eq!(
        run(
            r#"indent="yes" omit-xml-declaration="yes""#,
            "<p>one <b>two</b> three</p>"
        ),
        "<p>one <b>two</b> three</p>"
    );
    assert_eq!(
        run(
            r#"indent="yes" omit-xml-declaration="yes""#,
            "<a><xsl:comment>c</xsl:comment><b/></a>"
        ),
        "<a>\n  <!--c-->\n  <b></b>\n</a>"
    );
    assert_eq!(
        run(r#"indent="no" omit-xml-declaration="yes""#, "<a><b/></a>"),
        "<a><b></b></a>"
    );
    assert_eq!(
        run(r#"omit-xml-declaration="yes""#, "<a> <b/> </a>"),
        "<a> <b></b> </a>"
    );
}
