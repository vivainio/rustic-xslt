use rust_xslt::{StreamOptions, Stylesheet, TransformOptions, transform};
use std::io::Cursor;
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" exclude-result-prefixes="xs" version="3.0"><xsl:output omit-xml-declaration="yes"/>{body}</xsl:stylesheet>"##
    )
}
#[test]
fn copy_select_replaces_focus_and_ignores_body_for_atomic_or_empty_items() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:for-each select="r/n"><xsl:copy select="../chosen"><xsl:attribute name="focus" select="concat(position(),'/',last())"/></xsl:copy></xsl:for-each><xsl:copy select="()"><xsl:value-of select="1 div 0"/></xsl:copy><xsl:copy select="12"><xsl:value-of select="1 div 0"/></xsl:copy></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r><n/><n/><chosen/></r>").unwrap(),
        "<out><chosen focus=\"1/1\"></chosen><chosen focus=\"1/1\"></chosen>12</out>"
    );
}
#[test]
fn copy_select_requires_at_most_one_item_and_implicit_copy_requires_focus() {
    let s = sheet(r##"<xsl:template match="/"><xsl:copy select="r/n"/></xsl:template>"##);
    assert_eq!(
        transform(&s, "<r><n/><n/></r>").unwrap_err().code,
        "XTTE3180"
    );
    let s = Stylesheet::compile(&sheet(
        r##"<xsl:template name="main"><xsl:copy/></xsl:template>"##,
    ))
    .unwrap();
    assert_eq!(
        s.transform_with_options(
            "",
            &TransformOptions {
                initial_template: Some("main".into()),
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0002"
    );
}
#[test]
fn namespace_copy_no_prunes_unused_bindings_but_retains_names() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:copy-of select="r/p:n" copy-namespaces="no" xmlns:p="urn:p"/></out></xsl:template>"##,
    );
    let output=transform(&s,"<r xmlns:p='urn:p' xmlns:q='urn:q' xmlns:unused='urn:unused'><p:n q:attr='yes'><child/></p:n></r>").unwrap();
    assert!(!output.contains("urn:unused"), "{output}");
    let doc = roxmltree::Document::parse(&output).unwrap();
    let n = doc.root_element().first_element_child().unwrap();
    assert_eq!(n.tag_name().namespace(), Some("urn:p"));
    assert_eq!(n.attribute(("urn:q", "attr")), Some("yes"));
}
#[test]
fn shallow_copy_namespace_no_keeps_bindings_for_body_supplied_attributes() {
    let s = sheet(
        r##"<xsl:template match="/r"><xsl:copy copy-namespaces="no"><xsl:copy-of select="@*"/></xsl:copy></xsl:template>"##,
    );
    let output = transform(
        &s,
        "<r xmlns:p='urn:p' xmlns:unused='urn:unused' p:attr='v'/>",
    )
    .unwrap();
    assert_eq!(output, "<r xmlns:p=\"urn:p\" p:attr=\"v\"></r>");
}
#[test]
fn document_constructors_return_document_nodes_when_typed() {
    let s = sheet(
        r##"<xsl:variable name="tree" as="document-node()"><xsl:document><a>text</a><b/></xsl:document></xsl:variable><xsl:template match="/"><out type="{$tree instance of document-node()}" count="{count($tree/*)}"><xsl:copy-of select="$tree"/></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        "<out type=\"true\" count=\"2\"><a>text</a><b></b></out>"
    );
}
#[test]
fn nested_document_content_flattens_and_document_attributes_are_errors() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:document><a/><xsl:document><b/></xsl:document></xsl:document></out></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "<out><a></a><b></b></out>");
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:document><xsl:attribute name="bad">value</xsl:attribute></xsl:document></out></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XTDE0420");
}
#[test]
fn namespace_boolean_controls_are_checked_and_stream_copy_selection_cannot_escape() {
    assert_eq!(
        Stylesheet::compile(&sheet(
            r##"<xsl:template match="/"><xsl:copy copy-namespaces="maybe"/></xsl:template>"##
        ))
        .unwrap_err()
        .code,
        "XTSE0020"
    );
    let s = Stylesheet::compile(&sheet(
        r##"<xsl:template name="record"><xsl:copy select=".."/></xsl:template>"##,
    ))
    .unwrap();
    assert_eq!(
        s.stream_records(
            Cursor::new("<records/>"),
            &mut vec![],
            &StreamOptions::default()
        )
        .unwrap_err()
        .code,
        "RUST_XSLT_STREAM_UNSUPPORTED"
    );
    let s=Stylesheet::compile(&sheet(r##"<xsl:template name="record"><xsl:document><xsl:copy select="." copy-namespaces="no"><xsl:value-of select="@id"/></xsl:copy></xsl:document></xsl:template>"##)).unwrap();
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<records><record id='a'/><record id='b'/></records>"),
        &mut output,
        &StreamOptions::default(),
    )
    .unwrap();
    assert_eq!(output, b"<record>a</record><record>b</record>");
}
#[test]
fn inherit_namespace_no_does_not_change_required_child_name_binding() {
    let s = sheet(
        r##"<xsl:template match="/"><xsl:element name="outer" namespace="urn:parent" inherit-namespaces="no"><xsl:element name="child" namespace=""/></xsl:element></xsl:template>"##,
    );
    let output = transform(&s, "<r/>").unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert_eq!(
        doc.root_element().tag_name().namespace(),
        Some("urn:parent")
    );
    assert_eq!(
        doc.root_element()
            .first_element_child()
            .unwrap()
            .tag_name()
            .namespace()
            .unwrap_or(""),
        ""
    );
}

#[test]
fn default_namespace_undeclarations_match_unprefixed_xpath_names() {
    let s =
        sheet(r##"<xsl:template match="/"><xsl:value-of select="count(r/n)"/></xsl:template>"##)
            .replace("omit-xml-declaration=\"yes\"", "method=\"text\"");
    assert_eq!(transform(&s, "<r xmlns=''><n/></r>").unwrap(), "1");
    let s = Stylesheet::compile(&sheet(
        r##"<xsl:template name="record"><xsl:value-of select="@id"/></xsl:template>"##,
    ))
    .unwrap();
    let mut output = vec![];
    assert_eq!(
        s.stream_records(
            Cursor::new("<records xmlns=''><record id='yes'/></records>"),
            &mut output,
            &StreamOptions::default()
        )
        .unwrap()
        .records,
        1
    );
    assert_eq!(output, b"yes");
}
