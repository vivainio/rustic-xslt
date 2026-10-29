use rust_xslt::{Limits, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" xmlns:q="urn:p"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression))
        .unwrap()
        .transform("<r/>")
        .unwrap()
}
#[test]
fn quantifiers_use_boolean_results_and_empty_domain_identities() {
    for (expression, expected) in [
        ("some $x in (1,2) satisfies $x = 2", "true"),
        ("some $x in (1,2) satisfies $x = 3", "false"),
        ("every $x in (1,2) satisfies $x > 0", "true"),
        ("every $x in (1,2) satisfies $x = 1", "false"),
        ("some $x in () satisfies 1 div 0", "false"),
        ("every $x in () satisfies 1 div 0", "true"),
        ("every $x in (0,1), $y in () satisfies false()", "true"),
        ("some $x in 1, $y in () satisfies true()", "false"),
    ] {
        assert_eq!(evaluate(expression), expected, "{expression}");
    }
}
#[test]
fn dependent_domains_and_nested_quantifiers_preserve_scope_and_focus() {
    assert_eq!(
        evaluate("some $x in (1,2), $y in ($x, $x + 1) satisfies $y = 3"),
        "true"
    );
    assert_eq!(
        evaluate("every $x in (1,2) satisfies some $y in (2,4) satisfies $y = $x * 2"),
        "true"
    );
    assert_eq!(
        evaluate("let $p:x := 5 return (some $q:x in $p:x satisfies $p:x = 5, $q:x)"),
        "true 5"
    );
    assert_eq!(
        evaluate(
            "some $Q{urn:p}x in (1,2) satisfies $q:x = 2 and position() = 1 and last() = 1 and name() = ''"
        ),
        "true"
    );
}
#[test]
fn quantifiers_short_circuit_satisfying_expressions() {
    assert_eq!(evaluate("some $x in (1,0) satisfies 1 div $x = 1"), "true");
    assert_eq!(
        evaluate("every $x in (1,0) satisfies 1 div $x = 2"),
        "false"
    );
    let s = Stylesheet::compile(&source("some $x in 1 satisfies (1,2)")).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "FORG0006");
}
#[test]
fn quantifier_scope_is_checked_even_when_domain_is_empty() {
    for expression in [
        "some $x in () satisfies $missing",
        "every $x in $x satisfies true()",
        "some $x in $y, $y in 1 satisfies true()",
        "(some $x in 1 satisfies true()), $x",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expression)).unwrap_err().code,
            "XPST0008",
            "{expression}"
        );
    }
}
#[test]
fn quantified_grammar_requires_binding_and_satisfies_and_parentheses_in_operands() {
    for expression in [
        "some $x in 1 return $x",
        "every $x := 1 satisfies true()",
        "some $x in 1 satisfies",
        "some $x in 1",
        "true() and some $x in 1 satisfies true()",
        "1 ! some $x in 1 satisfies true()",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expression)).unwrap_err().code,
            "XPST0003",
            "{expression}"
        );
    }
    assert_eq!(
        evaluate("true() and (some $x in 1 satisfies true())"),
        "true"
    );
    assert_eq!(
        evaluate("(some $x in 1 satisfies false()), true()"),
        "false true"
    );
    assert_eq!(
        evaluate("if (some $x in 1 satisfies true()) then 2 else 3"),
        "2"
    );
}
#[test]
fn quantifier_iterations_consume_shared_budget() {
    let s = Stylesheet::compile(&source(
        "every $x in 1 to 20, $y in 1 to 20 satisfies $x + $y > 0",
    ))
    .unwrap();
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
fn record_safety_checks_quantifier_domains_and_conditions() {
    for expression in [
        "some $x in /r satisfies true()",
        "every $x in n satisfies empty($x/../outside)",
    ] {
        let style = source(expression).replace("match=\"/\"", "name=\"record\"");
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
}
#[test]
fn long_nested_quantifier_lists_respect_static_depth_limit() {
    let bindings = (0..100)
        .map(|i| format!("$v{i} in 1"))
        .collect::<Vec<_>>()
        .join(",");
    let expression = format!("some {bindings} satisfies every {bindings} satisfies true()");
    assert_eq!(
        Stylesheet::compile(&source(&expression)).unwrap_err().code,
        "XPST0003"
    );
}
