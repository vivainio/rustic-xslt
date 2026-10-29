use rust_xslt::Stylesheet;
fn sheet(expression: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#)).unwrap()
}
#[test]
fn exact_numeric_aggregates_preserve_values_and_result_types() {
    for (expr, expected) in [
        ("sum((0.1,0.2,0.3))", "0.6"),
        ("avg((1,2))", "1.5"),
        (
            "min((999999999999999999999,1000000000000000000000))",
            "999999999999999999999",
        ),
        ("max((0.01,0.1))", "0.1"),
        ("avg((1,2)) instance of xs:decimal", "true"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected);
    }
}
#[test]
fn numeric_aggregate_promotion_applies_to_all_inputs() {
    for expr in [
        "min((1,xs:double('2'))) instance of xs:double",
        "max((1,xs:float('0'))) instance of xs:float",
        "sum((1,xs:float('2'))) instance of xs:float",
        "avg((xs:float('1'),xs:float('2'))) instance of xs:float",
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), "true");
    }
}
#[test]
fn aggregate_empty_values_and_supplied_sum_zero_are_distinct() {
    for expr in ["min(())", "max(())", "avg(())", "sum((),())"] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), "");
    }
    assert_eq!(sheet("sum(())").transform("<r/>").unwrap(), "0");
    assert_eq!(sheet("sum((),'empty')").transform("<r/>").unwrap(), "empty");
    assert_eq!(
        sheet("sum((1,2),'ignored')").transform("<r/>").unwrap(),
        "3"
    );
}
#[test]
fn node_values_atomize_and_untyped_values_become_doubles() {
    assert_eq!(
        sheet("avg(r/n)")
            .transform("<r><n>1</n><n>2</n></r>")
            .unwrap(),
        "1.5"
    );
    assert_eq!(
        sheet("min(r/n) instance of xs:double")
            .transform("<r><n>1</n></r>")
            .unwrap(),
        "true"
    );
    assert_eq!(
        sheet("sum(r/n)")
            .transform("<r><n>bad</n></r>")
            .unwrap_err()
            .code,
        "FORG0001"
    );
}
#[test]
fn min_max_support_codepoint_strings_and_boolean_ordering() {
    assert_eq!(sheet("min(('z','a','é'))").transform("<r/>").unwrap(), "a");
    assert_eq!(sheet("max(('z','a','é'))").transform("<r/>").unwrap(), "é");
    assert_eq!(
        sheet("min((true(),false()))").transform("<r/>").unwrap(),
        "false"
    );
    assert_eq!(
        sheet("max((true(),false()))").transform("<r/>").unwrap(),
        "true"
    );
}
#[test]
fn incomparable_aggregate_types_and_unknown_collations_fail() {
    for expr in [
        "sum(('1','2'))",
        "avg((true(),false()))",
        "min((1,'a'))",
        "max((true(),'a'))",
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap_err().code, "FORG0006");
    }
    assert_eq!(
        sheet("min(('a','b'),'urn:unknown')")
            .transform("<r/>")
            .unwrap_err()
            .code,
        "FOCH0002"
    );
}
#[test]
fn nan_survives_min_max_and_common_numeric_promotion() {
    for expr in [
        "min((1,xs:float('NaN'),xs:double('2')))",
        "max((xs:double('NaN'),1))",
        "avg((1,xs:double('NaN')))",
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), "NaN");
    }
}

#[test]
fn min_max_retain_selected_integer_restriction_types_without_primitive_promotion() {
    for (expr, expected) in [
        ("min((xs:byte(1),xs:int(2))) instance of xs:byte", "true"),
        ("max((xs:byte(1),xs:int(2))) instance of xs:int", "true"),
        ("min((xs:byte(1),2.0)) instance of xs:byte", "true"),
        ("max((1.0,xs:short(2))) instance of xs:short", "true"),
        ("min((xs:byte(1),xs:float(2))) instance of xs:float", "true"),
        ("max((xs:byte(2),1e0)) instance of xs:double", "true"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
}
#[test]
fn min_max_retain_uri_type_unless_mixed_with_string_values() {
    for (expr, expected) in [
        (
            "min((xs:anyURI('a'),xs:anyURI('b'))) instance of xs:anyURI",
            "true",
        ),
        (
            "max((xs:anyURI('a'),xs:anyURI('b'))) instance of xs:anyURI",
            "true",
        ),
        ("min((xs:anyURI('a'),'b')) instance of xs:string", "true"),
        ("max(('a',xs:anyURI('b'))) instance of xs:string", "true"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
}
