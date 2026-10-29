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

#[test]
fn absolute_floor_and_ceiling_keep_exact_values_and_numeric_types() {
    for (e, want) in [
        ("abs(-9007199254740993)", "9007199254740993"),
        ("abs(-0.1)", "0.1"),
        ("floor(-1.1)", "-2"),
        ("ceiling(-1.1)", "-1"),
        ("floor(1.9)", "1"),
        ("ceiling(1.1)", "2"),
        ("floor(9007199254740993.9)", "9007199254740993"),
        ("floor(3) instance of xs:integer", "true"),
        ("ceiling(3.1) instance of xs:decimal", "true"),
        ("abs(xs:float('-2')) instance of xs:float", "true"),
        ("floor(2e0) instance of xs:double", "true"),
        (
            "empty(abs(())) and empty(floor(())) and empty(ceiling(()))",
            "true",
        ),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn numeric_unary_functions_preserve_special_values_and_signed_zero() {
    for (e, want) in [
        ("abs(xs:double('-INF'))", "INF"),
        ("ceiling(xs:float('INF'))", "INF"),
        ("floor(xs:double('-INF'))", "-INF"),
        ("floor(xs:double('NaN'))", "NaN"),
        ("1e0 div ceiling(-0.2e0)", "-INF"),
        ("1e0 div abs(-0e0)", "INF"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn unary_numeric_functions_atomize_nodes_and_reject_non_numeric_atomics() {
    assert_eq!(
        transform(&sheet("3.0", "floor(r/@a)"), "<r a='2.9'/>").unwrap(),
        "2"
    );
    for e in ["abs('1')", "floor(true())", "ceiling((1,2))"] {
        assert_eq!(
            transform(&sheet("3.0", e), "<r/>").unwrap_err().code,
            "XPTY0004",
            "{e}"
        );
    }
    assert_eq!(
        transform(&sheet("1.0", "floor(r/@missing)"), "<r/>").unwrap(),
        "NaN"
    );
}

#[test]
fn modern_float_strings_preserve_signed_zero_and_use_normalized_exponents() {
    for (e, want) in [
        ("xs:double('-0')", "-0"),
        ("xs:float('-0')", "-0"),
        ("1e6", "1.0E6"),
        ("1.234e7", "1.234E7"),
        ("-1e-7", "-1.0E-7"),
        ("1e-6", "0.000001"),
        ("999999e0", "999999"),
        ("xs:float('3.4028235e38')", "3.4028235E38"),
        ("xs:float('1e-45')", "1.0E-45"),
        ("string(1e6)", "1.0E6"),
        ("xs:string(1e6)", "1.0E6"),
        ("ceiling(-0.2e0)", "-0"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn numeric_lexical_boundaries_preserve_round_trip_values() {
    for lexical in [
        "1.2345678901234567e100",
        "5e-324",
        "1.7976931348623157e308",
        "0.0000009999999999999999",
        "0.0000010000000000000002",
    ] {
        let e = format!("let $n := xs:double('{lexical}') return xs:double(xs:string($n)) eq $n");
        assert_eq!(evaluate(&e), "true", "{lexical}");
    }
}
#[test]
fn legacy_numeric_strings_keep_plain_notation_and_legacy_special_values() {
    for (e, want) in [
        ("-0e0", "0"),
        ("1e6", "1000000"),
        ("1e-7", "0.0000001"),
        ("1e0 div 0e0", "Infinity"),
        ("-1e0 div 0e0", "-Infinity"),
        ("string(-0e0)", "0"),
        ("concat(1e6, ':', -0e0)", "1000000:0"),
    ] {
        assert_eq!(transform(&sheet("1.0", e), "<r/>").unwrap(), want, "{e}");
    }
    let style = r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><out a="{1e6}"><xsl:attribute name="b" select="1e-7"/><xsl:sequence select="1e6"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        transform(style, "<r/>").unwrap(),
        "<out a=\"1000000\" b=\"0.0000001\">1000000</out>"
    );
}

#[test]
fn text_sort_keys_use_their_stylesheets_numeric_string_rules() {
    for (version, expected) in [("1.0", "0.0000001;0.000001;"), ("3.0", "0.000001;1.0E-7;")] {
        let style = format!(
            r#"<xsl:stylesheet version="{version}" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:for-each select="(1e-6,1e-7)"><xsl:sort select="." data-type="text"/><xsl:value-of select="concat(.,';')"/></xsl:for-each></xsl:template></xsl:stylesheet>"#
        );
        assert_eq!(transform(&style, "<r/>").unwrap(), expected);
    }
}

#[test]
fn rounding_uses_distinct_tie_rules_without_losing_decimal_precision() {
    for (e, want) in [
        ("round(2.5)", "3"),
        ("round(-2.5)", "-2"),
        ("round-half-to-even(2.5)", "2"),
        ("round-half-to-even(3.5)", "4"),
        ("round-half-to-even(-2.5)", "-2"),
        ("round(9007199254740993.5)", "9007199254740994"),
        ("round-half-to-even(9007199254740992.5)", "9007199254740992"),
        ("round(12.345,2)", "12.35"),
        ("round(-12.345,2)", "-12.34"),
        ("round-half-to-even(12.345,2)", "12.34"),
        ("round(125,-1)", "130"),
        ("round-half-to-even(125,-1)", "120"),
        ("round-half-to-even(135,-1)", "140"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn rounding_preserves_numeric_types_and_floating_special_values() {
    for (e, want) in [
        ("round(12,-1) instance of xs:integer", "true"),
        ("round(12.1) instance of xs:decimal", "true"),
        ("round(xs:float('1.5')) instance of xs:float", "true"),
        ("round-half-to-even(1.5e0) instance of xs:double", "true"),
        ("round(-0.5e0)", "-0"),
        ("round-half-to-even(-0.2e0)", "-0"),
        ("round(xs:double('INF'))", "INF"),
        ("round-half-to-even(xs:float('-INF'))", "-INF"),
        ("round(xs:double('NaN'))", "NaN"),
        (
            "round-half-to-even(xs:float('150.015'),2) eq xs:float('150.01')",
            "true",
        ),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn extreme_rounding_precisions_do_not_allocate_huge_powers_of_ten() {
    for (e, want) in [
        ("round(1.234,999999999999999999999999)", "1.234"),
        ("round-half-to-even(123,-999999999999999999999999)", "0"),
        ("round(-1e0,-999999999999999999999999)", "-0"),
        ("round(1.234,1000000000)", "1.234"),
        ("round-half-to-even(123,-1000000000)", "0"),
        ("empty(round(())) and empty(round-half-to-even(()))", "true"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn rounding_validates_precision_cardinality_and_numeric_arguments() {
    for e in [
        "round('1')",
        "round(1,())",
        "round-half-to-even(1,2.0)",
        "round(1,'2')",
        "round((),())",
        "round((1,2))",
    ] {
        assert_eq!(
            transform(&sheet("3.0", e), "<r/>").unwrap_err().code,
            "XPTY0004",
            "{e}"
        );
    }
    assert_eq!(
        transform(&sheet("3.0", "round(r/@a,r/@p)"), "<r a='1.234' p='2'/>").unwrap(),
        "1.23"
    );
    assert_eq!(
        transform(&sheet("1.0", "round(-0.5)"), "<r/>").unwrap(),
        "0"
    );
}
