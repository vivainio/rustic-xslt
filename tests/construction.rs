use rust_xslt::{Limits, StreamOptions, Stylesheet, transform};
use std::io::Cursor;
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{body}</xsl:stylesheet>"##
    )
}
#[test]
fn computed_attributes_adapt_go_fixture() {
    let s = sheet(
        r##"<xsl:template match="/r"><out id="{@id}-x"><xsl:attribute name="count"><xsl:value-of select="count(item)"/></xsl:attribute></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r id='7'><item/><item/></r>").unwrap(),
        "<out id=\"7-x\" count=\"2\"></out>"
    );
}
#[test]
fn computed_element_and_attribute_namespaces() {
    let s = sheet(
        r##"<xsl:template match="/r"><xsl:element name="p:{@name}" namespace="urn:element"><xsl:attribute name="p:attr" namespace="urn:attribute" select="'value'"/><xsl:element name="plain" namespace=""/></xsl:element></xsl:template>"##,
    );
    let output = transform(&s, "<r name='made'/>").unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    let root = doc.root_element();
    assert_eq!(root.tag_name().namespace(), Some("urn:element"));
    assert_eq!(root.tag_name().name(), "made");
    assert_eq!(root.attribute(("urn:attribute", "attr")), Some("value"));
    assert_eq!(
        root.first_element_child().unwrap().tag_name().namespace(),
        None
    );
}
#[test]
fn computed_names_use_stylesheet_bindings_and_element_default_namespace() {
    let s = sheet(
        r##"<xsl:template xmlns="urn:default" xmlns:p="urn:p" match="/"><xsl:element name="outer"><xsl:attribute name="plain">no namespace</xsl:attribute><xsl:attribute name="p:qualified">yes</xsl:attribute></xsl:element></xsl:template>"##,
    );
    let output = transform(&s, "<r/>").unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    let root = doc.root_element();
    assert_eq!(root.tag_name().namespace(), Some("urn:default"));
    assert_eq!(root.attribute("plain"), Some("no namespace"));
    assert_eq!(root.attribute(("urn:p", "qualified")), Some("yes"));
}
#[test]
fn capture_scopes_and_result_nodes() {
    let s = sheet(
        r##"<xsl:template match="/r"><out><xsl:attribute name="mixed">a<xsl:if test="@yes"><xsl:value-of select="@yes"/></xsl:if><nested>b</nested></xsl:attribute><xsl:comment select="'a--b-'"/><xsl:processing-instruction name="target">data?&gt;end</xsl:processing-instruction><xsl:value-of select="n" separator="|"/></out></xsl:template>"##,
    );
    let output = transform(&s, "<r yes='Y'><n>1</n><n>2</n></r>").unwrap();
    assert_eq!(
        output,
        "<out mixed=\"aYb\"><!--a- -b- --><?target data? >end?>1|2</out>"
    );
    assert!(roxmltree::Document::parse(&output).is_ok());
}
#[test]
fn invalid_computed_names_and_late_attributes_report_dynamic_codes() {
    for (body, code) in [
        (r##"<xsl:element name="bad name"/>"##, "XTDE0820"),
        (r##"<xsl:element name="p:n"/>"##, "XTDE0830"),
        (r##"<out><xsl:attribute name="xmlns"/></out>"##, "XTDE0855"),
        (
            r##"<out><xsl:attribute name="n" namespace="http://www.w3.org/2000/xmlns/"/></out>"##,
            "XTDE0865",
        ),
        (r##"<xsl:processing-instruction name="XmL"/>"##, "XTDE0890"),
        (
            r##"<out>x<xsl:attribute name="n">late</xsl:attribute></out>"##,
            "XTDE0410",
        ),
    ] {
        let s = sheet(&format!(
            r##"<xsl:template match="/">{body}</xsl:template>"##
        ));
        assert_eq!(transform(&s, "<r/>").unwrap_err().code, code, "{body}");
    }
}
#[test]
fn numeric_sort_descending_adapts_go_fixture() {
    let s = sheet(
        r##"<xsl:template match="/list"><xsl:for-each select="n"><xsl:sort select="." data-type="number" order="descending"/><xsl:value-of select="."/><xsl:text> </xsl:text></xsl:for-each></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<list><n>3</n><n>1</n><n>2</n></list>").unwrap(),
        "3 2 1 "
    );
}
#[test]
fn multikey_sort_is_stable_and_nan_sorts_first() {
    let s = sheet(
        r##"<xsl:template match="/"><xsl:for-each select="r/n"><xsl:sort select="@group"/><xsl:sort select="@value" data-type="number"/><xsl:value-of select="@id"/><xsl:text>,</xsl:text></xsl:for-each></xsl:template>"##,
    );
    assert_eq!(transform(&s,"<r><n id='a' group='z' value='1'/><n id='b' group='a' value='2'/><n id='c' group='a' value='NaN'/><n id='d' group='a' value='2.0'/><n id='e' group='a' value='1'/><n id='f' group='a' value='bad'/></r>").unwrap(),"c,f,e,b,d,a,");
}
#[test]
fn apply_templates_sorts_and_resets_position_after_sorting() {
    let s = sheet(
        r##"<xsl:template match="/"><xsl:apply-templates select="r/n" mode="m"><xsl:sort select="@v" data-type="number"/></xsl:apply-templates></xsl:template><xsl:template match="n" mode="m"><xsl:value-of select="concat(position(), '/', last(), ':', @v)"/><xsl:text>;</xsl:text></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r><n v='3'/><n v='1'/><n v='2'/></r>").unwrap(),
        "1/3:1;2/3:2;3/3:3;"
    );
}
#[test]
fn sort_key_context_uses_original_focus() {
    let s = sheet(
        r##"<xsl:template match="/"><xsl:for-each select="r/n"><xsl:sort select="position()" data-type="number" order="descending"/><xsl:value-of select="concat(position(), ':', @id)"/><xsl:text>;</xsl:text></xsl:for-each></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r><n id='a'/><n id='b'/><n id='c'/></r>").unwrap(),
        "1:c;2:b;3:a;"
    );
}
#[test]
fn invalid_sort_placement_and_non_singleton_keys_fail() {
    assert!(Stylesheet::compile(&sheet(r##"<xsl:template match="/"><xsl:for-each select="r/n">bad<xsl:sort/></xsl:for-each></xsl:template>"##)).is_err());
    assert_eq!(transform(&sheet(r##"<xsl:template match="/"><xsl:for-each select="r/n"><xsl:sort select="(1,2)"/></xsl:for-each></xsl:template>"##),"<r><n/></r>").unwrap_err().code,"XTTE1020");
}
#[test]
fn record_local_sorting_and_computed_nodes_stream() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template name="record"><xsl:element name="result"><xsl:attribute name="id" select="@id"/><xsl:for-each select="n"><xsl:sort data-type="number"/><xsl:element name="n"><xsl:value-of select="."/></xsl:element></xsl:for-each></xsl:element></xsl:template>"##)).unwrap();
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<records><record id='a'><n>2</n><n>1</n></record></records>"),
        &mut output,
        &StreamOptions::default(),
    )
    .unwrap();
    assert_eq!(output, b"<result id=\"a\"><n>1</n><n>2</n></result>");
}
#[test]
fn capture_and_sort_share_budget_and_output_limit() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template match="/"><out><xsl:attribute name="long"><xsl:text>123456789</xsl:text></xsl:attribute></out></xsl:template>"##)).unwrap();
    assert_eq!(
        s.transform_with_limits(
            "<r/>",
            Limits {
                max_output_bytes: 8,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0130"
    );
    let s = Stylesheet::compile(&sheet(r##"<xsl:template match="/"><xsl:for-each select="r/n"><xsl:sort select="."/></xsl:for-each></xsl:template>"##)).unwrap();
    let source = format!("<r>{}</r>", "<n>test</n>".repeat(100));
    assert_eq!(
        s.transform_with_limits(
            &source,
            Limits {
                max_steps: 500,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0130"
    );
}
