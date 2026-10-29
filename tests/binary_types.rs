use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn sheet(expr: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#)).unwrap()
}
#[test]
fn binary_constructors_canonicalize_without_conflating_types() {
    for (expr, expected) in [
        ("xs:hexBinary(' 0aFf ')", "0AFF"),
        ("xs:base64Binary(' Y W J j &#xA; ')", "YWJj"),
        ("xs:hexBinary(xs:base64Binary('YWJj'))", "616263"),
        ("xs:base64Binary(xs:hexBinary('616263'))", "YWJj"),
        ("xs:base64Binary(xs:hexBinary('00'))", "AA=="),
        ("xs:base64Binary(xs:hexBinary('00FF'))", "AP8="),
        ("xs:base64Binary(xs:hexBinary('00FF01'))", "AP8B"),
        ("xs:hexBinary(xs:base64Binary('AP8B'))", "00FF01"),
        ("xs:string(xs:hexBinary(''))", ""),
        ("xs:hexBinary('') instance of xs:anyAtomicType", "true"),
        ("xs:hexBinary('') instance of xs:base64Binary", "false"),
        ("xs:base64Binary('') instance of xs:base64Binary", "true"),
        ("empty(xs:base64Binary(()))", "true"),
        ("xs:hexBinary(r/@hex)", "FF"),
    ] {
        assert_eq!(
            sheet(expr).transform("<r hex='ff'/>").unwrap(),
            expected,
            "{expr}"
        );
    }
}
#[test]
fn binary_lexical_validation_rejects_internal_hex_spaces_and_bad_padding() {
    for expr in [
        "xs:hexBinary('0')",
        "xs:hexBinary('0g')",
        "xs:hexBinary('00 FF')",
        "xs:base64Binary('A')",
        "xs:base64Binary('AAA')",
        "xs:base64Binary('=AAA')",
        "xs:base64Binary('AA=A')",
        "xs:base64Binary('AA==AAAA')",
        "xs:base64Binary('AA===')",
        "xs:base64Binary('AB==')",
        "xs:base64Binary('AAB=')",
        "xs:base64Binary('AA-_')",
        "xs:base64Binary('AAé=')",
    ] {
        assert_eq!(
            sheet(expr).transform("<r/>").unwrap_err().code,
            "FORG0001",
            "{expr}"
        );
    }
    for expr in [
        "xs:hexBinary(1)",
        "xs:base64Binary(true())",
        "xs:hexBinary(xs:anyURI('00'))",
        "xs:integer(xs:hexBinary('00'))",
        "xs:boolean(xs:hexBinary('00'))",
    ] {
        assert_eq!(
            sheet(expr).transform("<r/>").unwrap_err().code,
            "XPTY0004",
            "{expr}"
        );
    }
}
#[test]
fn binary_casts_comparisons_and_sequence_functions_use_octets() {
    for (expr, expected) in [
        ("'ff' cast as xs:hexBinary", "FF"),
        ("min((xs:hexBinary('FF'), xs:hexBinary('00')))", "00"),
        (
            "max((xs:base64Binary('AA=='), xs:base64Binary('/w==')))",
            "/w==",
        ),
        ("'AA==' castable as xs:base64Binary", "true"),
        ("'AB==' castable as xs:base64Binary", "false"),
        ("xs:base64Binary('AA==') cast as xs:hexBinary", "00"),
        ("xs:hexBinary('00ff') eq xs:hexBinary('00FF')", "true"),
        ("xs:hexBinary('00') lt xs:hexBinary('0000')", "true"),
        ("xs:base64Binary('AA==') lt xs:base64Binary('/w==')", "true"),
        (
            "deep-equal(xs:hexBinary('00'),xs:base64Binary('AA=='))",
            "false",
        ),
        (
            "count(distinct-values((xs:hexBinary('00'),xs:hexBinary('00'),xs:base64Binary('AA=='))))",
            "2",
        ),
        (
            "index-of((xs:hexBinary('00'),xs:hexBinary('FF')),xs:hexBinary('ff'))",
            "2",
        ),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
    assert_eq!(
        sheet("xs:hexBinary('00') eq xs:base64Binary('AA==')")
            .transform("<r/>")
            .unwrap_err()
            .code,
        "XPTY0004"
    );
    assert_eq!(
        sheet("boolean(xs:hexBinary(''))")
            .transform("<r/>")
            .unwrap_err()
            .code,
        "FORG0006"
    );
}
#[test]
fn binary_binding_conversion_and_raw_template_results_preserve_values() {
    let s = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template name="binary" as="xs:hexBinary"><xsl:param name="v" as="xs:hexBinary"/><xsl:sequence select="$v"/></xsl:template><xsl:template match="/"><xsl:variable name="v" as="xs:hexBinary"><xsl:call-template name="binary"><xsl:with-param name="v" select="r/@hex"/></xsl:call-template></xsl:variable><xsl:sequence select="$v instance of xs:hexBinary"/><xsl:sequence select="$v"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(s)
            .unwrap()
            .transform("<r hex='ff'/>")
            .unwrap(),
        "true FF"
    );
}
#[test]
fn binary_decoding_charges_lexical_input_to_the_execution_budget() {
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 30,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet("xs:hexBinary(r/@hex)")
            .transform_with_options(&format!("<r hex='{}'/>", "00".repeat(100)), &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}
