use rust_xslt::{Limits, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:static"><xsl:output method="text"/><xsl:template match="/" name="record"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expr: &str, xml: &str) -> String {
    Stylesheet::compile(&source(expr))
        .unwrap()
        .transform(xml)
        .unwrap()
}
#[test]
fn node_names_retain_exact_element_and_attribute_prefixes() {
    let xml = "<p:r xmlns:p='urn:p' xmlns:q='urn:p' q:a='v'/>";
    assert_eq!(
        evaluate(
            "concat(string(node-name(/*)), ':', string(node-name(/*/@*)), ':', name(/*/@*))",
            xml
        ),
        "p:r:q:a:q:a"
    );
    assert_eq!(
        evaluate("node-name(/*/@*) eq QName('urn:p','other:a')", xml),
        "true"
    );
    assert_eq!(evaluate("node-name(/*) instance of xs:QName", xml), "true");
    assert_eq!(
        evaluate(
            "concat(empty(prefix-from-QName(node-name(/*))), ':', namespace-uri-from-QName(node-name(/*)))",
            "<r xmlns='urn:def'/>"
        ),
        "true:urn:def"
    );
}
#[test]
fn unnamed_nodes_have_empty_names_and_processing_instructions_have_qnames() {
    assert_eq!(
        evaluate(
            "empty(node-name(/)) and empty(node-name(r/text())) and empty(node-name(r/comment()))",
            "<r>text<!--comment--></r>"
        ),
        "true"
    );
    assert_eq!(
        evaluate(
            "node-name(r/processing-instruction()) eq QName('','go')",
            "<r><?go value?></r>"
        ),
        "true"
    );
    assert_eq!(evaluate("empty(node-name(()))", "<r/>"), "true");
}
#[test]
fn dynamic_qname_resolution_uses_element_namespaces_and_strips_outer_whitespace() {
    for (expr, xml, expected) in [
        (
            "resolve-QName(' p:n ',r) eq QName('urn:source','n')",
            "<r xmlns:p='urn:source'/>",
            "true",
        ),
        (
            "resolve-QName('n',/*) eq QName('urn:def','n')",
            "<r xmlns='urn:def'/>",
            "true",
        ),
        ("resolve-QName('n',r) eq QName('','n')", "<r/>", "true"),
        (
            "resolve-QName('xml:lang',r) eq QName('http://www.w3.org/XML/1998/namespace','lang')",
            "<r/>",
            "true",
        ),
        ("empty(resolve-QName((),r))", "<r/>", "true"),
        (
            "QName('  urn:p  ',' p:n ') eq QName('urn:p','n')",
            "<r/>",
            "true",
        ),
    ] {
        assert_eq!(evaluate(expr, xml), expected, "{expr}");
    }
}
#[test]
fn namespace_lookup_and_prefix_lists_respect_shadowing_and_undeclarations() {
    let xml = "<r xmlns='urn:def' xmlns:p='urn:p'><n xmlns='' xmlns:p='urn:inner'/></r>";
    assert_eq!(evaluate("namespace-uri-for-prefix((),/*)", xml), "urn:def");
    assert_eq!(
        evaluate("namespace-uri-for-prefix('p',/*/*)", xml),
        "urn:inner"
    );
    assert_eq!(
        evaluate("empty(namespace-uri-for-prefix((),/*/*))", xml),
        "true"
    );
    assert_eq!(
        evaluate("empty(namespace-uri-for-prefix('missing',/*))", xml),
        "true"
    );
    assert_eq!(
        evaluate("namespace-uri-for-prefix('xml',/*)", xml),
        "http://www.w3.org/XML/1998/namespace"
    );
    assert_eq!(
        evaluate(
            "namespace-uri-for-prefix('p',/*) instance of xs:anyURI",
            xml
        ),
        "true"
    );
    assert_eq!(
        evaluate(
            "every $p in ('','p','xml') satisfies $p = in-scope-prefixes(/*)",
            xml
        ),
        "true"
    );
    assert_eq!(evaluate("count(in-scope-prefixes(/*/*))", xml), "2");
}
#[test]
fn namespace_functions_enforce_types_and_qname_error_codes() {
    for (expr, code) in [
        ("node-name(1)", "XPTY0004"),
        ("resolve-QName('p:',r)", "FOCA0002"),
        ("resolve-QName('z:n',r)", "FONS0004"),
        ("resolve-QName((),())", "XPTY0004"),
        ("in-scope-prefixes(())", "XPTY0004"),
        ("namespace-uri-for-prefix(1,r)", "XPTY0004"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
}
#[test]
fn node_name_handles_absent_focus_and_record_local_focus() {
    let s = Stylesheet::compile(&source("node-name()")).unwrap();
    let opts = TransformOptions {
        initial_template: Some("record".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("", &opts).unwrap_err().code,
        "XPDY0002"
    );
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<catalog><book/></catalog>"),
        &mut output,
        &StreamOptions {
            record_path: "/catalog/book".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output, b"book");
}
#[test]
fn namespace_scans_consume_the_shared_budget() {
    let mut xml = "<r".to_owned();
    for i in 0..200 {
        xml.push_str(&format!(" xmlns:p{i}='urn:{i}'"));
    }
    xml.push_str("/>");
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 50,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        Stylesheet::compile(&source("in-scope-prefixes(r)"))
            .unwrap()
            .transform_with_options(&xml, &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}
