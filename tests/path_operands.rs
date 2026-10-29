use rust_xslt::{Limits, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn sheet(expression: &str) -> Stylesheet {
    Stylesheet::compile(&source(expression)).unwrap()
}
fn evaluate(expression: &str) -> String {
    sheet(expression)
        .transform("<r><a>1</a><b>2</b><a>3</a></r>")
        .unwrap()
}
#[test]
fn path_steps_accept_parenthesized_expressions_and_function_calls() {
    assert_eq!(evaluate("r/(a | b)/string()"), "1 2 3");
    assert_eq!(evaluate("r/(a, b)/string()"), "1 2 3");
    assert_eq!(
        evaluate("r/*/concat(name(), ':', string(.))"),
        "a:1 b:2 a:3"
    );
    assert_eq!(evaluate("/(r/a)/string()"), "1 3");
    assert_eq!(evaluate("//(a | b)/string()"), "1 2 3");
}
#[test]
fn path_focus_preserves_input_sequence_for_atomic_final_steps() {
    assert_eq!(
        evaluate("(r/b,r/a[1],r/b)/concat(name(), ':', position(), '/', last())"),
        "b:1/3 a:2/3 b:3/3"
    );
    assert_eq!(evaluate("r/*/1"), "1 1 1");
    assert_eq!(evaluate("r/*/('x','y')"), "x y x y x y");
    assert_eq!(
        evaluate("r/*/(if (position() = last()) then 'last' else name())"),
        "a b last"
    );
}
#[test]
fn node_results_normalize_before_subsequent_step_focus() {
    assert_eq!(
        evaluate("(r/b,r/a[1],r/b)/(.)/concat(name(), ':', position(), '/', last())"),
        "a:1/2 b:2/2"
    );
    assert_eq!(evaluate("r/*/(..)/name()"), "r");
    assert_eq!(evaluate("r/(b,a)[1]/string()"), "2");
    assert_eq!(evaluate("r/(a,b)/name()[1]"), "a b a");
}
#[test]
fn path_errors_distinguish_atomic_inputs_from_mixed_outputs() {
    for (expression, code) in [
        ("1/string()", "XPTY0019"),
        ("(r,1)/()", "XPTY0019"),
        ("r/*/string()/name()", "XPTY0019"),
        ("r/(.,1)", "XPTY0018"),
        ("r/*/(if (position() = 1) then . else 1)", "XPTY0018"),
    ] {
        assert_eq!(
            sheet(expression)
                .transform("<r><a/><b/></r>")
                .unwrap_err()
                .code,
            code,
            "{expression}"
        );
    }
    assert_eq!(evaluate("()/string()"), "");
    assert_eq!(evaluate("r/()"), "");
}
#[test]
fn general_path_operands_validate_scope_and_grammar() {
    assert_eq!(
        Stylesheet::compile(&source("r/($missing)"))
            .unwrap_err()
            .code,
        "XPST0008"
    );
    for expression in [
        "r/+1",
        "r/if (true()) then . else .",
        "r/for $x in 1 return $x",
        "r//",
        "r/(1",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expression)).unwrap_err().code,
            "XPST0003",
            "{expression}"
        );
    }
}
#[test]
fn general_path_safety_traverses_both_operands_for_record_streaming() {
    for expression in ["r/(../outside)", "r/(root())", "/(r)"] {
        let style = source(expression).replace("match=\"/\"", "name=\"record\"");
        let mut output = vec![];
        assert_eq!(
            Stylesheet::compile(&style)
                .unwrap()
                .stream_records(
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
fn general_path_iterations_consume_shared_budget() {
    let s = sheet("(for $i in 1 to 50 return r)/(1 to 50)");
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 500,
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
fn general_path_chains_have_a_static_nesting_bound() {
    let expression = format!("r{}", "/(.)".repeat(140));
    assert_eq!(
        Stylesheet::compile(&source(&expression)).unwrap_err().code,
        "XPST0003"
    );
}
