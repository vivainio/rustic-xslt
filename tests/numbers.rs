use rust_xslt::transform;
fn sheet(version: &str, e: &str) -> String {
    format!(
        r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" version="{version}"><xsl:output method="text"/><xsl:template match="/"><xsl:value-of select="{e}"/></xsl:template></xsl:stylesheet>"##
    )
}
fn evaluate(e: &str) -> String {
    transform(&sheet("3.0", e), "<r/>").unwrap()
}
#[test]
fn integer_and_decimal_arithmetic_is_exact() {
    for (e, want) in [
        ("9007199254740993 + 1", "9007199254740994"),
        ("999999999999999999999999 * 2", "1999999999999999999999998"),
        ("0.1 + 0.2", "0.3"),
        ("1.2 * 3.4", "4.08"),
        ("1 div 8", "0.125"),
        ("10 mod 3", "1"),
        ("-10 mod 3", "-1"),
        ("-0.00", "0"),
        ("sum((0.1, 0.2, 0.3))", "0.6"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn exact_comparisons_and_double_promotion_are_distinct() {
    assert_eq!(evaluate("9007199254740993 = 9007199254740992"), "false");
    assert_eq!(
        evaluate("xs:double(9007199254740993) = 9007199254740992"),
        "true"
    );
    assert_eq!(evaluate("0.1 + 0.2 eq 0.3"), "true");
    assert_eq!(evaluate("'10' lt '2'"), "true");
    assert_eq!(evaluate("xs:double('NaN') ne xs:double('NaN')"), "true");
}
#[test]
fn constructors_handle_numeric_boolean_and_untyped_values() {
    for (e, want) in [
        ("xs:integer(2.9)", "2"),
        ("xs:integer(-2.9)", "-2"),
        (
            "xs:integer('999999999999999999999')",
            "999999999999999999999",
        ),
        ("xs:decimal('0.1000')", "0.1"),
        ("xs:double('INF')", "INF"),
        ("xs:double('-INF')", "-INF"),
        ("xs:double('NaN')", "NaN"),
        ("xs:boolean('0')", "false"),
        ("xs:boolean(0 div 1)", "false"),
        ("xs:boolean(xs:double('NaN'))", "false"),
        ("xs:untypedAtomic('2') + 3", "5"),
        ("xs:string(true())", "true"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn constructors_resolve_namespace_aliases_and_reject_wrong_bindings() {
    let s = sheet("3.0", "schema:integer('7') + 1").replace("xmlns:xs=", "xmlns:schema=");
    assert_eq!(transform(&s, "<r/>").unwrap(), "8");
    let s =
        sheet("3.0", "xs:integer('7')").replace("http://www.w3.org/2001/XMLSchema", "urn:wrong");
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XPST0017");
}
#[test]
fn invalid_lexicals_and_division_by_zero_report_standard_errors() {
    for (e, code) in [
        ("xs:integer('1.5')", "FORG0001"),
        ("xs:decimal('1e2')", "FORG0001"),
        ("xs:double('Infinity')", "FORG0001"),
        ("xs:boolean('yes')", "FORG0001"),
        ("xs:integer(xs:double('INF'))", "FOCA0002"),
        ("1 div 0", "FOAR0001"),
        ("1 mod 0", "FOAR0001"),
        ("'2' + 1", "XPTY0004"),
        ("(1,2) eq 1", "XPTY0004"),
        ("1 eq '1'", "XPTY0004"),
    ] {
        assert_eq!(
            transform(&sheet("3.0", e), "<r/>").unwrap_err().code,
            code,
            "{e}"
        );
    }
    assert_eq!(evaluate("1e0 div 0e0"), "INF");
}
#[test]
fn empty_numeric_operands_remain_empty_and_value_comparison_is_singleton() {
    for e in ["() + 1", "-()", "() eq 1", "xs:integer(())"] {
        assert_eq!(evaluate(e), "", "{e}");
    }
    assert_eq!(evaluate("boolean((1,2) = 2)"), "true");
}
#[test]
fn node_atomization_and_data_are_untyped_atomic() {
    assert_eq!(
        transform(&sheet("3.0", "data(r/n) + 1"), "<r><n>2</n></r>").unwrap(),
        "3"
    );
    assert_eq!(
        transform(&sheet("3.0", "r/n = 2"), "<r><n>2</n></r>").unwrap(),
        "true"
    );
    assert_eq!(
        transform(&sheet("3.0", "r/n eq 2"), "<r><n>2</n></r>")
            .unwrap_err()
            .code,
        "XPTY0004"
    );
}
#[test]
fn xpath_one_numeric_conversions_and_empty_node_boolean_comparison() {
    assert_eq!(transform(&sheet("1.0", "'2' + 1"), "<r/>").unwrap(), "3");
    assert_eq!(
        transform(&sheet("1.0", "missing = false()"), "<r/>").unwrap(),
        "true"
    );
}
#[test]
fn numeric_implementation_limit_is_explicit() {
    let e = format!("xs:integer('{}')", "9".repeat(10_001));
    assert_eq!(
        transform(&sheet("3.0", &e), "<r/>").unwrap_err().code,
        "XPDY0130"
    );
}

#[test]
fn integer_division_is_exact_and_truncates_toward_zero() {
    for (e, want) in [
        ("10 idiv 3", "3"),
        ("-10 idiv 3", "-3"),
        ("10 idiv -3", "-3"),
        ("-10 idiv -3", "3"),
        ("9007199254740993 idiv 1", "9007199254740993"),
        ("5.99 idiv 0.2", "29"),
        ("-5.99 idiv 0.2", "-29"),
        ("1.999999999999999999999999999999 idiv 2", "0"),
        ("10 idiv 0.01", "1000"),
        ("(10 idiv 3) instance of xs:integer", "true"),
        ("2 + 10 idiv 3 * 2", "8"),
        ("() idiv 0", ""),
        ("1 idiv ()", ""),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn floating_integer_division_uses_promoted_precision_and_returns_integer() {
    for (e, want) in [
        ("10e0 idiv 3", "3"),
        ("xs:float('-10') idiv 3", "-3"),
        ("xs:float('16777216') idiv 16777217", "1"),
        ("xs:float('16777216') idiv xs:double('16777217')", "0"),
        ("3 idiv xs:double('INF')", "0"),
        ("-3 idiv xs:float('-INF')", "0"),
        ("(10e0 idiv 3) instance of xs:integer", "true"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn integer_division_reports_zero_invalid_and_overflow_errors() {
    for (e, code) in [
        ("1 idiv 0", "FOAR0001"),
        ("1.0 idiv 0.0", "FOAR0001"),
        ("1e0 idiv -0e0", "FOAR0001"),
        ("1 idiv xs:float('0')", "FOAR0001"),
        ("xs:double('INF') idiv 1", "FOAR0002"),
        ("1 idiv xs:float('NaN')", "FOAR0002"),
        ("xs:double('NaN') idiv 1", "FOAR0002"),
        ("1e308 idiv 1e-308", "FOAR0002"),
        ("xs:float('3e38') idiv xs:float('1e-38')", "FOAR0002"),
        ("(1,2) idiv 1", "XPTY0004"),
        ("'1' idiv 1", "XPTY0004"),
    ] {
        assert_eq!(
            transform(&sheet("3.0", e), "<r/>").unwrap_err().code,
            code,
            "{e}"
        );
    }
}
#[test]
fn integer_division_atomizes_nodes_as_untyped_numbers() {
    assert_eq!(
        transform(&sheet("3.0", "r/@a idiv r/b"), "<r a='10'><b>3</b></r>").unwrap(),
        "3"
    );
}
