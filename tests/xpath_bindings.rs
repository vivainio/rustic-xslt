use rust_xslt::{Limits, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" xmlns:q="urn:p"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
#[test]
fn for_bindings_generate_ordered_cartesian_products() {
    let s = Stylesheet::compile(&source("for $x in 1 to 2, $y in (10,20) return $x + $y")).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "11 21 12 22");
    assert_eq!(
        Stylesheet::compile(&source("for $x in () return 1 div 0"))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        ""
    );
}
#[test]
fn let_binds_whole_sequences_and_preserves_empty_bindings() {
    for (expr, expected) in [
        ("let $x := (1,2,3) return sum($x)", "6"),
        ("let $x := () return count($x)", "0"),
        ("let $x := 1, $y := $x + 2 return $y", "3"),
        ("let $x:=1 return$x", "1"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap(),
            expected
        );
    }
}
#[test]
fn bindings_preserve_the_callers_focus_and_shadow_lexically() {
    let expr = "let $x := 5 return (for $x in (1,2) return concat(name(), ':', position(), '/', last(), ':', $x), $x)";
    assert_eq!(
        Stylesheet::compile(&source(expr))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        ":1/1:1 :1/1:2 5"
    );
}
#[test]
fn binding_selects_see_outer_scopes_and_expanded_names() {
    let expr = "let $p:x := 3 return (let $q:x := $p:x + 1 return $p:x)";
    assert_eq!(
        Stylesheet::compile(&source(expr))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "4"
    );
    let expr = "for $Q{ urn:p }x in 1 to 3 return $q:x * 2";
    assert_eq!(
        Stylesheet::compile(&source(expr))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "2 4 6"
    );
}
#[test]
fn binding_variables_do_not_leak_and_forward_references_are_static_errors() {
    for expr in [
        "(let $x := 1 return $x), $x",
        "let $x := $x return $x",
        "for $x in () return $missing",
        "let $x := $y, $y := 1 return $x",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr)).unwrap_err().code,
            "XPST0008",
            "{expr}"
        );
    }
}
#[test]
fn malformed_bindings_and_unparenthesized_operands_are_rejected() {
    for expr in [
        "for $x 1 return $x",
        "let $x = 1 return $x",
        "1 + for $x in 1 return $x",
        "let $x := 1 $x",
        "for $x in 1return $x",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr)).unwrap_err().code,
            "XPST0003"
        );
    }
}
#[test]
fn binding_loops_consume_the_shared_budget() {
    let s =
        Stylesheet::compile(&source("for $x in 1 to 10, $y in 1 to 10 return $x + $y")).unwrap();
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap_err().code,
        "XPDY0130"
    );
}
#[test]
fn record_safety_traverses_binding_selects_and_return_expressions() {
    let style = source("1").replace(r#"<xsl:template match="/"><xsl:sequence select="1"/></xsl:template>"#, r#"<xsl:template name="record"><xsl:sequence select="let $x := n return $x/../outside"/></xsl:template>"#);
    let s = Stylesheet::compile(&style).unwrap();
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

#[test]
fn nested_binding_lists_cannot_bypass_the_xpath_depth_limit() {
    let bindings = (0..100)
        .map(|i| format!("$v{i}:=1"))
        .collect::<Vec<_>>()
        .join(",");
    let expr = format!("let {bindings} return let {bindings} return 1");
    assert_eq!(
        Stylesheet::compile(&source(&expr)).unwrap_err().code,
        "XPST0003"
    );
}
