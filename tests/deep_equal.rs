use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn sheet(expression: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#)).unwrap()
}
#[test]
fn atomic_deep_equality_handles_promotion_nan_and_type_classes() {
    for (expression, expected) in [
        ("deep-equal((1,2), (1.0,xs:double('2')))", "true"),
        ("deep-equal(xs:float('NaN'), xs:double('NaN'))", "true"),
        ("deep-equal(xs:untypedAtomic('x'), 'x')", "true"),
        ("deep-equal(1, '1')", "false"),
        ("deep-equal((),())", "true"),
        ("deep-equal((1,2),(2,1))", "false"),
        ("deep-equal((1,2),1)", "false"),
    ] {
        assert_eq!(sheet(expression).transform("<r/>").unwrap(), expected);
    }
}
#[test]
fn node_deep_equality_ignores_prefixes_and_attribute_order() {
    let xml = "<r><p:a xmlns:p='urn:p' x='1' y='2'><p:b>text</p:b></p:a><q:a xmlns:q='urn:p' y='2' x='1'><q:b>text</q:b></q:a></r>";
    assert_eq!(
        sheet("deep-equal(r/*[1], r/*[2])").transform(xml).unwrap(),
        "true"
    );
    assert_eq!(
        sheet("deep-equal(r/*[1], r/*[2])")
            .transform(&xml.replace("y='2' x='1'", "y='3' x='1'"))
            .unwrap(),
        "false"
    );
}
#[test]
fn child_comments_and_pis_are_ignored_but_direct_items_are_compared() {
    let xml = "<r><a>x<!--one--><?p one?><n/></a><a>x<!--two--><?p two?><n/></a></r>";
    assert_eq!(
        sheet("deep-equal(r/a[1], r/a[2])").transform(xml).unwrap(),
        "true"
    );
    assert_eq!(
        sheet("deep-equal(r/a[1]/comment(), r/a[2]/comment())")
            .transform(xml)
            .unwrap(),
        "false"
    );
    assert_eq!(
        sheet("deep-equal(r/a[1]/processing-instruction(), r/a[2]/processing-instruction())")
            .transform(xml)
            .unwrap(),
        "false"
    );
}
#[test]
fn node_kinds_and_expanded_names_remain_distinct() {
    for expr in [
        "deep-equal(r/a, r/b)",
        "deep-equal(r/@v, r/a/text())",
        "deep-equal(r/a, 'text')",
    ] {
        assert_eq!(
            sheet(expr)
                .transform("<r v='text'><a>text</a><b>text</b></r>")
                .unwrap(),
            "false"
        );
    }
}
#[test]
fn independent_temporary_documents_compare_by_structure() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="a"><r><n/></r></xsl:variable><xsl:variable name="b"><r><n/></r></xsl:variable><xsl:template match="/"><xsl:value-of select="deep-equal($a,$b) and not($a is $b)"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "true"
    );
}
#[test]
fn deep_equality_validates_collation_arguments() {
    assert_eq!(
        sheet("deep-equal(1,1,'urn:unknown')")
            .transform("<r/>")
            .unwrap_err()
            .code,
        "FOCH0002"
    );
    assert_eq!(
        sheet("deep-equal(1,1,())")
            .transform("<r/>")
            .unwrap_err()
            .code,
        "XPTY0004"
    );
}
#[test]
fn deep_equality_tree_traversal_consumes_shared_steps() {
    let xml = format!(
        "<r><a>{}</a><a>{}</a></r>",
        "<n/>".repeat(100),
        "<n/>".repeat(100)
    );
    let s = sheet("deep-equal(r/a[1],r/a[2])");
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options(&xml, &opts).unwrap_err().code,
        "XPDY0130"
    );
}
