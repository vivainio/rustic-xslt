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
fn untyped_constructor_delegation_atomizes_for_string_results() {
    let s = sheet(
        r#"<xsl:template match="/" as="xs:string"><xsl:call-template name="constructor"/></xsl:template><xsl:template name="constructor"><n>text</n></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "text");
}

#[test]
fn mixed_constructed_nodes_keep_kinds_and_boundaries() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="item()*"><xsl:sequence select="7"/><n>e</n><xsl:text>a</xsl:text><xsl:text>b</xsl:text><xsl:comment>c</xsl:comment><xsl:processing-instruction name="p">d</xsl:processing-instruction><xsl:document><doc/></xsl:document><xsl:sequence select="true()"/></xsl:variable><xsl:value-of select="concat(count($v), ':', $v[2] instance of element(n), ':', $v[3] instance of text(), ':', $v[4] instance of text(), ':', $v[5] instance of comment(), ':', $v[6] instance of processing-instruction(), ':', $v[7] instance of document-node(), ':', empty($v[2]/parent::node()), ':', string-join($v[3], '|'))"/></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r/>").unwrap(),
        "8:true:true:true:true:true:true:true:a"
    );
}
#[test]
fn nested_element_contents_are_grounded_within_raw_sequences() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="item()*"><xsl:sequence select="1"/><out><xsl:sequence select="(2,3)"/></out><xsl:sequence select="4"/></xsl:variable><xsl:value-of select="concat(count($v), ':', string($v[2]))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "3:2 3");
}
#[test]
fn adjacent_constructed_text_nodes_atomize_separately() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="xs:string+"><xsl:text>a</xsl:text><xsl:value-of select="'b'"/></xsl:variable><xsl:value-of select="concat(count($v), ':', string-join($v, '|'))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "2:a|b");
}

#[test]
fn raw_copy_of_preserves_types_but_allocates_fresh_node_identity() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="item()*"><xsl:copy-of select="(7, r/a, r/a, /)"/></xsl:variable><xsl:value-of select="concat(count($v), ':', $v[1] instance of xs:integer, ':', $v[2] is r/a, ':', $v[2] is $v[3], ':', deep-equal($v[2], r/a), ':', empty($v[2]/parent::node()), ':', $v[4] instance of document-node())"/></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r><a>text</a></r>").unwrap(),
        "4:true:false:false:true:true:true"
    );
}

#[test]
fn raw_shallow_copy_grounds_only_its_constructor_body() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="item()*"><xsl:sequence select="1"/><xsl:copy select="r"><xsl:attribute name="a">v</xsl:attribute><child/></xsl:copy><xsl:copy select="/"/><xsl:copy select="true()"/></xsl:variable><xsl:value-of select="concat(count($v), ':', name($v[2]), ':', string($v[2]/@a), ':', count($v[2]/child), ':', $v[3] instance of document-node(), ':', $v[4] instance of xs:boolean)"/></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r original='x'><discard/></r>").unwrap(),
        "4:r:v:1:true:true"
    );
}

#[test]
fn raw_apply_templates_collects_explicit_and_builtin_results() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="item()*"><xsl:apply-templates select="r/*"/></xsl:variable><xsl:value-of select="concat(count($v), ':', $v[1] instance of xs:integer, ':', $v[2] instance of text(), ':', string($v[2]))"/></xsl:template><xsl:template match="a"><xsl:sequence select="7"/></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r><a/><b>text</b></r>").unwrap(),
        "2:true:true:text"
    );
}
#[test]
fn raw_builtin_copy_policies_keep_node_kinds() {
    for policy in ["shallow-copy", "deep-copy"] {
        let s = sheet(&format!(
            r#"<xsl:mode name="m" on-no-match="{policy}"/><xsl:template match="/"><xsl:variable name="v" as="item()*"><xsl:apply-templates select="r" mode="m"/></xsl:variable><xsl:value-of select="concat(count($v), ':', $v[1] instance of element(r), ':', empty($v[1]/parent::node()), ':', string($v[1]/@a), ':', count($v[1]/n))"/></xsl:template>"#
        ));
        assert_eq!(s.transform("<r a='v'><n/></r>").unwrap(), "1:true:true:v:1");
    }
}

#[test]
fn raw_constructed_attributes_are_parentless_and_keep_names_and_values() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="attribute(Q{urn:a}v)"><xsl:attribute name="p:v" namespace="urn:a" separator="|"><xsl:sequence select="('a', 'b')"/></xsl:attribute></xsl:variable><xsl:value-of select="concat($v instance of attribute(Q{urn:a}v), ':', string($v), ':', root($v) is $v, ':', empty($v/ancestor::node()), ':', empty($v/following::node()), ':', empty($v/preceding::node()))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "true:a|b:true:true:true:true");
}
#[test]
fn raw_attribute_copies_have_fresh_identities_and_attach_to_elements() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="attribute()+"><xsl:copy-of select="(r/@a, r/@a)"/><xsl:copy select="r/@a"/></xsl:variable><xsl:variable name="e" as="element()"><out><xsl:sequence select="$v[1]"/></out></xsl:variable><xsl:value-of select="concat(count($v), ':', $v[1] is $v[2], ':', $v[1] is r/@a, ':', root($v[3]) is $v[3], ':', string($e/@a), ':', $e/@a/parent::node() is $e)"/></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r a='v'/>").unwrap(),
        "3:false:false:true:v:true"
    );
}
#[test]
fn raw_attribute_builtin_copy_and_typed_template_results() {
    for policy in ["shallow-copy", "deep-copy"] {
        let s = sheet(&format!(
            r#"<xsl:mode name="m" on-no-match="{policy}"/><xsl:template name="a" as="attribute(v)"><xsl:attribute name="v" select="'constructed'"/></xsl:template><xsl:template match="/"><xsl:variable name="v" as="attribute()+"><xsl:apply-templates select="r/@a" mode="m"/><xsl:call-template name="a"/></xsl:variable><xsl:value-of select="concat(count($v), ':', string($v[1]), ':', string($v[2]), ':', empty($v/parent::node()))"/></xsl:template>"#
        ));
        assert_eq!(
            s.transform("<r a='copied'/>").unwrap(),
            "2:copied:constructed:true"
        );
    }
}
#[test]
fn constructed_attributes_enforce_required_types() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="attribute(other)"><xsl:attribute name="v">value</xsl:attribute></xsl:variable><xsl:sequence select="$v"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0570");
}

#[test]
fn empty_constructed_text_nodes_preserve_identity_and_navigation() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="text()+"><xsl:value-of select="''"/><xsl:value-of select="()"/><xsl:text/></xsl:variable><xsl:value-of select="concat(count($v), ':', string-length($v[1]), ':', string-length($v[2]), ':', string-length($v[3]), ':', $v[1] is $v[2], ':', root($v[1]) is $v[1], ':', empty($v/parent::node()), ':', empty($v/following::node()))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "3:0:0:0:false:true:true:true");
}
#[test]
fn empty_text_copies_stay_empty_and_element_content_discards_them() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:variable name="v" as="text()"><xsl:value-of select="''"/></xsl:variable><xsl:variable name="c" as="text()+"><xsl:copy-of select="($v, $v)"/><xsl:copy select="$v"/></xsl:variable><xsl:variable name="e" as="element()"><out><xsl:sequence select="$v"/></out></xsl:variable><xsl:value-of select="concat(count($c), ':', string-length(string-join($c, '')), ':', $c[1] is $c[2], ':', $c[1] is $v, ':', count($e/node()))"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "3:0:false:false:0");
}
