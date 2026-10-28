use rust_xslt::{StreamOptions, Stylesheet, TransformOptions, transform};
use std::io::Cursor;
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{body}</xsl:stylesheet>"##
    )
}
fn canonical(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut result = String::new();
    for n in doc.descendants() {
        if n.is_element() {
            result.push_str(&format!(
                "E{:?}:{}",
                n.tag_name().namespace(),
                n.tag_name().name()
            ));
            let mut attrs: Vec<_> = n
                .attributes()
                .map(|a| format!("A{:?}:{}={}", a.namespace(), a.name(), a.value()))
                .collect();
            attrs.sort();
            result.push_str(&attrs.join(";"));
        } else if n.is_text() {
            result.push_str(&format!("T{}", n.text().unwrap()));
        } else if n.is_comment() {
            result.push_str(&format!("C{}", n.text().unwrap()));
        } else if let Some(pi) = n.pi() {
            result.push_str(&format!("P{}:{:?}", pi.target, pi.value));
        }
        // Include ancestry so different tree shapes cannot compare equal.
        result.push_str(&format!("D{};", n.ancestors().count()));
    }
    result
}
#[test]
fn identity_copies_attributes_namespaces_comments_and_pi() {
    let s = sheet(
        r##"<xsl:template match="@*|node()"><xsl:copy><xsl:apply-templates select="@*|node()"/></xsl:copy></xsl:template>"##,
    );
    let input = r##"<?notice keep?><r xmlns="urn:default" xmlns:p="urn:p" id="a" p:id="b"><p:item xml:lang="en">A&amp;B<!--comment--></p:item><plain xmlns="">text</plain></r>"##;
    let output = transform(&s, input).unwrap();
    assert_eq!(canonical(&output), canonical(input), "{output}");
}
#[test]
fn deep_copy_keeps_subtrees_and_avoids_template_dispatch() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:copy-of select="r/*"/></out></xsl:template><xsl:template match="*"><wrong/></xsl:template>"##,
    );
    let input = r##"<r xmlns:p="urn:p"><p:n a="&quot;"><x/>A<!--c--><?pi data?></p:n></r>"##;
    let output = transform(&s, input).unwrap();
    assert_eq!(
        canonical(&output),
        canonical(r##"<out><p:n xmlns:p="urn:p" a="&quot;"><x/>A<!--c--><?pi data?></p:n></out>"##)
    );
}
#[test]
fn literal_namespace_fixup_and_default_undeclaration() {
    let s = sheet(
        r##"<xsl:template match="/"><outer xmlns="urn:a" xmlns:p="urn:p" p:id="{@id}"><plain xmlns=""><xsl:text>x</xsl:text></plain><p:inner/></outer></xsl:template>"##,
    );
    let output = transform(&s, "<r/>").unwrap();
    assert_eq!(
        canonical(&output),
        canonical(
            r##"<outer xmlns="urn:a" xmlns:p="urn:p" p:id=""><plain xmlns="">x</plain><p:inner/></outer>"##
        ),
        "{output}"
    );
}
#[test]
fn template_patterns_respect_namespaces_predicates_and_priority() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:apply-templates select="r/*"/></out></xsl:template><xsl:template match="*"><wild/></xsl:template><xsl:template match="n"><named/></xsl:template><xsl:template match="n[@special='yes']"><special/></xsl:template><xsl:template match="p:n" xmlns:p="urn:p"><qualified/></xsl:template><xsl:template match="n[@override]" priority="5"><explicit/></xsl:template>"##,
    );
    assert_eq!(
        transform(
            &s,
            "<r xmlns:p='urn:p'><other/><n/><n special='yes'/><p:n/><n override='yes'/></r>"
        )
        .unwrap(),
        "<out><wild></wild><named></named><special></special><qualified xmlns:p=\"urn:p\"></qualified><explicit></explicit></out>"
    );
}
#[test]
fn positional_pattern_uses_actual_sibling_focus() {
    let s = sheet(
        r##"<xsl:template match="/"><out><xsl:apply-templates select="r/n"/></out></xsl:template><xsl:template match="n"><other/></xsl:template><xsl:template match="n[1]"><first/></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r><n/><n/></r>").unwrap(),
        "<out><first></first><other></other></out>"
    );
}
#[test]
fn named_modes_and_current_mode_survive_named_calls() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template match="/" mode="m"><out><xsl:call-template name="continue"/></out></xsl:template><xsl:template name="continue"><xsl:apply-templates select="r/n" mode="#current"/></xsl:template><xsl:template match="n"><default/></xsl:template><xsl:template match="n" mode="m"><m/></xsl:template>"##)).unwrap();
    let opts = TransformOptions {
        initial_mode: Some("m".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r><n/></r>", &opts).unwrap(),
        "<out><m></m></out>"
    );
}
#[test]
fn duplicate_copied_attributes_replace_and_late_attributes_fail() {
    let s = sheet(
        r##"<xsl:template match="/r"><out id="original"><xsl:copy-of select="@id"/></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r id='replacement'/>").unwrap(),
        "<out id=\"replacement\"></out>"
    );
    let s = sheet(
        r##"<xsl:template match="/r"><out>text<xsl:copy-of select="@id"/></out></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r id='late'/>").unwrap_err().code,
        "XTDE0410"
    );
}
#[test]
fn streaming_identity_is_record_local_and_preserves_namespaces() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template name="record"><xsl:apply-templates select="."/></xsl:template><xsl:template match="@*|node()"><xsl:copy><xsl:apply-templates select="@*|node()"/></xsl:copy></xsl:template>"##)).unwrap();
    let input = r##"<records xmlns:p="urn:p"><record p:id="a"><p:n>ok</p:n></record></records>"##;
    let mut output = vec![];
    let stats = s
        .stream_records(Cursor::new(input), &mut output, &StreamOptions::default())
        .unwrap();
    assert_eq!(stats.records, 1);
    assert_eq!(
        canonical(std::str::from_utf8(&output).unwrap()),
        canonical(r##"<record xmlns:p="urn:p" p:id="a"><p:n>ok</p:n></record>"##)
    );
}
#[test]
fn streaming_rejects_dispatch_patterns_that_see_record_siblings() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template name="record"><xsl:apply-templates select="."/></xsl:template><xsl:template match="record[1]"/>"##)).unwrap();
    assert_eq!(
        s.stream_records(
            Cursor::new("<records/>"),
            &mut vec![],
            &StreamOptions::default()
        )
        .unwrap_err()
        .code,
        "RUST_XSLT_STREAM_UNSUPPORTED"
    );
}

#[test]
fn copied_attribute_namespace_conflicts_do_not_rebind_existing_names() {
    let s = sheet(
        r##"<xsl:template xmlns:p="urn:one" match="/r"><out p:id="original"><xsl:copy-of select="@*"/></out></xsl:template>"##,
    );
    let output = transform(&s, "<r xmlns:p='urn:two' p:value='copied'/>").unwrap();
    let doc = roxmltree::Document::parse(&output).unwrap();
    let root = doc.root_element();
    assert_eq!(root.attribute(("urn:one", "id")), Some("original"));
    assert_eq!(root.attribute(("urn:two", "value")), Some("copied"));
}
#[test]
fn node_pattern_does_not_match_attributes_on_the_child_axis() {
    let s = sheet(
        r##"<xsl:template match="/r"><out><xsl:apply-templates select="@id"/></out></xsl:template><xsl:template match="@*"><xsl:copy/></xsl:template><xsl:template match="node()"><wrong/></xsl:template>"##,
    );
    assert_eq!(
        transform(&s, "<r id='yes'/>").unwrap(),
        "<out id=\"yes\"></out>"
    );
}
