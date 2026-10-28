use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn sheet(expression: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#)).unwrap()
}
#[test]
fn reverse_head_and_tail_preserve_sequence_items() {
    assert_eq!(
        sheet("reverse((1,2,3))").transform("<r/>").unwrap(),
        "3 2 1"
    );
    assert_eq!(sheet("head((1,2,3))").transform("<r/>").unwrap(), "1");
    assert_eq!(sheet("tail((1,2,3))").transform("<r/>").unwrap(), "2 3");
    assert_eq!(
        sheet("head(r/n) is r/n[1]")
            .transform("<r><n/><n/></r>")
            .unwrap(),
        "true"
    );
    for expression in ["reverse(())", "head(())", "tail(())", "tail(1)"] {
        assert_eq!(sheet(expression).transform("<r/>").unwrap(), "");
    }
}
#[test]
fn remove_and_insert_before_use_exact_integer_positions() {
    for (expr, expected) in [
        ("remove((1,2,3), 2)", "1 3"),
        ("remove((1,2), -1)", "1 2"),
        ("remove((1,2), 999999999999999999999)", "1 2"),
        ("insert-before((1,2), 2, (3,4))", "1 3 4 2"),
        ("insert-before((1,2), -1, 3)", "3 1 2"),
        ("insert-before((1,2), 999999999999999999999, 3)", "1 2 3"),
        ("insert-before((), 1, (3,4))", "3 4"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected);
    }
}
#[test]
fn subsequence_rounds_half_toward_positive_infinity() {
    for (expr, expected) in [
        ("subsequence(1 to 5, 1.5, 2.5)", "2 3 4"),
        ("subsequence(1 to 5, -0.5, 2)", "1"),
        ("subsequence(1 to 5, 0, 3)", "1 2"),
        ("subsequence(1 to 5, 3)", "3 4 5"),
        ("subsequence(1 to 5, 2, -1)", ""),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected);
    }
}
#[test]
fn subsequence_obeys_nan_and_infinity_rules() {
    for (expr, expected) in [
        ("subsequence(1 to 3, xs:double('NaN'))", ""),
        ("subsequence(1 to 3, xs:double('-INF'))", "1 2 3"),
        (
            "subsequence(1 to 3, xs:double('-INF'), xs:double('INF'))",
            "",
        ),
        ("subsequence(1 to 3, 2, xs:double('INF'))", "2 3"),
        ("subsequence(1 to 3, xs:double('INF'))", ""),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected);
    }
}
#[test]
fn sequence_position_arguments_apply_function_conversion() {
    assert_eq!(
        sheet("subsequence(1 to 5, r/@start, r/@length)")
            .transform("<r start='2' length='2'/>")
            .unwrap(),
        "2 3"
    );
    assert_eq!(
        sheet("remove((1,2,3), r/@position)")
            .transform("<r position='2'/>")
            .unwrap(),
        "1 3"
    );
    for expr in [
        "remove((1,2), 1.0)",
        "remove((1,2), ())",
        "insert-before((1,2), '1', 3)",
        "subsequence((1,2), true())",
        "subsequence((1,2), ())",
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap_err().code, "XPTY0004");
    }
}
#[test]
fn sequence_function_work_consumes_the_shared_budget() {
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 40,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet("reverse(1 to 30)")
            .transform_with_options("<r/>", &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}
