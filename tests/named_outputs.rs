use rust_xslt::{Stylesheet, TransformOptions};
fn run(
    declarations: &str,
    body: &str,
) -> Result<rust_xslt::TransformResult, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:output" xmlns:b="urn:output" exclude-result-prefixes="#all">{declarations}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"##))?
        .transform_with_documents("<r/>", &TransformOptions::default())
}

#[test]
fn named_outputs_select_their_own_method_and_leave_the_principal_definition_independent() {
    let result = run(r#"<xsl:output method="text"/><xsl:output name="a:xml" method="xml" omit-xml-declaration="yes"/><xsl:output name="text" method="text"/>"#,
        r#"principal<xsl:result-document href="a" format="b:xml"><p>xml</p></xsl:result-document><xsl:result-document href="b" format="text"><p>text</p></xsl:result-document>"#).unwrap();
    assert_eq!(result.principal, "principal");
    assert_eq!(result.documents[0].content, "<p>xml</p>");
    assert_eq!(result.documents[1].content, "text");
}

#[test]
fn format_avts_resolve_names_in_the_instruction_static_namespace_scope() {
    let declarations = r#"<xsl:output name="a:xml" omit-xml-declaration="yes"/>"#;
    let body = r#"<xsl:variable name="format" select="'b:xml'"/><xsl:result-document href="a" format="{$format}"><p/></xsl:result-document>"#;
    assert_eq!(
        run(declarations, body).unwrap().documents[0].content,
        "<p></p>"
    );
    for format in ["missing", "undeclared:xml", ""] {
        let body = format!(
            r#"<xsl:result-document href="a" format="{format}"><p/></xsl:result-document>"#
        );
        assert_eq!(run(declarations, &body).unwrap_err().code, "XTDE1460");
    }
    assert_eq!(
        run(
            declarations,
            r#"<xsl:result-document href="a" format="{$missing}"/>"#
        )
        .unwrap_err()
        .code,
        "XPST0008"
    );
}

#[test]
fn named_declarations_merge_by_expanded_name() {
    let declarations = r#"<xsl:output name="a:xml" method="xml" omit-xml-declaration="yes"/><xsl:output name="b:xml" indent="yes"/>"#;
    assert_eq!(
        run(
            declarations,
            r#"<xsl:result-document href="a" format="a:xml"><p><q/></p></xsl:result-document>"#
        )
        .unwrap()
        .documents[0]
            .content,
        "<p>\n  <q></q>\n</p>"
    );
    let declarations = format!(r#"{declarations}<xsl:output name="a:xml" indent="no"/>"#);
    assert_eq!(run(&declarations, "").unwrap_err().code, "XTSE1560");
}

#[test]
fn cdata_character_maps_and_declaration_settings_belong_to_each_named_definition() {
    let declarations = r#"<xsl:output method="text"/><xsl:character-map name="m"><xsl:output-character character="~" string="mapped"/></xsl:character-map>
      <xsl:output name="xml" omit-xml-declaration="yes" cdata-section-elements="p"/>
      <xsl:output name="text" method="text" use-character-maps="m" byte-order-mark="yes"/>"#;
    let result = run(declarations, r#"<xsl:result-document href="a" format="xml"><p>&lt;x&gt;</p></xsl:result-document><xsl:result-document href="b" format="text">~</xsl:result-document>"#).unwrap();
    assert_eq!(result.documents[0].content, "<p><![CDATA[<x>]]></p>");
    assert_eq!(result.documents[1].content, "\u{feff}mapped");
}

#[test]
fn secondary_results_inherit_unnamed_settings_and_allow_explicit_overrides() {
    let result = run(r#"<xsl:output method="text" encoding="US-ASCII" omit-xml-declaration="yes"/>"#,
        r#"<xsl:result-document href="a"><p>text</p></xsl:result-document><xsl:result-document href="b" method="xml"><p>xml</p></xsl:result-document>"#).unwrap();
    assert_eq!(result.documents[0].content, "text");
    assert_eq!(result.documents[1].content, "<p>xml</p>");
    assert_eq!(
        run(
            r#"<xsl:output method="text" encoding="US-ASCII"/>"#,
            r#"<xsl:result-document href="a">é</xsl:result-document>"#
        )
        .unwrap_err()
        .code,
        "SERE0008"
    );
}

#[test]
fn character_maps_do_not_change_text_in_cdata_sections() {
    let declarations = r#"<xsl:character-map name="m"><xsl:output-character character="~" string="&amp;raw;"/></xsl:character-map><xsl:output name="xml" omit-xml-declaration="yes" cdata-section-elements="p" use-character-maps="m"/>"#;
    let result = run(
        declarations,
        r#"<xsl:result-document href="a" format="xml"><p>a~b</p></xsl:result-document>"#,
    )
    .unwrap();
    assert_eq!(result.documents[0].content, "<p><![CDATA[a~b]]></p>");
}

#[cfg(feature = "conformance")]
#[test]
fn development_metadata_uses_the_compiled_unnamed_output_method() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output name="named" method="text"/><xsl:output indent="yes"/><xsl:output method="html"/></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .conformance_output_method(),
        "html"
    );
}
