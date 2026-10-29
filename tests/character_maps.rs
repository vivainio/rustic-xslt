use rust_xslt::Stylesheet;
fn run(declarations: &str, body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">{declarations}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r/>")
}
#[test]
fn mapped_characters_are_written_verbatim_in_text_and_attributes() {
    let declarations = r##"<xsl:output omit-xml-declaration="yes" use-character-maps="m"/><xsl:character-map name="m"><xsl:output-character character="&#xA0;" string="&amp;nbsp;"/><xsl:output-character character="#" string="&lt;hash/&gt;"/></xsl:character-map>"##;
    assert_eq!(
        run(declarations, "<p a='x#y'>a&#xA0;b#c &amp; d</p>").unwrap(),
        "<p a=\"x<hash/>y\">a&nbsp;b<hash/>c &amp; d</p>"
    );
}
#[test]
fn character_maps_can_use_other_maps_and_errors_are_static() {
    let nested = r#"<xsl:output omit-xml-declaration="yes" use-character-maps="outer"/><xsl:character-map name="inner"><xsl:output-character character="a" string="1"/><xsl:output-character character="b" string="2"/></xsl:character-map><xsl:character-map name="outer" use-character-maps="inner"><xsl:output-character character="b" string="B"/></xsl:character-map>"#;
    assert_eq!(run(nested, "<p>abc</p>").unwrap(), "<p>1Bc</p>");
    assert_eq!(
        run(r#"<xsl:output use-character-maps="missing"/>"#, "<p/>")
            .unwrap_err()
            .code,
        "XTSE1590"
    );
    let cycle = r#"<xsl:output use-character-maps="a"/><xsl:character-map name="a" use-character-maps="b"/><xsl:character-map name="b" use-character-maps="a"/>"#;
    assert_eq!(run(cycle, "<p/>").unwrap_err().code, "XTSE1600");
    let duplicate = r#"<xsl:character-map name="a"/><xsl:character-map name="a"/>"#;
    assert_eq!(run(duplicate, "<p/>").unwrap_err().code, "XTSE1580");
    let long = r#"<xsl:character-map name="a"><xsl:output-character character="ab" string="x"/></xsl:character-map>"#;
    assert_eq!(run(long, "<p/>").unwrap_err().code, "XTSE0020");
}

#[test]
fn result_document_maps_are_checked_in_unexecuted_branches() {
    let error = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:if test="false()"><xsl:result-document use-character-maps="missing"/></xsl:if></xsl:template></xsl:stylesheet>"#).err().unwrap();
    assert_eq!(error.code, "XTSE1590");
    assert_eq!(error.phase, "compile");
}
