use rust_xslt::Stylesheet;

#[test]
fn generates_an_executable_stylesheet_using_the_default_namespace() {
    let source = r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:gen="urn:generator" xmlns="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:namespace-alias stylesheet-prefix="gen" result-prefix="#default"/><xsl:template match="/"><gen:stylesheet version="3.0"><gen:output method="text"/><gen:template match="/"><gen:value-of select="/root/value"/></gen:template></gen:stylesheet></xsl:template></xsl:stylesheet>"##;
    let generated = Stylesheet::compile(source)
        .unwrap()
        .transform("<ignored/>")
        .unwrap();
    let result = Stylesheet::compile(&generated)
        .unwrap()
        .transform("<root><value>works</value></root>")
        .unwrap();
    assert_eq!(result, "works");
}

#[test]
fn aliases_literal_names_and_preserves_target_namespace_bindings() {
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:source" xmlns:b="urn:target" exclude-result-prefixes="a b" version="3.0"><xsl:namespace-alias stylesheet-prefix="a" result-prefix="b"/><xsl:template match="/"><a:result a:attr="v" test="b:some-name"/></xsl:template></xsl:stylesheet>"#;
    let output = Stylesheet::compile(source)
        .unwrap()
        .transform("<r/>")
        .unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    let root = doc.root_element();
    assert_eq!(root.tag_name().namespace(), Some("urn:target"));
    assert_eq!(root.attribute(("urn:target", "attr")), Some("v"));
    assert_eq!(root.lookup_namespace_uri(Some("b")), Some("urn:target"));
    assert_ne!(root.lookup_namespace_uri(Some("a")), Some("urn:source"));
}

#[test]
fn alias_conflicts_obey_import_precedence() {
    let imported = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" xmlns:b="urn:b" xmlns:c="urn:c" version="3.0"><xsl:namespace-alias stylesheet-prefix="a" result-prefix="b"/><xsl:namespace-alias stylesheet-prefix="a" result-prefix="c"/><xsl:template match="/"><a:item/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(Stylesheet::compile(imported).unwrap_err().code, "XTSE0810");
    let outer = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" xmlns:d="urn:d" version="3.0"><xsl:import href="inner.xsl"/><xsl:namespace-alias stylesheet-prefix="a" result-prefix="d"/></xsl:stylesheet>"#;
    let sheet = Stylesheet::compile_with_resolver(outer, &|_| Ok(imported.to_owned())).unwrap();
    let output = sheet.transform("<r/>").unwrap();
    assert_eq!(
        roxmltree::Document::parse(&output)
            .unwrap()
            .root_element()
            .tag_name()
            .namespace(),
        Some("urn:d")
    );
}

#[test]
fn named_templates_can_declare_private_visibility() {
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output method="text"/><xsl:template match="/"><xsl:call-template name="inner"/></xsl:template><xsl:template name="inner" visibility="private">ok</xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "ok"
    );
}

#[test]
fn aliasing_applies_to_literal_attributes_after_attribute_sets() {
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" xmlns:b="urn:b" version="3.0"><xsl:namespace-alias stylesheet-prefix="a" result-prefix="b"/><xsl:attribute-set name="s"><xsl:attribute name="other">set</xsl:attribute></xsl:attribute-set><xsl:template match="/"><a:item xsl:use-attribute-sets="s" a:attr="literal"/></xsl:template></xsl:stylesheet>"#;
    let output = Stylesheet::compile(source)
        .unwrap()
        .transform("<r/>")
        .unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    let root = doc.root_element();
    assert_eq!(root.tag_name().namespace(), Some("urn:b"));
    assert_eq!(root.attribute(("urn:b", "attr")), Some("literal"));
    assert_eq!(root.attribute("other"), Some("set"));
}

#[test]
fn aliases_null_namespace_and_reports_unbound_prefixes() {
    let source = r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:out="urn:result" version="3.0"><xsl:namespace-alias stylesheet-prefix="#default" result-prefix="out"/><xsl:template match="/"><item attribute="value"/></xsl:template></xsl:stylesheet>"##;
    let output = Stylesheet::compile(source)
        .unwrap()
        .transform("<r/>")
        .unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert_eq!(
        doc.root_element().tag_name().namespace(),
        Some("urn:result")
    );
    assert_eq!(
        doc.root_element().attribute(("urn:result", "attribute")),
        Some("value")
    );
    let bad = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:namespace-alias stylesheet-prefix="missing" result-prefix="xsl"/></xsl:stylesheet>"#;
    assert_eq!(Stylesheet::compile(bad).unwrap_err().code, "XTSE0812");
}
