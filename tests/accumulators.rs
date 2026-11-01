use rust_xslt::Stylesheet;

fn run(declarations: &str, expression: &str) -> Result<String, rust_xslt::Diagnostic> {
    let sheet = format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" xmlns:map="http://www.w3.org/2005/xpath-functions/map"><xsl:output method="text"/>{declarations}<xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    );
    Stylesheet::compile(&sheet)?.transform("<root><n><n/></n><n/></root>")
}

#[test]
fn counts_start_and_end_events_in_document_order() {
    let declarations = r#"<xsl:mode use-accumulators="a:count"/><xsl:accumulator name="a:count" as="xs:integer" initial-value="0"><xsl:accumulator-rule match="n" select="$value + 1"/><xsl:accumulator-rule match="n" phase="end" select="$value + 10"/></xsl:accumulator>"#;
    assert_eq!(
        run(
            declarations,
            "root/n ! concat(accumulator-before('a:count'), ':', accumulator-after('a:count'))"
        )
        .unwrap(),
        "1:22 23:33"
    );
    assert_eq!(
        run(declarations, "root/n/n/accumulator-before('a:count')").unwrap(),
        "2"
    );
    assert_eq!(
        run(declarations, "root/accumulator-after('a:count')").unwrap(),
        "33"
    );
}

#[test]
fn applies_only_accumulators_selected_for_the_initial_mode() {
    let accumulator = r#"<xsl:accumulator name="count" initial-value="0"><xsl:accumulator-rule match="n" select="$value + 1"/></xsl:accumulator>"#;
    assert_eq!(
        run(accumulator, "root/accumulator-after('count')")
            .unwrap_err()
            .code,
        "XTDE3362"
    );
    let declarations = format!(r##"<xsl:mode use-accumulators="#all"/>{accumulator}"##);
    assert_eq!(
        run(&declarations, "root/accumulator-after('count')").unwrap(),
        "3"
    );
    assert_eq!(
        run(&declarations, "root/accumulator-before('missing')")
            .unwrap_err()
            .code,
        "XTDE3340"
    );
}

#[test]
fn accumulator_values_can_be_maps_and_rules_use_globals() {
    let declarations = r#"<xsl:mode use-accumulators="seen"/><xsl:variable name="suffix" select="'!'"/><xsl:accumulator name="seen" as="map(*)" initial-value="map{}"><xsl:accumulator-rule match="n" select="map:put($value, concat(generate-id(.), $suffix), true())" xmlns:map="http://www.w3.org/2005/xpath-functions/map"/></xsl:accumulator>"#;
    assert_eq!(
        run(declarations, "map:size(root/accumulator-after('seen'))").unwrap(),
        "3"
    );
}

#[test]
fn later_rule_wins_even_if_the_earlier_pattern_is_more_specific() {
    let declarations = r#"<xsl:mode use-accumulators="a"/><xsl:accumulator name="a" initial-value="0"><xsl:accumulator-rule match="n" select="$value + 1"/><xsl:accumulator-rule match="*" select="$value + 10"/></xsl:accumulator>"#;
    assert_eq!(
        run(declarations, "root/accumulator-after('a')").unwrap(),
        "40"
    );
}

#[test]
fn accumulator_function_references_resolve_prefixes_at_their_declaration() {
    let declarations = r#"<xsl:mode use-accumulators="a:count"/><xsl:accumulator name="a:count" initial-value="0"><xsl:accumulator-rule match="n" select="$value + 1"/></xsl:accumulator>"#;
    assert_eq!(
        run(declarations, "root/(accumulator-after#1)('a:count')").unwrap(),
        "3"
    );
    assert_eq!(
        run(declarations, "root/(accumulator-after(?))('a:count')").unwrap(),
        "3"
    );
}

#[test]
fn accumulator_dependency_cycles_are_reported() {
    let declarations = r#"<xsl:mode use-accumulators="a"/><xsl:accumulator name="a" initial-value="0"><xsl:accumulator-rule match="n" select="accumulator-before('a')"/></xsl:accumulator>"#;
    assert_eq!(
        run(declarations, "root/n[1]/accumulator-before('a')")
            .unwrap_err()
            .code,
        "XTDE3400"
    );
}

#[test]
fn record_streaming_rejects_accumulator_stylesheets() {
    use rust_xslt::StreamOptions;
    use std::io::Cursor;
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:accumulator name="a" initial-value="0"><xsl:accumulator-rule match="n" select="$value + 1"/></xsl:accumulator><xsl:template name="record"/></xsl:stylesheet>"#;
    let stylesheet = Stylesheet::compile(source).unwrap();
    let err = stylesheet
        .stream_records(
            Cursor::new("<root><n/></root>"),
            &mut Vec::new(),
            &StreamOptions::default(),
        )
        .unwrap_err();
    assert_eq!(err.code, "RUST_XSLT_UNSUPPORTED");
}

#[test]
fn accumulator_rules_apply_to_temporary_trees_without_initial_mode_selection() {
    let declarations = r#"<xsl:accumulator name="count" initial-value="0"><xsl:accumulator-rule match="n" select="$value + 1"/></xsl:accumulator><xsl:variable name="tree"><r><n/><n/></r></xsl:variable>"#;
    assert_eq!(
        run(declarations, "$tree/r/accumulator-after('count')").unwrap(),
        "2"
    );
}

#[test]
fn accumulator_evaluation_is_lazy_and_initial_value_sees_the_root() {
    let declarations = r#"<xsl:mode use-accumulators="a"/><xsl:accumulator name="a" initial-value="count(root/n)"><xsl:accumulator-rule match="n" select="$value + 1"/></xsl:accumulator>"#;
    assert_eq!(
        run(declarations, "root/accumulator-after('a')").unwrap(),
        "5"
    );
    let unused = r#"<xsl:accumulator name="a" initial-value="error()"><xsl:accumulator-rule match="n" select="$value"/></xsl:accumulator>"#;
    assert_eq!(run(unused, "'ok'").unwrap(), "ok");
}
