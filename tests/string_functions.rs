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

#[test]
fn codepoint_conversion_preserves_unicode_and_empty_sequence_semantics() {
    for (expr, expected) in [
        ("codepoints-to-string((65,233,128578))", "Aé🙂"),
        ("string-to-codepoints('Aé🙂')", "65 233 128578"),
        ("codepoints-to-string(()) instance of xs:string", "true"),
        ("empty(string-to-codepoints(()))", "true"),
        ("empty(string-to-codepoints(''))", "true"),
        (
            "codepoints-to-string(string-to-codepoints('a🙂é界'))",
            "a🙂é界",
        ),
        ("string-to-codepoints(r/@a)", "97 98 67 100"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
    assert_eq!(
        Stylesheet::compile(&source("codepoints-to-string(r/n)", "3.0"))
            .unwrap()
            .transform("<r><n>65</n></r>")
            .unwrap(),
        "A"
    );
}
#[test]
fn invalid_xml_codepoints_and_wrong_argument_types_report_distinct_errors() {
    for expr in [
        "codepoints-to-string(0)",
        "codepoints-to-string(-1)",
        "codepoints-to-string(55296)",
        "codepoints-to-string(65535)",
        "codepoints-to-string(1114112)",
        "codepoints-to-string(999999999999999999999999)",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr, "3.0"))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "FOCH0001",
            "{expr}"
        );
    }
    for expr in [
        "codepoints-to-string(65.0)",
        "codepoints-to-string('65')",
        "codepoints-to-string(true())",
        "string-to-codepoints(1)",
        "string-to-codepoints(('a','b'))",
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
}
#[test]
fn codepoint_conversions_consume_shared_budget() {
    for expr in [
        format!("string-to-codepoints('{}')", "a".repeat(200)),
        "codepoints-to-string(for $x in 1 to 200 return 65)".into(),
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

#[test]
fn codepoint_comparisons_return_typed_results_without_normalization() {
    for (expr, expected) in [
        ("compare('abc','abc')", "0"),
        ("compare('abc','abd')", "-1"),
        ("compare('ab','a')", "1"),
        ("compare('🙂','界')", "1"),
        ("compare('a','A')", "1"),
        ("compare('','') instance of xs:integer", "true"),
        ("codepoint-equal('é','é')", "false"),
        ("codepoint-equal('','')", "true"),
        ("codepoint-equal('🙂','🙂')", "true"),
        ("empty(compare((),'x'))", "true"),
        ("empty(codepoint-equal('x',()))", "true"),
        (
            "compare('a','b','http://www.w3.org/2005/xpath-functions/collation/codepoint')",
            "-1",
        ),
        ("codepoint-equal(r/@a, 'abCd')", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn string_comparisons_validate_types_collations_and_arity() {
    for (expr, code) in [
        ("compare('a','b','urn:unsupported')", "FOCH0002"),
        ("compare('a','b',())", "XPTY0004"),
        ("codepoint-equal((),1)", "XPTY0004"),
        ("compare(1,'a')", "XPTY0004"),
        ("compare('a',('a','b'))", "XPTY0004"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr, "3.0"))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
    for expr in ["compare('a')", "codepoint-equal('a','a','a')"] {
        assert_eq!(
            Stylesheet::compile(&source(expr, "3.0")).unwrap_err().code,
            "XPST0017"
        );
    }
}

#[test]
fn unicode_normalization_supports_canonical_and_compatibility_forms() {
    for (expr, expected) in [
        ("normalize-unicode('é')", "é"),
        ("normalize-unicode('é','NFD')", "é"),
        ("normalize-unicode('ﬃ①','NFKC')", "ffi1"),
        ("normalize-unicode('ﬃé','NFKD')", "ffié"),
        ("normalize-unicode('é','')", "é"),
        ("normalize-unicode('é',' nfc ')", "é"),
        ("normalize-unicode('é','  ')", "é"),
        ("normalize-unicode('가','NFC')", "가"),
        ("normalize-unicode('가','NFD')", "가"),
        ("normalize-unicode(()) instance of xs:string", "true"),
        ("codepoint-equal(normalize-unicode('é'), 'é')", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn unicode_normalization_reorders_combining_marks() {
    assert_eq!(
        evaluate(
            "string-to-codepoints(normalize-unicode(codepoints-to-string((97,789,768)), 'NFD'))"
        ),
        "97 768 789"
    );
}
#[test]
fn normalization_forms_and_function_conversion_are_validated() {
    for (expr, code) in [
        ("normalize-unicode('a','UNKNOWN')", "FOCH0003"),
        ("normalize-unicode('a','FULLY-NORMALIZED')", "FOCH0003"),
        ("normalize-unicode(1)", "XPTY0004"),
        ("normalize-unicode('a',())", "XPTY0004"),
        ("normalize-unicode(('a','b'))", "XPTY0004"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr, "3.0"))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
    assert_eq!(
        Stylesheet::compile(&source("normalize-unicode()", "3.0"))
            .unwrap_err()
            .code,
        "XPST0017"
    );
}
#[test]
fn unicode_normalization_consumes_shared_budget() {
    let expr = format!("normalize-unicode('{}','NFKD')", "ﬃ".repeat(200));
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
