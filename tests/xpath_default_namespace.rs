use rust_xslt::Stylesheet;

fn style(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" xpath-default-namespace="urn:source"><xsl:output method="text"/>{body}</xsl:stylesheet>"#
    )
}

#[test]
fn default_xpath_namespace_applies_to_elements_but_not_attributes_or_variables() {
    let source = style(
        r#"<xsl:variable name="v" select="'global'"/><xsl:template match="/"><xsl:value-of select="concat($v, ':', r/n/@id, ':', count(r/attribute::id), ':', count(r/element(n)), ':', count(r/attribute::attribute(id)))"/></xsl:template>"#,
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r xmlns='urn:source' id='root'><n id='child'/></r>")
            .unwrap(),
        "global:child:1:1:1"
    );
}

#[test]
fn inherited_namespace_settings_allow_local_empty_resets_and_explicit_eqnames() {
    let source = style(
        r#"<xsl:template match="/"><xsl:value-of select="r/n"/><xsl:for-each select="r"><xsl:value-of xpath-default-namespace="" select="n"/><xsl:value-of select="Q{}n"/></xsl:for-each></xsl:template>"#,
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r xmlns='urn:source'><n>A</n><n xmlns=''>B</n></r>")
            .unwrap(),
        "ABB"
    );
}

#[test]
fn named_sequence_types_and_casts_use_the_default_element_type_namespace() {
    let source = style(
        r#"<xsl:template match="/"><xsl:variable name="r" as="element(r)" select="r"/><xsl:variable name="id" as="attribute(id)" select="$r/@id"/><xsl:value-of select="concat(. instance of document-node(element(r)), ':', $r instance of element(r), ':', $id instance of attribute(id))"/><xsl:value-of xpath-default-namespace="http://www.w3.org/2001/XMLSchema" select="'7' cast as integer"/></xsl:template>"#,
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r xmlns='urn:source' id='ok'/>")
            .unwrap(),
        "true:true:true7"
    );
}

#[test]
fn default_namespace_affects_patterns_key_use_and_whitespace_name_tests() {
    let source = style(
        r#"<xsl:strip-space elements="r"/><xsl:key name="k" match="n" use="v"/><xsl:template match="/"><xsl:value-of select="count(r/text())"/><xsl:apply-templates select="key('k', 'x')"/></xsl:template><xsl:template match="n"><xsl:value-of select="@id"/></xsl:template>"#,
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r xmlns='urn:source'> <n id='a'><v>x</v></n> </r>")
            .unwrap(),
        "0a"
    );
}

#[test]
fn literal_result_settings_scope_avts_without_changing_literal_names() {
    let source = style(r#"<xsl:template match="/"><out xsl:xpath-default-namespace="" value="{Q{urn:source}r/n}"><xsl:value-of select="Q{urn:source}r/n"/></out></xsl:template>"#).replace("method=\"text\"", "omit-xml-declaration=\"yes\"");
    let output = Stylesheet::compile(&source)
        .unwrap()
        .transform("<r xmlns='urn:source'><n xmlns=''>B</n><n>A</n></r>")
        .unwrap();
    assert!(output.starts_with("<out "));
    assert!(output.contains("value=\"B\""));
    assert!(output.ends_with(">B</out>"));
    assert!(!output.contains("xpath-default-namespace"));
    assert!(!output.contains("urn:source"));
}

#[test]
fn qname_casts_use_the_xpath_default_namespace() {
    let source = style(
        r#"<xsl:template match="/"><xsl:value-of select="xs:QName('n') eq QName('urn:source', 'n')"/></xsl:template>"#,
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "true"
    );
}

#[test]
fn xslt_elements_require_the_unprefixed_standard_attribute() {
    let source = style(
        r#"<xsl:template match="/"><xsl:value-of xsl:xpath-default-namespace="urn:a" select="r"/></xsl:template>"#,
    );
    assert_eq!(Stylesheet::compile(&source).unwrap_err().code, "XTSE0090");
}
