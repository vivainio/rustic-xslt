use rust_xslt::Stylesheet;
const SOURCE: &str = "<r><i k='a'>1</i><i k='b'>2</i><i k='a'>3</i><i k='a'>4</i><i k='c'>5</i><h>H1</h><i k='x'>6</i><h>H2</h><i k='y'>7</i></r>";
fn run(instruction: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/">{instruction}</xsl:template></xsl:stylesheet>"#))?
        .transform(SOURCE)
}
#[test]
fn group_by_collects_items_in_first_appearance_order() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k">[<xsl:value-of select="current-grouping-key()"/>:<xsl:value-of select="current-group()" separator=","/>]</xsl:for-each-group>"#).unwrap(),
        "[a:1,3,4][b:2][c:5][x:6][y:7]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="1 to 6" group-by="(., . + 1)[. le 6]">[<xsl:value-of select="current-grouping-key()"/>=<xsl:value-of select="count(current-group())"/>]</xsl:for-each-group>"#).unwrap(),
        "[1=1][2=2][3=2][4=2][5=2][6=2]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:value-of select="position()"/>/<xsl:value-of select="last()"/>;</xsl:for-each-group>"#).unwrap(),
        "1/5;2/5;3/5;4/5;5/5;"
    );
}
#[test]
fn adjacent_grouping_only_merges_consecutive_keys() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-adjacent="@k">[<xsl:value-of select="current-grouping-key()"/>:<xsl:value-of select="current-group()" separator=","/>]</xsl:for-each-group>"#).unwrap(),
        "[a:1][b:2][a:3,4][c:5][x:6][y:7]"
    );
    assert!(run(r#"<xsl:for-each-group select="r/i" group-adjacent="(1,2)"/>"#).is_err());
}
#[test]
fn starting_and_ending_patterns_split_node_sequences() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-starting-with="h">[<xsl:value-of select="count(current-group())"/>]</xsl:for-each-group>"#).unwrap(),
        "[5][2][2]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-ending-with="h">[<xsl:value-of select="count(current-group())"/>]</xsl:for-each-group>"#).unwrap(),
        "[6][2][1]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-starting-with="h"><xsl:value-of select="name(current-group()[1])"/>;</xsl:for-each-group>"#).unwrap(),
        "i;h;h;"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-starting-with="h"><xsl:value-of select="current-grouping-key()"/></xsl:for-each-group>"#).unwrap_err().code,
        "XTDE1071"
    );
    assert_eq!(
        run("<xsl:for-each-group select='1 to 3' group-starting-with='h'/>")
            .unwrap_err()
            .code,
        "XTTE1120"
    );
}
#[test]
fn grouping_declaration_errors() {
    assert_eq!(
        run("<xsl:for-each-group select='r/i'/>").unwrap_err().code,
        "XTSE1080"
    );
    assert_eq!(
        run("<xsl:for-each-group select='r/i' group-by='1' group-adjacent='2'/>")
            .unwrap_err()
            .code,
        "XTSE1080"
    );
    assert_eq!(
        run("<xsl:value-of select='current-group()'/>")
            .unwrap_err()
            .code,
        "XTDE1061"
    );
}
#[test]
fn groups_can_be_sorted_by_keys_that_see_the_group() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:sort select="current-grouping-key()" order="descending"/>[<xsl:value-of select="current-grouping-key()"/>]</xsl:for-each-group>"#).unwrap(),
        "[y][x][c][b][a]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:sort select="count(current-group())" data-type="number" order="descending"/><xsl:sort select="current-grouping-key()"/>[<xsl:value-of select="current-grouping-key()"/>]</xsl:for-each-group>"#).unwrap(),
        "[a][b][c][x][y]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:sort select="current-grouping-key()" order="descending"/><xsl:value-of select="position()"/>,</xsl:for-each-group>"#).unwrap(),
        "1,2,3,4,5,"
    );
}
