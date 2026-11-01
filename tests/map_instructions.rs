use rust_xslt::Stylesheet;
fn run(body: &str, source: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:map="http://www.w3.org/2005/xpath-functions/map"><xsl:output method="text"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform(source)
}
const SOURCE: &str = "<r><i k='a'>1</i><i k='b'>2</i><i k='c'>3</i></r>";
#[test]
fn sequence_constructor_can_supply_map_entries() {
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:sequence><xsl:fallback><bad/></xsl:fallback><xsl:map-entry key="'k'" select="'v'"/></xsl:sequence></xsl:map></xsl:variable><xsl:value-of select="$m?k"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "v");
}

#[test]
fn map_entry_is_a_standalone_instruction_and_accepts_pure_constructors() {
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry key="'k'"><xsl:variable name="v" select="2"/><xsl:sequence select="$v"/><xsl:if test="true()"><xsl:sequence select="$v + 1"/></xsl:if></xsl:map-entry></xsl:variable><xsl:value-of select="$m?k"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "2 3");
    let fallback = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry key="'k'" select="'v'"><xsl:fallback><bad/></xsl:fallback></xsl:map-entry></xsl:variable><xsl:value-of select="$m?k"/>"#;
    assert_eq!(run(fallback, SOURCE).unwrap(), "v");
    let invalid = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry key="'k'" select="1"><xsl:sequence select="2"/></xsl:map-entry></xsl:variable>"#;
    assert_eq!(run(invalid, SOURCE).unwrap_err().code, "XTSE3280");
    let nested = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:map-entry key="'k'" select="1"><xsl:sequence select="2"/></xsl:map-entry></xsl:map></xsl:variable>"#;
    assert_eq!(run(nested, SOURCE).unwrap_err().code, "XTSE3280");
}

#[test]
fn standalone_map_entry_retains_node_and_atomic_values() {
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry key="'k'"><node><xsl:value-of select="r/i[1]"/></node><xsl:sequence select="(2, 3)"/></xsl:map-entry></xsl:variable><xsl:value-of select="concat($m?k[1]/name(), ':', $m?k[1], ':', sum($m?k[position() gt 1]))"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "node:1:5");
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry key="'k'"><xsl:try><item>ok</item><xsl:catch><bad/></xsl:catch></xsl:try></xsl:map-entry></xsl:variable><xsl:value-of select="$m?k/name()"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "item");
}

#[test]
fn map_constructor_retains_constructed_entry_values() {
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:map-entry key="'node'"><node>value</node></xsl:map-entry><xsl:map-entry key="'number'" select="7"/></xsl:map></xsl:variable><xsl:value-of select="concat($m?node/name(), ':', $m?node, ':', $m?number)"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "node:value:7");
    let duplicate = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:map-entry key="'k'"><node/></xsl:map-entry><xsl:map-entry key="'k'" select="1"/></xsl:map></xsl:variable>"#;
    assert_eq!(run(duplicate, SOURCE).unwrap_err().code, "XTDE3365");
    let sorted = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:for-each select="r/i"><xsl:sort select="@k" order="descending"/><xsl:map-entry key="string(@k)" select="string(.)"/></xsl:for-each></xsl:map></xsl:variable><xsl:value-of select="map:keys($m)"/>"#;
    assert_eq!(run(sorted, SOURCE).unwrap(), "c b a");
}

#[test]
fn map_constructor_accepts_general_sequence_constructors() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template name="entry"><xsl:map-entry key="'k'"><value>ok</value></xsl:map-entry></xsl:template><xsl:template match="/"><xsl:variable name="m" as="map(*)"><xsl:map><xsl:fallback><bad/></xsl:fallback><xsl:call-template name="entry"/><xsl:try><xsl:sequence select="error()"/><xsl:catch><xsl:map-entry key="'n'" select="2"/></xsl:catch></xsl:try></xsl:map></xsl:variable><xsl:value-of select="concat($m?k, ':', $m?n)"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform(SOURCE)
            .unwrap(),
        "ok:2"
    );
    let duplicate = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:map-entry key="1" select="1"/><xsl:map-entry key="1" select="2"/></xsl:map></xsl:variable>"#;
    assert_eq!(run(duplicate, SOURCE).unwrap_err().code, "XTDE3365");
    let non_map = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:if test="true()"><bad/></xsl:if></xsl:map></xsl:variable>"#;
    assert_eq!(run(non_map, SOURCE).unwrap_err().code, "XTTE3375");
}

#[test]
fn expression_lowered_stylesheet_functions_can_construct_maps() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:function name="f:m" as="map(*)"><xsl:param name="v"/><xsl:map><xsl:map-entry key="'k'" select="$v"/></xsl:map></xsl:function><xsl:template match="/"><xsl:value-of select="f:m(7)?k"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform(SOURCE)
            .unwrap(),
        "7"
    );
}

#[test]
fn stylesheet_function_maps_use_xslt_error_codes() {
    let source = |content: &str| {
        format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:function name="f:m" as="map(*)"><xsl:map>{content}</xsl:map></xsl:function><xsl:template match="/"><xsl:value-of select="f:m()?k"/></xsl:template></xsl:stylesheet>"#
        )
    };
    let duplicate =
        source(r#"<xsl:map-entry key="'k'" select="1"/><xsl:map-entry key="'k'" select="2"/>"#);
    assert_eq!(
        Stylesheet::compile(&duplicate)
            .unwrap()
            .transform(SOURCE)
            .unwrap_err()
            .code,
        "XTDE3365"
    );
    let non_map = source(r#"<xsl:sequence select="1"/>"#);
    assert_eq!(
        Stylesheet::compile(&non_map)
            .unwrap()
            .transform(SOURCE)
            .unwrap_err()
            .code,
        "XTTE3375"
    );
    let ordinary_xpath = r#"<xsl:value-of select="map:merge((map{'k':1}, map{'k':2}), map{'duplicates':'reject'})?k"/>"#;
    assert_eq!(run(ordinary_xpath, SOURCE).unwrap_err().code, "FOJS0003");
}

#[test]
fn map_rejects_nonstandard_unqualified_attributes() {
    for attribute in ["use-attribute-sets='missing'", "on-duplicates='use-first'"] {
        let body =
            format!(r#"<xsl:variable name="m" as="map(*)"><xsl:map {attribute}/></xsl:variable>"#);
        assert_eq!(run(&body, SOURCE).unwrap_err().code, "XTSE0090");
    }
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map xmlns:e="urn:extension" e:use-attribute-sets="ignored"/></xsl:variable><xsl:value-of select="map:size($m)"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "0");
}

#[test]
fn map_entry_ignores_namespaced_key_and_select_attributes() {
    let missing = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry xmlns:e="urn:e" e:key="'wrong'" select="1"/></xsl:variable>"#;
    assert_eq!(run(missing, SOURCE).unwrap_err().code, "XTSE0010");
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry xmlns:e="urn:e" key="'k'" e:select="'wrong'"><xsl:sequence select="'right'"/></xsl:map-entry></xsl:variable><xsl:value-of select="$m?k"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "right");
    let both = r#"<xsl:variable name="m" as="map(*)"><xsl:map-entry xmlns:e="urn:e" e:select="'wrong'" key="'k'" select="'right'"/></xsl:variable><xsl:value-of select="$m?k"/>"#;
    assert_eq!(run(both, SOURCE).unwrap(), "right");
    let required = r#"<xsl:if xmlns:e="urn:e" e:test="true()"/>"#;
    assert_eq!(run(required, SOURCE).unwrap_err().code, "XTSE0010");
}
#[test]
fn xsl_map_builds_maps_from_entries_and_control_flow() {
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:map-entry key="'x'" select="1"/><xsl:map-entry key="'y'"><xsl:sequence select="(2, 3)"/></xsl:map-entry></xsl:map></xsl:variable><xsl:value-of select="map:size($m), $m?x, $m?y"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "2 1 2 3");
    let looped = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:for-each select="r/i"><xsl:map-entry key="string(@k)" select="number(.)"/></xsl:for-each></xsl:map></xsl:variable><xsl:value-of select="map:keys($m), sum(map:keys($m) ! $m(.))"/>"#;
    assert_eq!(run(looped, SOURCE).unwrap(), "a b c 6");
    let conditional = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:variable name="base" select="10"/><xsl:if test="$base gt 5"><xsl:map-entry key="'big'" select="$base"/></xsl:if><xsl:choose><xsl:when test="false()"><xsl:map-entry key="'w'" select="1"/></xsl:when><xsl:otherwise><xsl:map-entry key="'o'" select="2"/></xsl:otherwise></xsl:choose><xsl:sequence select="map { 'z': 0 }"/></xsl:map></xsl:variable><xsl:value-of select="map:keys($m)"/>"#;
    assert_eq!(run(conditional, SOURCE).unwrap(), "big o z");
    let empty = r#"<xsl:variable name="m" as="map(*)"><xsl:map/></xsl:variable><xsl:value-of select="map:size($m)"/>"#;
    assert_eq!(run(empty, SOURCE).unwrap(), "0");
}
#[test]
fn duplicate_keys_and_misplaced_entries_are_errors() {
    let duplicate = r#"<xsl:variable name="m" as="map(*)"><xsl:map><xsl:map-entry key="1" select="1"/><xsl:map-entry key="1" select="2"/></xsl:map></xsl:variable><xsl:value-of select="map:size($m)"/>"#;
    assert!(run(duplicate, SOURCE).is_err());
    assert!(run(r#"<xsl:map-entry key="1" select="1"/>"#, SOURCE).is_err());
    assert!(
        run(
            r#"<xsl:variable name="m" as="map(*)"><xsl:map><out/></xsl:map></xsl:variable>"#,
            SOURCE
        )
        .is_err()
    );
}

#[test]
fn choose_structure_and_required_tests_are_checked_in_all_constructors() {
    for choose in [
        "<xsl:choose/>",
        "<xsl:choose><xsl:when/></xsl:choose>",
        "<xsl:choose><xsl:otherwise/><xsl:when test='true()'/></xsl:choose>",
        "<xsl:choose><xsl:when test='true()'/><xsl:otherwise/><xsl:otherwise/></xsl:choose>",
        "<xsl:choose>text<xsl:when test='true()'/></xsl:choose>",
        "<xsl:choose><when test='true()'/></xsl:choose>",
    ] {
        for body in [choose.to_owned(), format!("<xsl:map>{choose}</xsl:map>")] {
            assert_eq!(run(&body, SOURCE).unwrap_err().code, "XTSE0010", "{body}");
        }
    }
    assert_eq!(run("<xsl:if/>", SOURCE).unwrap_err().code, "XTSE0010");
}
#[test]
fn perform_sort_orders_a_sequence_by_keys() {
    let select = r#"<xsl:value-of select="(for $i in r/i return string($i/@k))"/>|<xsl:variable name="s"><xsl:perform-sort select="r/i"><xsl:sort select="." order="descending" data-type="number"/></xsl:perform-sort></xsl:variable><xsl:variable name="t" as="element()*"><xsl:perform-sort select="r/i"><xsl:sort select="number(.)" order="descending"/></xsl:perform-sort></xsl:variable><xsl:value-of select="for $n in $t return string($n/@k)"/>"#;
    assert_eq!(run(select, SOURCE).unwrap(), "a b c|c b a");
    let body = r#"<xsl:variable name="t" as="xs:integer*" xmlns:xs="http://www.w3.org/2001/XMLSchema"><xsl:perform-sort><xsl:sort select="-." data-type="number"/><xsl:sequence select="(3, 1, 2)"/></xsl:perform-sort></xsl:variable><xsl:value-of select="$t"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "3 2 1");
    assert!(run(r#"<xsl:perform-sort select="r/i"/>"#, SOURCE).is_err());
}
