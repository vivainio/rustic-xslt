use rust_xslt::{Limits, StreamOptions, Stylesheet, transform};
use std::io::Cursor;
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" exclude-result-prefixes="xs" version="3.0"><xsl:output method="text"/>{body}</xsl:stylesheet>"##
    )
}
#[test]
fn temporary_document_wrapper_is_invisible_to_xpath() {
    let s = sheet(
        r##"<xsl:variable name="tree"><a><b>text</b></a><c/></xsl:variable><xsl:template match="/"><xsl:value-of select="concat($tree instance of document-node(), ':', count($tree/*), ':', name($tree), ':', $tree/a/.. is $tree, ':', count($tree/_rust_xslt_document), ':', count($tree/a/ancestor::node()))"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<source/>").unwrap(), "true:2::true:0:1");
}
#[test]
fn distinct_documents_have_distinct_identity_and_consistent_order() {
    let s = sheet(
        r##"<xsl:variable name="one"><a/></xsl:variable><xsl:variable name="two"><a/></xsl:variable><xsl:template match="/"><xsl:value-of select="concat($one is $one, ':', $one is $two, ':', count($one/a | $two/a), ':', ($one &lt;&lt; $two) != ($two &lt;&lt; $one))"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "true:false:2:true");
}
#[test]
fn typed_constructor_returns_children_and_untyped_constructor_returns_document() {
    let s = sheet(
        r##"<xsl:variable name="element" as="element()"><a>value</a></xsl:variable><xsl:variable name="text" as="xs:string"><a>atomized</a></xsl:variable><xsl:template match="/"><xsl:value-of select="concat($element instance of element(), ':', name($element), ':', $text instance of xs:string, ':', $text)"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "true:a:true:atomized");
}
#[test]
fn local_constructor_scope_and_global_forward_dependencies() {
    let s = sheet(
        r##"<xsl:variable name="tree"><a value="{$later}"/></xsl:variable><xsl:variable name="later" select="'global'"/><xsl:template match="/"><xsl:variable name="local"><xsl:variable name="later" select="'local'"/><b value="{$later}"/></xsl:variable><xsl:value-of select="concat($tree/a/@value, ':', $local/b/@value, ':', $later)"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "global:local:global");
}
#[test]
fn constructed_default_and_argument_parameters_keep_caller_scope() {
    let s = sheet(
        r##"<xsl:variable name="label" select="'global'"/><xsl:template match="/"><xsl:variable name="label" select="'caller'"/><xsl:call-template name="target"><xsl:with-param name="arg"><arg><xsl:value-of select="$label"/></arg></xsl:with-param></xsl:call-template></xsl:template><xsl:template name="target"><xsl:param name="arg"/><xsl:param name="default"><default><xsl:value-of select="$label"/></default></xsl:param><xsl:value-of select="concat($arg/arg, ':', $default/default)"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "caller:global");
}
#[test]
fn untyped_sequence_constructor_wraps_values_in_a_document() {
    let s = sheet(
        r##"<xsl:variable name="tree"><xsl:sequence select="(1,2,3)"/></xsl:variable><xsl:variable name="number" as="xs:integer"><xsl:sequence select="12"/></xsl:variable><xsl:template match="/"><xsl:value-of select="concat($tree instance of document-node(), ':', $tree, ':', $number instance of xs:integer, ':', $number + 1)"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "true:1 2 3:true:13");
}
#[test]
fn temporary_document_copy_has_no_wrapper_or_extra_namespace() {
    let s=sheet(r##"<xsl:variable name="tree"><a xmlns:p="urn:p"><p:b/></a><c/></xsl:variable><xsl:template match="/"><out><xsl:copy-of select="$tree"/></out></xsl:template>"##).replace("method=\"text\"","method=\"xml\" omit-xml-declaration=\"yes\"");
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        "<out><a xmlns:p=\"urn:p\"><p:b></p:b></a><c></c></out>"
    );
}
#[test]
fn source_whitespace_rules_do_not_strip_constructed_whitespace() {
    let s = sheet(
        r##"<xsl:strip-space elements="*"/><xsl:variable name="tree"><a><xsl:text> </xsl:text></a></xsl:variable><xsl:template match="/"><xsl:value-of select="concat(count(r/text()), ':', string-length($tree/a))"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r> </r>").unwrap(), "0:1");
}
#[test]
fn temporary_tree_budget_bounds_retained_sources() {
    let s=Stylesheet::compile(&sheet(r##"<xsl:template match="/"><xsl:for-each select="r/n"><xsl:variable name="tree"><a>content</a></xsl:variable></xsl:for-each></xsl:template>"##)).unwrap();
    let source = format!("<r>{}</r>", "<n/>".repeat(100));
    assert_eq!(
        s.transform_with_limits(
            &source,
            Limits {
                max_temporary_bytes: 100,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0130"
    );
}
#[test]
fn streamed_temporary_trees_are_released_between_records() {
    let s=Stylesheet::compile(&sheet(r##"<xsl:template name="record"><xsl:variable name="tree"><a><xsl:value-of select="@id"/></a></xsl:variable><xsl:value-of select="$tree/a"/></xsl:template>"##)).unwrap();
    let source = format!("<records>{}</records>", "<record id='x'/>".repeat(1000));
    let options = StreamOptions {
        limits: Limits {
            max_temporary_bytes: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut out = vec![];
    let stats = s
        .stream_records(Cursor::new(source), &mut out, &options)
        .unwrap();
    assert_eq!(stats.records, 1000);
    assert_eq!(out, b"x".repeat(1000));
}
#[test]
fn streamed_globals_with_context_dependent_constructors_are_rejected() {
    let s=Stylesheet::compile(&sheet(r##"<xsl:variable name="tree"><a><xsl:value-of select="/records/@secret"/></a></xsl:variable><xsl:template name="record"><xsl:value-of select="$tree"/></xsl:template>"##)).unwrap();
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
}
#[test]
fn temporary_document_attributes_have_the_document_error_code() {
    let s = sheet(
        r##"<xsl:variable name="tree"><xsl:attribute name="bad">value</xsl:attribute></xsl:variable><xsl:template match="/"/>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XTDE0420");
}

#[test]
fn comment_and_pi_atomization_has_string_type() {
    let s = sheet(
        r##"<xsl:variable name="tree"><a><xsl:comment>12</xsl:comment><xsl:processing-instruction name="pi">34</xsl:processing-instruction></a></xsl:variable><xsl:template match="/"><xsl:value-of select="concat(data($tree/a/comment()) instance of xs:string, ':', data($tree/a/processing-instruction()) instance of xs:string)"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "true:true");
}
