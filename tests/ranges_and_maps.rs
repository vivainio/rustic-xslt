use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn sheet(expression: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#)).unwrap()
}
#[test]
fn ranges_keep_arbitrary_precision_integer_values() {
    assert_eq!(sheet("-2 to 2").transform("<r/>").unwrap(), "-2 -1 0 1 2");
    assert_eq!(
        sheet("999999999999999999999 to 1000000000000000000001")
            .transform("<r/>")
            .unwrap(),
        "999999999999999999999 1000000000000000000000 1000000000000000000001"
    );
}
#[test]
fn optional_and_descending_ranges_are_empty() {
    for expression in ["() to 3", "1 to ()", "3 to 1"] {
        assert_eq!(sheet(expression).transform("<r/>").unwrap(), "");
    }
    assert_eq!(
        sheet("(2 to 2) instance of xs:integer")
            .transform("<r/>")
            .unwrap(),
        "true"
    );
}
#[test]
fn range_bounds_atomize_untyped_nodes_but_reject_wrong_types() {
    assert_eq!(
        sheet("r/@low to r/@high")
            .transform("<r low='2' high='4'/>")
            .unwrap(),
        "2 3 4"
    );
    for expression in ["1.0 to 2", "'1' to 2", "true() to 2", "(1,2) to 3"] {
        assert_eq!(
            sheet(expression).transform("<r/>").unwrap_err().code,
            "XPTY0004"
        );
    }
    assert_eq!(
        sheet("r/@low to 3")
            .transform("<r low='bad'/>")
            .unwrap_err()
            .code,
        "FORG0001"
    );
}
#[test]
fn range_precedence_and_nonassociativity_follow_the_grammar() {
    assert_eq!(sheet("1 + 1 to 2 * 2").transform("<r/>").unwrap(), "2 3 4");
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:sequence select="1 to 2 to 3"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(Stylesheet::compile(source).unwrap_err().code, "XPST0003");
}
#[test]
fn simple_maps_use_each_input_items_focus_and_keep_duplicates() {
    assert_eq!(
        sheet("(3,1,3) ! concat(., ':', position(), '/', last())")
            .transform("<r/>")
            .unwrap(),
        "3:1/3 1:2/3 3:3/3"
    );
    assert_eq!(
        sheet("(r/n, r/n) ! .")
            .transform("<r><n>A</n></r>")
            .unwrap(),
        "AA"
    );
}
#[test]
fn simple_map_chains_and_operator_precedence_are_distinct() {
    assert_eq!(
        sheet("1 to 3 ! (. * 2)").transform("<r/>").unwrap(),
        "1 2 3 4 5 6"
    );
    assert_eq!(
        sheet("(1 to 3) ! (. * 2) ! (. + 1)")
            .transform("<r/>")
            .unwrap(),
        "3 5 7"
    );
    assert_eq!(
        sheet("-(1,2) ! (.)").transform("<r/>").unwrap_err().code,
        "XPTY0004"
    );
}
#[test]
fn simple_maps_are_lazy_for_empty_inputs_and_keep_lexical_variables() {
    assert_eq!(sheet("() ! (1 div 0)").transform("<r/>").unwrap(), "");
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:variable name="v" select="10"/><xsl:sequence select="(1,2) ! ($v + .)"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "11 12"
    );
}
#[test]
fn large_ranges_and_map_work_share_the_execution_budget() {
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 50,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet("1 to 999999999999999999999999999")
            .transform_with_options("<r/>", &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
    assert_eq!(
        sheet("(1 to 10) ! (1 to 10)")
            .transform_with_options("<r/>", &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}
