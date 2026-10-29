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

#[test]
fn restricted_encodings_reject_unrepresentable_comment_and_pi_content() {
    for body in [
        "<xsl:comment>é</xsl:comment>",
        "<xsl:processing-instruction name='p'>é</xsl:processing-instruction>",
    ] {
        let error = run(r#"encoding="US-ASCII""#, body).unwrap_err();
        assert_eq!(error.code, "SERE0008");
    }
    assert_eq!(
        run(
            r#"encoding="ISO-8859-1" omit-xml-declaration="yes""#,
            "<xsl:comment>é</xsl:comment>"
        )
        .unwrap(),
        "<!--é-->"
    );
}

#[test]
fn restricted_encodings_reject_unrepresentable_markup_names() {
    for body in ["<é/>", "<out é='value'/>"] {
        assert_eq!(
            run(r#"encoding="US-ASCII""#, body).unwrap_err().code,
            "SERE0008"
        );
    }
    assert_eq!(
        run(
            r#"encoding="US-ASCII" omit-xml-declaration="yes""#,
            "<out attr='é'/>"
        )
        .unwrap(),
        "<out attr=\"&#xE9;\"></out>"
    );
}

#[test]
fn output_limits_include_encoding_expansion() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output encoding="US-ASCII" omit-xml-declaration="yes"/><xsl:template match="/"><a>é</a></xsl:template></xsl:stylesheet>"#).unwrap();
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_output_bytes: 10,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet
            .transform_with_options("<r/>", &options)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}

#[test]
fn restricted_encodings_reject_unrepresentable_doctype_identifiers() {
    assert_eq!(
        run(r#"encoding="US-ASCII" doctype-system="café.dtd""#, "<out/>")
            .unwrap_err()
            .code,
        "SERE0008"
    );
    assert!(
        run(
            r#"encoding="ISO-8859-1" doctype-system="café.dtd""#,
            "<out/>"
        )
        .unwrap()
        .contains("café.dtd")
    );
}

#[test]
fn markup_like_character_map_text_in_attributes_preserves_encoding_state() {
    let sheet = Stylesheet::compile(r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output encoding="US-ASCII" omit-xml-declaration="yes" use-character-maps="m"/><xsl:character-map name="m"><xsl:output-character character="#" string="&lt;!--"/></xsl:character-map><xsl:template match="/"><out a="#é"/></xsl:template></xsl:stylesheet>"##).unwrap();
    assert_eq!(
        sheet.transform("<r/>").unwrap(),
        "<out a=\"<!--&#xE9;\"></out>"
    );
}

#[test]
fn cdata_carriage_returns_are_serialized_outside_sections() {
    assert_eq!(
        run(
            r#"omit-xml-declaration="yes" cdata-section-elements="code""#,
            r#"<code><xsl:value-of select="codepoints-to-string((97,13,98))"/></code>"#
        )
        .unwrap(),
        "<code><![CDATA[a]]>&#13;<![CDATA[b]]></code>"
    );
}
