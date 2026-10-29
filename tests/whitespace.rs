use rust_xslt::{StreamOptions, Stylesheet, transform};
use std::io::Cursor;
fn sheet(decls: &str, body: &str) -> String {
    format!(
        r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output omit-xml-declaration="yes"/>{decls}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"##
    )
}
#[test]
fn stripping_changes_xpath_counts_string_values_and_identity_together() {
    let s = sheet(
        r##"<xsl:strip-space elements="*"/>"##,
        r##"<out count="{count(r/text())}"><xsl:copy-of select="r"/></out>"##,
    );
    assert_eq!(
        transform(&s, "<r> \n <n>keep</n> \t <![CDATA[  ]]></r>").unwrap(),
        "<out count=\"0\"><r><n>keep</n></r></out>"
    );
}
#[test]
fn preserve_rules_are_more_specific_and_xml_space_is_inherited() {
    let s = sheet(
        r##"<xsl:strip-space elements="*"/><xsl:preserve-space elements="keep"/>"##,
        r##"<xsl:copy-of select="r"/>"##,
    );
    let output = transform(&s,"<r> <keep> </keep><drop> </drop><group xml:space='preserve'> <child> </child><reset xml:space='default'> </reset></group> </r>").unwrap();
    assert_eq!(
        output,
        "<r><keep> </keep><drop></drop><group xml:space=\"preserve\"> <child> </child><reset xml:space=\"default\"></reset></group></r>"
    );
}
#[test]
fn namespace_wildcard_rules_and_conflicting_rules() {
    let s = sheet(
        r##"<xsl:strip-space xmlns:p="urn:p" elements="p:*"/>"##,
        r##"<xsl:copy-of select="r"/>"##,
    );
    let output = transform(&s, "<r xmlns:p='urn:p'><p:n> </p:n><n> </n></r>").unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    let children: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.is_element())
        .collect();
    assert_eq!(children[0].text(), None);
    assert_eq!(children[1].text(), Some(" "));
    assert_eq!(
        Stylesheet::compile(&sheet(
            r##"<xsl:strip-space elements="n"/><xsl:preserve-space elements="n"/>"##,
            ""
        ))
        .unwrap_err()
        .code,
        "XTSE0270"
    );
}
#[test]
fn stylesheet_xml_space_and_non_xml_whitespace_are_preserved() {
    let s = sheet("", r##"<out xml:space="preserve">   </out>"##);
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        "<out xml:space=\"preserve\">   </out>"
    );
    let s = sheet("", "<out>\u{a0}</out>");
    assert_eq!(transform(&s, "<r/>").unwrap(), "<out>\u{a0}</out>");
}
#[test]
fn exclude_all_is_a_snapshot_of_its_declaring_scope() {
    let s = r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" exclude-result-prefixes="#all" version="3.0"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><out xmlns:b="urn:b"><a:required/></out></xsl:template></xsl:stylesheet>"##;
    let output = transform(s, "<r/>").unwrap();
    assert_eq!(
        output,
        "<out xmlns:b=\"urn:b\"><a:required xmlns:a=\"urn:a\"></a:required></out>"
    );
}
#[test]
fn literal_exclusion_preserves_required_attribute_namespaces() {
    let s = sheet(
        "",
        r##"<out xmlns:p="urn:p" xmlns:q="urn:q" xsl:exclude-result-prefixes="p q" p:required="yes"/>"##,
    );
    assert_eq!(
        transform(&s, "<r/>").unwrap(),
        "<out xmlns:p=\"urn:p\" p:required=\"yes\"></out>"
    );
    let s = sheet("", r##"<out xsl:exclude-result-prefixes="missing"/>"##);
    assert_eq!(Stylesheet::compile(&s).unwrap_err().code, "XTSE0808");
}
#[test]
fn output_utf8_and_explicit_no_indent_are_supported() {
    let s = r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output method="xml" encoding="utf-8" indent="no" version="1.0"/><xsl:template match="/"><out>é</out></xsl:template></xsl:stylesheet>"##;
    assert_eq!(
        transform(s, "<r/>").unwrap(),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><out>é</out>"
    );
    assert!(Stylesheet::compile(&s.replace("utf-8", "ISO-8859-1")).is_err());
    assert!(Stylesheet::compile(&s.replace("indent=\"no\"", "indent=\"maybe\"")).is_err());
}
#[test]
fn streamed_whitespace_rules_honor_ancestor_xml_space_without_injecting_attributes() {
    let s = r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output omit-xml-declaration="yes"/><xsl:strip-space elements="*"/><xsl:template name="record"><xsl:copy-of select="."/></xsl:template></xsl:stylesheet>"##;
    let s = Stylesheet::compile(s).unwrap();
    for (attribute, expected) in [
        ("", "<record><n></n></record>"),
        (" xml:space='preserve'", "<record> <n> </n> </record>"),
    ] {
        let input = format!("<records{attribute}><record> <n> </n> </record></records>");
        let mut output = vec![];
        s.stream_records(Cursor::new(input), &mut output, &StreamOptions::default())
            .unwrap();
        assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    }
}

#[test]
fn coalesced_text_cdata_and_character_references_are_stripped_as_one_node() {
    let s = sheet(
        r##"<xsl:strip-space elements="*"/>"##,
        r##"<xsl:copy-of select="r"/>"##,
    );
    assert_eq!(
        transform(
            &s,
            "<r><![CDATA[ ]]>&#32; <![CDATA[\t]]><!--boundary--> <![CDATA[ ]]></r>"
        )
        .unwrap(),
        "<r><!--boundary--></r>"
    );
    assert_eq!(
        transform(&s, "<r> <![CDATA[x]]> </r>").unwrap(),
        "<r> x </r>"
    );
}
