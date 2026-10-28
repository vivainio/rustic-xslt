use rust_xslt::{StreamOptions, Stylesheet};
use std::io::Cursor;
fn sheet(body: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/>{body}</xsl:stylesheet>"#)).unwrap()
}
#[test]
fn constructed_typed_roots_hide_their_backing_document_and_siblings() {
    let s = sheet(
        r#"<xsl:variable name="v" as="element()+"><a><b/></a><c/></xsl:variable><xsl:template match="/"><xsl:value-of select="concat(empty($v[1]/..), ':', root($v[1]) is $v[1], ':', $v[1]/b/.. is $v[1], ':', count($v[1]/b/ancestor::node()), ':', count($v[1]/following::node()), ':', count($v[2]/preceding-sibling::*))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "true:true:true:1:0:0");
}
#[test]
fn constructed_template_results_retain_parentless_boundaries_in_raw_bindings() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="element()"><xsl:call-template name="node"/></xsl:variable><xsl:value-of select="root($v) is $v and empty($v/..)"/></xsl:template><xsl:template name="node" as="element()"><n/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "true");
}
#[test]
fn parentless_text_nodes_keep_their_own_root_and_match_text_rules() {
    let s = sheet(
        r#"<xsl:variable name="v" as="text()"><xsl:text>text</xsl:text></xsl:variable><xsl:template match="/"><xsl:value-of select="root($v) is $v and empty($v/..)"/><xsl:apply-templates select="$v"/></xsl:template><xsl:template match="text()">:matched</xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "true:matched");
}
#[test]
fn parentless_name_patterns_use_singleton_predicate_focus() {
    let s = sheet(
        r#"<xsl:variable name="v" as="element()"><a><b/></a></xsl:variable><xsl:template match="/"><xsl:apply-templates select="$v"/></xsl:template><xsl:template match="a[1]">one:<xsl:apply-templates/></xsl:template><xsl:template match="a[2]" priority="2">wrong</xsl:template><xsl:template match="/a" priority="3">wrong</xsl:template><xsl:template match="a/b">child</xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "one:child");
}
#[test]
fn root_function_and_absolute_paths_have_different_requirements() {
    let s = sheet(
        r#"<xsl:variable name="v" as="element()"><n/></xsl:variable><xsl:template match="/"><xsl:for-each select="$v"><xsl:sequence select="/"/></xsl:for-each></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XPDY0050");
    let s = sheet(
        r#"<xsl:template match="/"><xsl:value-of select="empty(root(())) and root(r/@v) is /"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r v='1'/>").unwrap(), "true");
}
#[test]
fn document_node_patterns_match_actual_document_roots() {
    let s = sheet(r#"<xsl:template match="document-node()">document</xsl:template>"#);
    assert_eq!(s.transform("<r/>").unwrap(), "document");
}
#[test]
fn record_streaming_rejects_root_navigation_before_consuming_input() {
    let s = sheet(r#"<xsl:template name="record"><xsl:sequence select="root()"/></xsl:template>"#);
    let mut output = vec![];
    assert_eq!(
        s.stream_records(
            Cursor::new("not XML"),
            &mut output,
            &StreamOptions::default()
        )
        .unwrap_err()
        .code,
        "RUST_XSLT_STREAM_UNSUPPORTED"
    );
    assert!(output.is_empty());
}
