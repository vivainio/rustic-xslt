use rust_xslt::Stylesheet;
fn run(body: &str, source: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:map="http://www.w3.org/2005/xpath-functions/map"><xsl:output method="text"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform(source)
}
const SOURCE: &str = "<r><i k='a'>1</i><i k='b'>2</i><i k='c'>3</i></r>";
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
fn perform_sort_orders_a_sequence_by_keys() {
    let select = r#"<xsl:value-of select="(for $i in r/i return string($i/@k))"/>|<xsl:variable name="s"><xsl:perform-sort select="r/i"><xsl:sort select="." order="descending" data-type="number"/></xsl:perform-sort></xsl:variable><xsl:variable name="t" as="element()*"><xsl:perform-sort select="r/i"><xsl:sort select="number(.)" order="descending"/></xsl:perform-sort></xsl:variable><xsl:value-of select="for $n in $t return string($n/@k)"/>"#;
    assert_eq!(run(select, SOURCE).unwrap(), "a b c|c b a");
    let body = r#"<xsl:variable name="t" as="xs:integer*" xmlns:xs="http://www.w3.org/2001/XMLSchema"><xsl:perform-sort><xsl:sort select="-."/><xsl:sequence select="(3, 1, 2)"/></xsl:perform-sort></xsl:variable><xsl:value-of select="$t"/>"#;
    assert_eq!(run(body, SOURCE).unwrap(), "3 2 1");
    assert!(run(r#"<xsl:perform-sort select="r/i"/>"#, SOURCE).is_err());
}
