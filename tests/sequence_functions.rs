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

#[test]
fn cardinality_functions_preserve_nodes_atomics_and_empty_sequences() {
    for (expr, expected) in [
        ("exactly-one(3)", "3"),
        ("exactly-one(3) instance of xs:integer", "true"),
        ("zero-or-one(())", ""),
        ("zero-or-one('') instance of xs:string", "true"),
        ("one-or-more((1,true(),'a'))", "1 true a"),
        ("exactly-one(r/n[1]) is r/n[1]", "true"),
        ("zero-or-one(r/@a) is r/@a", "true"),
        ("deep-equal(one-or-more(r/n), r/n)", "true"),
    ] {
        assert_eq!(
            sheet(expr).transform("<r a='v'><n/><n/></r>").unwrap(),
            expected,
            "{expr}"
        );
    }
}
#[test]
fn cardinality_functions_report_distinct_errors_and_validate_arity() {
    for (expr, code) in [
        ("exactly-one(())", "FORG0005"),
        ("exactly-one((1,2))", "FORG0005"),
        ("zero-or-one((1,2))", "FORG0003"),
        ("one-or-more(())", "FORG0004"),
    ] {
        assert_eq!(
            sheet(expr).transform("<r/>").unwrap_err().code,
            code,
            "{expr}"
        );
    }
    for expr in ["exactly-one()", "one-or-more(1,2)", "zero-or-one()"] {
        let style = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#
        );
        assert_eq!(Stylesheet::compile(&style).unwrap_err().code, "XPST0017");
    }
}

#[test]
fn distinct_values_atomizes_without_erasing_selected_atomic_types() {
    for (expr, expected) in [
        ("distinct-values((1, 2.0, 1, 2, 3))", "1 2 3"),
        (
            "count(distinct-values((xs:float('NaN'), xs:double('NaN'))))",
            "1",
        ),
        ("count(distinct-values((0, xs:double('-0'))))", "1"),
        (
            "count(distinct-values((1, '1', true(), xs:untypedAtomic('1'))))",
            "3",
        ),
        (
            "distinct-values((xs:byte(1), 1)) instance of xs:byte",
            "true",
        ),
        (
            "distinct-values(xs:untypedAtomic('x')) instance of xs:untypedAtomic",
            "true",
        ),
        (
            "count(distinct-values((QName('u','p:n'), QName('u','q:n'))))",
            "1",
        ),
        ("empty(distinct-values(()))", "true"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
    assert_eq!(
        sheet("distinct-values(r/n)")
            .transform("<r><n>a</n><n>a</n><n>b</n></r>")
            .unwrap(),
        "a b"
    );
}

#[test]
fn index_of_uses_atomic_equality_and_one_based_positions() {
    for (expr, expected) in [
        ("index-of((1, 2, 1.0), 1)", "1 3"),
        ("index-of((1, '1', xs:untypedAtomic('1')), 1)", "1"),
        ("index-of((1, '1', xs:untypedAtomic('1')), '1')", "2 3"),
        (
            "empty(index-of(xs:double('NaN'), xs:double('NaN')))",
            "true",
        ),
        ("index-of((QName('u','p:n'), 'n'), QName('u','q:n'))", "1"),
        ("empty(index-of((), 'x'))", "true"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
    assert_eq!(
        sheet("index-of(r/n, r/@search)")
            .transform("<r search='a'><n>a</n><n>b</n><n>a</n></r>")
            .unwrap(),
        "1 3"
    );
}

#[test]
fn atomic_sequence_functions_check_collations_cardinality_and_budgets() {
    for (expr, code) in [
        ("index-of((), ())", "XPTY0004"),
        ("index-of(1, (1,2))", "XPTY0004"),
        ("distinct-values((), 'urn:unknown')", "FOCH0002"),
        ("index-of((), 1, 'urn:unknown')", "FOCH0002"),
        ("distinct-values(1, ())", "XPTY0004"),
    ] {
        assert_eq!(
            sheet(expr).transform("<r/>").unwrap_err().code,
            code,
            "{expr}"
        );
    }
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 500,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet("distinct-values(1 to 50)")
            .transform_with_options("<r/>", &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}
