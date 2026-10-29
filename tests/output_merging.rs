use rust_xslt::Stylesheet;
fn style(declarations: &str, body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">{declarations}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#
    )
}

#[test]
fn unnamed_output_declarations_merge_independent_properties() {
    let source = style(
        r#"<xsl:output method="text"/><xsl:output encoding="US-ASCII"/><xsl:output indent="no"/>"#,
        "plain",
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<s/>")
            .unwrap(),
        "plain"
    );
    let source = style(
        r#"<xsl:output method="html"/><xsl:output indent="no"/>"#,
        "<br/>",
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<s/>")
            .unwrap(),
        "<br>"
    );
}

#[test]
fn duplicate_boolean_values_are_compared_after_lexical_conversion() {
    let source = style(
        r#"<xsl:output omit-xml-declaration="yes"/><xsl:output omit-xml-declaration="true"/><xsl:output omit-xml-declaration="1"/>"#,
        "<r/>",
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<s/>")
            .unwrap(),
        "<r></r>"
    );
}

#[test]
fn conflicting_properties_at_the_winning_precedence_are_static_errors() {
    for declarations in [
        r#"<xsl:output method="text"/><xsl:output method="xml"/>"#,
        r#"<xsl:output indent="yes"/><xsl:output indent="no"/>"#,
        r#"<xsl:output standalone="yes"/><xsl:output standalone="omit"/>"#,
    ] {
        assert_eq!(
            Stylesheet::compile(&style(declarations, "<r/>"))
                .unwrap_err()
                .code,
            "XTSE1560"
        );
    }
}

#[test]
fn importing_modules_override_each_property_independently() {
    let imported = style(
        r#"<xsl:output method="text" encoding="ISO-8859-1"/>"#,
        "ignored",
    );
    let source = style(
        r#"<xsl:import href="base"/><xsl:output method="xml"/>"#,
        "<r/>",
    );
    let sheet = Stylesheet::compile_with_resolver(&source, &|_| Ok(imported.clone())).unwrap();
    assert_eq!(
        sheet.transform("<s/>").unwrap(),
        "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><r></r>"
    );
}

#[test]
fn a_higher_precedence_value_resolves_lower_precedence_conflicts() {
    let imported = style(
        r#"<xsl:output method="text"/><xsl:output method="html"/>"#,
        "ignored",
    );
    let source = style(
        r#"<xsl:import href="base"/><xsl:output method="xml" omit-xml-declaration="yes"/>"#,
        "<r/>",
    );
    let sheet = Stylesheet::compile_with_resolver(&source, &|_| Ok(imported.clone())).unwrap();
    assert_eq!(sheet.transform("<s/>").unwrap(), "<r></r>");
}

#[test]
fn included_modules_merge_at_the_including_precedence() {
    let included = style(r#"<xsl:output method="text"/>"#, "ignored");
    let source = style(
        r#"<xsl:include href="base"/><xsl:output encoding="UTF-8"/>"#,
        "text",
    );
    assert_eq!(
        Stylesheet::compile_with_resolver(&source, &|_| Ok(included.clone()))
            .unwrap()
            .transform("<s/>")
            .unwrap(),
        "text"
    );
    let source = style(
        r#"<xsl:include href="base"/><xsl:output method="xml"/>"#,
        "text",
    );
    assert_eq!(
        Stylesheet::compile_with_resolver(&source, &|_| Ok(included.clone()))
            .unwrap_err()
            .code,
        "XTSE1560"
    );
}

#[test]
fn cdata_names_accumulate_and_use_the_stylesheet_xml_default_namespace() {
    let source = style(r#"<xsl:output omit-xml-declaration="yes" cdata-section-elements="a"/><xsl:output cdata-section-elements="b"/>"#, "<r><a>&lt;a&gt;</a><b>&lt;b&gt;</b></r>")
        .replace("version=\"3.0\"", "version=\"3.0\" xmlns=\"urn:literal\" xpath-default-namespace=\"urn:source\"");
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<s/>")
            .unwrap(),
        "<r xmlns=\"urn:literal\"><a><![CDATA[<a>]]></a><b><![CDATA[<b>]]></b></r>"
    );
}
