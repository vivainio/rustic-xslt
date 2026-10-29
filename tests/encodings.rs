use rust_xslt::Stylesheet;
fn run(output: &str, body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output {output}/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r/>")
}
#[test]
fn latin1_and_ascii_escape_unrepresentable_characters() {
    assert_eq!(
        run(r#"encoding="ISO-8859-1""#, "<a b='é€'>ü€</a>").unwrap(),
        "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a b=\"é&#x20AC;\">ü&#x20AC;</a>"
    );
    assert_eq!(
        run(
            r#"encoding="us-ascii" omit-xml-declaration="yes""#,
            "<a>é</a>"
        )
        .unwrap(),
        "<a>&#xE9;</a>"
    );
    assert_eq!(
        run(
            r#"encoding="UTF-8" omit-xml-declaration="yes""#,
            "<a>é€</a>"
        )
        .unwrap(),
        "<a>é€</a>"
    );
}
#[test]
fn text_output_cannot_represent_missing_characters_and_other_encodings_are_rejected() {
    assert_eq!(
        run(r#"method="text" encoding="ISO-8859-1""#, "caf\u{e9}").unwrap(),
        "caf\u{e9}"
    );
    assert_eq!(
        run(r#"method="text" encoding="US-ASCII""#, "caf\u{e9}")
            .unwrap_err()
            .code,
        "SERE0008"
    );
    assert!(Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output encoding="EBCDIC"/></xsl:stylesheet>"#).is_err());
}
#[test]
fn cdata_section_elements_wrap_text_content() {
    assert_eq!(
        run(
            r#"omit-xml-declaration="yes" cdata-section-elements="code""#,
            "<doc><code>a &lt; b ]]&gt; c</code><p>x &lt; y</p></doc>"
        )
        .unwrap(),
        "<doc><code><![CDATA[a < b ]]]]><![CDATA[> c]]></code><p>x &lt; y</p></doc>"
    );
    assert_eq!(
        run(
            r#"omit-xml-declaration="yes" cdata-section-elements="a b""#,
            "<r><a>1</a><b>2</b><c>3</c></r>"
        )
        .unwrap(),
        "<r><a><![CDATA[1]]></a><b><![CDATA[2]]></b><c>3</c></r>"
    );
}
