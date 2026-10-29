use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn source(expression: &str, version: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="{version}" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression, "3.0"))
        .unwrap()
        .transform("<r a='abCd'><n>2</n></r>")
        .unwrap()
}
#[test]
fn unicode_case_mapping_preserves_full_expansions_and_empty_inputs() {
    for (expr, expected) in [
        ("upper-case('abCd0')", "ABCD0"),
        ("lower-case('ABc!D')", "abc!d"),
        ("upper-case('Straße')", "STRASSE"),
        ("lower-case('İ')", "i\u{307}"),
        ("lower-case('ΟΣ')", "ος"),
        ("upper-case(r/@a)", "ABCD"),
        ("lower-case(())", ""),
        ("upper-case(()) instance of xs:string", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn translation_uses_first_mapping_and_deletes_missing_replacements() {
    for (expr, expected) in [
        ("translate('bar','abc','ABC')", "BAr"),
        ("translate('--aaa--','abc-','ABC')", "AAA"),
        ("translate('abcdabc','abc','AB')", "ABdAB"),
        ("translate('aba','aba','XYZ')", "XYX"),
        ("translate('abc','','longer')", "abc"),
        ("translate('🙂é🙂', '🙂é', '界')", "界界"),
        ("translate((), 'a', 'b')", ""),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn substring_uses_one_based_unicode_positions_and_xpath_rounding() {
    for (expr, expected) in [
        ("substring('12345',1.5,2.6)", "234"),
        ("substring('12345',0,3)", "12"),
        ("substring('12345',5,-3)", ""),
        ("substring('12345',-3,5)", "1"),
        ("substring('12345',-0.5,2)", "1"),
        ("substring('a🙂é界',2,2)", "🙂é"),
        ("substring('abc',2)", "bc"),
        ("substring((),1,3)", ""),
        ("substring('abc',r/n)", "bc"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn substring_handles_nan_and_infinity_without_integer_index_casts() {
    for (expr, expected) in [
        ("substring('12345',xs:double('NaN'),3)", ""),
        ("substring('12345',1,xs:double('NaN'))", ""),
        ("substring('12345',-42,xs:double('INF'))", "12345"),
        ("substring('12345',xs:double('-INF'),xs:double('INF'))", ""),
        ("substring('12345',xs:double('-INF'))", "12345"),
        ("substring('12345',xs:double('INF'))", ""),
        ("substring('12345',1e300)", ""),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn string_functions_validate_types_cardinality_and_arity() {
    for expr in [
        "upper-case(1)",
        "lower-case(true())",
        "upper-case(('a','b'))",
        "translate('a',(), 'b')",
        "translate('a','a',())",
        "substring('a','1')",
        "substring('a',())",
        "substring('a',1,(1,2))",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr, "3.0"))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "XPTY0004",
            "{expr}"
        );
    }
    for expr in ["upper-case()", "substring('a')", "translate('a','b')"] {
        assert_eq!(
            Stylesheet::compile(&source(expr, "3.0")).unwrap_err().code,
            "XPST0017"
        );
    }
}
#[test]
fn backwards_compatible_string_functions_convert_legacy_arguments() {
    for (expr, expected) in [
        ("upper-case(123)", "123"),
        ("substring('abcd','2','2')", "bc"),
        ("substring('abc',())", ""),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr, "1.0"))
                .unwrap()
                .transform("<r/>")
                .unwrap(),
            expected,
            "{expr}"
        );
    }
}
#[test]
fn string_traversals_consume_shared_budget() {
    let text = "a".repeat(200);
    for expr in [
        format!("upper-case('{text}')"),
        format!("translate('{text}','a','b')"),
        format!("substring('{text}',1)"),
    ] {
        let s = Stylesheet::compile(&source(&expr, "3.0")).unwrap();
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
}
