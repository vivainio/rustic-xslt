use rust_xslt::{Limits, Stylesheet, transform};
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output omit-xml-declaration="yes"/>{body}</xsl:stylesheet>"##
    )
}
#[test]
fn comment_body_atomizes_direct_nodes_and_joins_with_spaces() {
    let s = sheet(
        r##"<xsl:template match="/r"><out><xsl:comment><xsl:copy-of select="node()"/></xsl:comment></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r><a>A</a><b>B</b><!--C--><?pi D?></r>").unwrap(),
        "<out><!--A B C D--></out>"
    );
}

#[test]
fn value_of_constructs_simple_content_and_preserves_empty_text_items() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:value-of>left<xsl:sequence select="(1, 2)"/></xsl:value-of>|<xsl:value-of separator="|">left<xsl:sequence select="(1, 2)"/></xsl:value-of></out></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "<out>left12|left|1|2</out>");
    let s = sheet(
        r##"<xsl:template match="/"><xsl:variable name="v" as="node()*"><xsl:value-of/></xsl:variable><out count="{count($v)}" empty="{string-length($v)}"><xsl:where-populated><xsl:value-of/></xsl:where-populated></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        r#"<out count="1" empty="0"></out>"#
    );
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:value-of xmlns:e="urn:e" e:select="'wrong'">right</xsl:value-of></out></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "<out>right</out>");
    let invalid = sheet(
        r##"<xsl:template match="/"><xsl:value-of select="1">bad</xsl:value-of></xsl:template>"##,
    );
    assert_eq!(Stylesheet::compile(&invalid).unwrap_err().code, "XTSE0870");
}
#[test]
fn text_nodes_merge_but_element_boundaries_are_preserved() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:comment><xsl:text>A</xsl:text><xsl:text>B</xsl:text><empty/><a>C<!--ignored--><?pi ignored?><b>D</b></a><xsl:text>E</xsl:text></xsl:comment></out></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "<out><!--AB  CD E--></out>");
}
#[test]
fn document_item_is_atomized_as_one_item_instead_of_flattening() {
    let s = sheet(
        r##"<xsl:variable name="tree"><a>A</a><b>B</b></xsl:variable><xsl:template match="/"><out><xsl:comment><xsl:text>prefix</xsl:text><xsl:copy-of select="$tree"/></xsl:comment></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        "<out><!--prefix AB--></out>"
    );
}
#[test]
fn attribute_body_separator_applies_to_atoms_and_merges_text_nodes() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:attribute name="value" separator="|"><xsl:text>A</xsl:text><xsl:text>B</xsl:text><a>C</a><xsl:sequence select="(1,2)"/><xsl:attribute name="nested">D</xsl:attribute></xsl:attribute></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        "<out value=\"AB|C|1|2|D\"></out>"
    );
}
#[test]
fn default_attribute_body_separator_is_empty() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:attribute name="value"><xsl:sequence select="(1,2)"/><a>A</a><b>B</b></xsl:attribute></out></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "<out value=\"12AB\"></out>");
}
#[test]
fn value_of_select_merges_adjacent_text_nodes_before_atomization() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:value-of select="r/a/text() | r/b/text()" separator="|"/></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r><a>A</a><b>B</b></r>").unwrap(),
        "<out>AB</out>"
    );
}
#[test]
fn adjacent_atomics_across_instructions_have_one_space_but_text_breaks_the_run() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:sequence select="1"/><xsl:sequence select="2"/><xsl:text>-</xsl:text><xsl:sequence select="3"/><xsl:copy select="4"/><empty/><xsl:sequence select="5"/><xsl:sequence select="''"/><xsl:sequence select="6"/></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        "<out>1 2-3 4<empty></empty>5  6</out>"
    );
}
#[test]
fn text_serialization_ignores_comments_but_simple_content_atomizes_them() {
    let s=sheet(r##"<xsl:template match="/"><xsl:comment>ignored</xsl:comment><xsl:attribute name="bad">x</xsl:attribute></xsl:template>"##).replace("omit-xml-declaration=\"yes\"","method=\"text\"");
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XTDE0410");
    let s=sheet(r##"<xsl:template match="/"><xsl:comment>ignored</xsl:comment><xsl:processing-instruction name="pi">ignored</xsl:processing-instruction><xsl:text>visible</xsl:text></xsl:template>"##).replace("omit-xml-declaration=\"yes\"","method=\"text\"");
    assert_eq!(transform(&s, "<r/>").unwrap(), "visible");
}
#[test]
fn simple_content_separator_expansion_is_output_bounded() {
    let s=Stylesheet::compile(&sheet(r##"<xsl:template match="/"><out><xsl:attribute name="value" separator="1234567890"><xsl:sequence select="(1,2,3)"/></xsl:attribute></out></xsl:template>"##)).unwrap();
    assert_eq!(
        s.transform_with_limits(
            "<r/>",
            Limits {
                max_output_bytes: 10,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0130"
    );
}
