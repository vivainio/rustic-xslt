use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression))
        .unwrap()
        .transform("<r a='v'><n>text</n></r>")
        .unwrap()
}
#[test]
fn string_concat_atomizes_optional_singletons_and_returns_string() {
    for (expr, expected) in [
        ("'a'||'b'||'c'", "abc"),
        ("1 || true()", "1true"),
        ("r/@a || ':' || r/n", "v:text"),
        ("() || 'x'", "x"),
        ("'x' || ()", "x"),
        ("() || ()", ""),
        ("(() || ()) instance of xs:string", "true"),
        ("'ä' || '🙂'", "ä🙂"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn string_concat_precedence_lies_between_comparison_and_range() {
    for (expr, expected) in [
        ("'x'||1+2", "x3"),
        ("'x'||1 to 1", "x1"),
        ("1 to 1 || 'x'", "1x"),
        ("'a'||'b' eq 'ab'", "true"),
        ("'a'||'b' = 'ab' and true()", "true"),
        ("'x'|| -2 * 3", "x-6"),
        ("'x'|| 10 idiv 3", "x3"),
        ("'x'|| r/n/string()", "xtext"),
        ("'x'|| (1 ! (. + 1))", "x2"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn string_concat_rejects_multiple_items_and_malformed_operands() {
    for expr in ["(1,2) || 'x'", "'x' || (1,2)", "() || (1,2)"] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "XPTY0004"
        );
    }
    for expr in ["'a'||", "||'a'", "'a'|||'b'"] {
        assert_eq!(
            Stylesheet::compile(&source(expr)).unwrap_err().code,
            "XPST0003"
        );
    }
}
#[test]
fn concatenated_string_copying_consumes_shared_budget() {
    let expr = "let $a := 'abcdefghij', $b := $a||$a, $c := $b||$b, $d := $c||$c return string-length($d||$d)";
    let s = Stylesheet::compile(&source(expr)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "160");
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
