use rust_xslt::Stylesheet;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression))
        .unwrap()
        .transform("<r a='12'/>")
        .unwrap()
}
#[test]
fn signed_and_unsigned_subtypes_accept_exact_boundary_values() {
    for (kind, low, high) in [
        ("byte", "-128", "127"),
        ("short", "-32768", "32767"),
        ("int", "-2147483648", "2147483647"),
        ("long", "-9223372036854775808", "9223372036854775807"),
        ("unsignedByte", "0", "255"),
        ("unsignedShort", "0", "65535"),
        ("unsignedInt", "0", "4294967295"),
        ("unsignedLong", "0", "18446744073709551615"),
    ] {
        for value in [low, high] {
            assert_eq!(evaluate(&format!("xs:{kind}('{value}')")), value);
            assert_eq!(
                evaluate(&format!("xs:{kind}('{value}') instance of xs:{kind}")),
                "true"
            );
        }
    }
}
#[test]
fn restriction_facets_and_subtype_hierarchy_preserve_type_identity() {
    for (expr, want) in [
        ("xs:byte(1) instance of xs:short", "true"),
        ("xs:byte(1) instance of xs:long", "true"),
        (
            "xs:unsignedByte(1) instance of xs:nonNegativeInteger",
            "true",
        ),
        (
            "xs:positiveInteger(1) instance of xs:nonNegativeInteger",
            "true",
        ),
        (
            "xs:negativeInteger(-1) instance of xs:nonPositiveInteger",
            "true",
        ),
        ("xs:unsignedInt(1) instance of xs:long", "false"),
        ("xs:int(1) instance of xs:positiveInteger", "false"),
        ("xs:byte(1) instance of xs:integer", "true"),
        ("xs:byte(1) instance of xs:decimal", "true"),
        ("xs:byte(1) instance of xs:numeric", "true"),
    ] {
        assert_eq!(evaluate(expr), want, "{expr}");
    }
}
#[test]
fn bounds_errors_distinguish_numeric_casts_from_invalid_lexical_values() {
    for (expr, code) in [
        ("xs:byte('128')", "FORG0001"),
        ("xs:byte(128)", "FOCA0002"),
        ("xs:unsignedLong('18446744073709551616')", "FORG0001"),
        ("xs:unsignedByte(-1)", "FOCA0002"),
        ("xs:negativeInteger(0)", "FOCA0002"),
        ("xs:positiveInteger('0')", "FORG0001"),
        ("xs:nonPositiveInteger(1)", "FOCA0002"),
        ("xs:nonNegativeInteger(-1)", "FOCA0002"),
        ("xs:int('1.0')", "FORG0001"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
    assert_eq!(evaluate("128 castable as xs:byte"), "false");
    assert_eq!(evaluate("'128' castable as xs:byte"), "false");
}
#[test]
fn numeric_casts_truncate_and_arithmetic_returns_unrestricted_integer_values() {
    for (expr, want) in [
        ("xs:byte(127.9)", "127"),
        ("xs:unsignedByte(-0.9)", "0"),
        ("xs:byte(true())", "1"),
        ("xs:int(r/@a)", "12"),
        ("xs:byte(127) + 1", "128"),
        ("-xs:byte(-128)", "128"),
        (
            "xs:unsignedLong('18446744073709551615') + 1",
            "18446744073709551616",
        ),
        ("xs:int(7) idiv xs:byte(2)", "3"),
        ("(xs:byte(127) + 1) instance of xs:byte", "false"),
        ("abs(xs:negativeInteger(-10))", "10"),
        ("round(xs:positiveInteger(1),-1)", "0"),
    ] {
        assert_eq!(evaluate(expr), want, "{expr}");
    }
}
#[test]
fn integer_subtypes_work_as_range_bounds_positions_codepoints_and_precisions() {
    for (expr, want) in [
        ("xs:byte(1) to xs:short(3)", "1 2 3"),
        ("remove((1,2,3),xs:byte(2))", "1 3"),
        ("subsequence((1,2,3),xs:unsignedByte(2))", "2 3"),
        ("codepoints-to-string(xs:unsignedByte(65))", "A"),
        ("round(1.234,xs:byte(2))", "1.23"),
        ("sum((xs:byte(1),xs:short(2)))", "3"),
        ("min((xs:byte(1),xs:short(2)))", "1"),
    ] {
        assert_eq!(evaluate(expr), want, "{expr}");
    }
}
#[test]
fn raw_typed_bindings_and_template_parameters_preserve_subtypes() {
    let body = r#"<xsl:template name="n" as="xs:int"><xsl:param name="n" as="xs:int"/><xsl:sequence select="$n"/></xsl:template><xsl:template match="/"><xsl:variable name="v" as="xs:int+"><xsl:sequence select="xs:byte(1)"/><xsl:copy-of select="xs:int(2)"/><xsl:call-template name="n"><xsl:with-param name="n" select="xs:short(3)"/></xsl:call-template></xsl:variable><xsl:value-of select="concat(count($v), ':', $v[1] instance of xs:byte, ':', $v[3] instance of xs:short)"/></xsl:template>"#;
    let style = source("1").replace(
        r#"<xsl:template match="/"><xsl:sequence select="1"/></xsl:template>"#,
        body,
    );
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "3:true:true"
    );
    let style = source("1").replace(
        r#"<xsl:sequence select="1"/>"#,
        r#"<xsl:variable name="n" as="xs:int" select="1"/><xsl:sequence select="$n"/>"#,
    );
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .transform("<r/>")
            .unwrap_err()
            .code,
        "XTTE0570"
    );
}
