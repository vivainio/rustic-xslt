use rust_xslt::{Limits, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn sheet(body: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/>{body}</xsl:stylesheet>"#)).unwrap()
}
#[test]
fn typed_global_and_local_sequences_preserve_atomic_types_and_boundaries() {
    let s = sheet(
        r#"<xsl:variable name="g" as="item()*"><xsl:sequence select="1"/><xsl:sequence select="true()"/></xsl:variable><xsl:template match="/"><xsl:variable name="v" as="xs:string+"><xsl:sequence select="'a'"/><xsl:sequence select="'b'"/></xsl:variable><xsl:value-of select="concat($g[1] instance of xs:integer, ':', $g[2] instance of xs:boolean, ':', count($v), ':', string-join($v, '|'))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "true:true:2:a|b");
}
#[test]
fn multiple_raw_values_apply_binding_and_parameter_error_codes() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="xs:string"><xsl:sequence select="'a'"/><xsl:sequence select="'b'"/></xsl:variable></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0570");
    let s = sheet(
        r#"<xsl:template match="/"><xsl:param name="v" as="xs:string"><xsl:sequence select="'a'"/><xsl:sequence select="'b'"/></xsl:param></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0600");
}
#[test]
fn typed_template_delegation_does_not_stringify_before_conversion() {
    let s = sheet(
        r#"<xsl:template match="/" as="xs:string"><xsl:call-template name="private"/></xsl:template><xsl:template name="private"><xsl:sequence select="'text'"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "text");
    let s = sheet(
        r#"<xsl:template match="/" as="xs:string"><xsl:call-template name="private"/></xsl:template><xsl:template name="private"><xsl:sequence select="true()"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0505");
}
#[test]
fn raw_sequences_retain_selected_node_and_attribute_identity() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="nodes" as="node()+"><xsl:sequence select="r/a"/><xsl:sequence select="r/b"/></xsl:variable><xsl:variable name="attrs" as="attribute()+"><xsl:sequence select="r/@v"/><xsl:sequence select="r/@w"/></xsl:variable><xsl:value-of select="concat($nodes[1] is r/a, ':', $attrs[1] is r/@v, ':', count($nodes), ':', count($attrs))"/></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r v='1' w='2'><a/><b/></r>").unwrap(),
        "true:true:2:2"
    );
}
#[test]
fn raw_capture_restores_nested_raw_and_document_constructors() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="outer" as="xs:integer+"><xsl:variable name="tree"><n>7</n></xsl:variable><xsl:variable name="inner" as="xs:integer+"><xsl:sequence select="1"/><xsl:sequence select="2"/></xsl:variable><xsl:sequence select="$inner"/><xsl:sequence select="xs:integer($tree/n)"/></xsl:variable><xsl:value-of select="concat(count($outer), ':', sum($outer))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "3:10");
}
#[test]
fn typed_constructor_template_results_can_flow_into_raw_bindings() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="element()+"><xsl:call-template name="node"/><xsl:call-template name="node"/></xsl:variable><xsl:value-of select="concat(count($v), ':', deep-equal($v[1], $v[2]), ':', $v[1] is $v[2])"/></xsl:template><xsl:template name="node" as="element(n)"><n/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "2:true:false");
}
#[test]
fn template_control_flow_and_next_match_return_typed_values() {
    let s = sheet(
        r#"<xsl:template match="r" priority="1" as="xs:integer"><xsl:next-match/></xsl:template><xsl:template match="r"><xsl:sequence select="7"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "7");
    let s = sheet(
        r#"<xsl:template match="/" as="xs:integer+"><xsl:for-each select="1 to 3"><xsl:if test=". != 2"><xsl:sequence select="."/></xsl:if></xsl:for-each></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "1 3");
}
#[test]
fn raw_atomic_sequences_need_no_temporary_tree_source_bytes() {
    let s = sheet(
        r#"<xsl:variable name="v" as="xs:integer+"><xsl:sequence select="1"/><xsl:sequence select="2"/></xsl:variable><xsl:template match="/" as="xs:integer+"><xsl:sequence select="$v"/><xsl:sequence select="3"/></xsl:template>"#,
    );
    let mut opts = TransformOptions {
        limits: Limits {
            max_temporary_bytes: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), "1 2 3");
    opts.limits.max_output_bytes = 1;
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap_err().code,
        "XPDY0130"
    );
}
#[test]
fn raw_record_results_do_not_accumulate_across_records() {
    let s = sheet(
        r#"<xsl:template name="record" as="xs:integer+"><xsl:sequence select="position()"/><xsl:sequence select="position()+1"/></xsl:template>"#,
    );
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<r><n/><n/></r>"),
        &mut output,
        &StreamOptions {
            record_path: "/r/n".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output, b"1 22 3");
}
#[test]
fn unsupported_untyped_constructor_delegation_is_explicit() {
    let s = sheet(
        r#"<xsl:template match="/" as="xs:string"><xsl:call-template name="constructor"/></xsl:template><xsl:template name="constructor"><n>text</n></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r/>").unwrap_err().code,
        "RUST_XSLT_UNSUPPORTED"
    );
}
