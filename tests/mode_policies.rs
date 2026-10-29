use rust_xslt::{StreamOptions, Stylesheet};
use std::io::Cursor;
fn source(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{body}</xsl:stylesheet>"#
    )
}
#[test]
fn shallow_copy_visits_attributes_and_children_and_preserves_other_nodes() {
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-no-match="shallow-copy"/><xsl:template match="@v"><xsl:attribute name="changed" select="."/></xsl:template><xsl:template match="b"><B/></xsl:template>"#)).unwrap();
    assert_eq!(
        s.transform("<r v='1'><b/><!--c--><?p d?><n>text</n></r>")
            .unwrap(),
        "<r changed=\"1\"><B></B><!--c--><?p d?><n>text</n></r>"
    );
}
#[test]
fn deep_copy_and_deep_skip_do_not_dispatch_descendants() {
    for (policy, expected) in [("deep-copy", "<r><b></b></r>"), ("deep-skip", "")] {
        let s = Stylesheet::compile(&source(&format!(
            r#"<xsl:mode on-no-match="{policy}"/><xsl:template match="b">wrong</xsl:template>"#
        )))
        .unwrap();
        assert_eq!(s.transform("<r><b/></r>").unwrap(), expected);
    }
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-no-match="deep-skip"/><xsl:template match="r">document-descended</xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "document-descended");
}
#[test]
fn shallow_skip_uses_separate_attribute_and_child_focus() {
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-no-match="shallow-skip"/><xsl:template match="@*"><xsl:value-of select="concat(name(), ':', position(), '/', last(), ';')"/></xsl:template><xsl:template match="n"><xsl:value-of select="concat(position(), '/', last(), ';')"/></xsl:template>"#)).unwrap();
    assert_eq!(
        s.transform("<r a='x' b='y'><n/><n/></r>").unwrap(),
        "a:1/2;b:2/2;1/2;2/2;"
    );
}
#[test]
fn fail_policy_and_multiple_match_errors_are_distinct() {
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-no-match="fail"/><xsl:template match="/"><xsl:apply-templates/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0555");
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-multiple-match="fail"/><xsl:template match="r">first</xsl:template><xsl:template match="r">second</xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0540");
}
#[test]
fn only_highest_priority_distinct_templates_are_ambiguous() {
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-multiple-match="fail"/><xsl:template match="r">first</xsl:template><xsl:template match="r">second</xsl:template><xsl:template match="r|r" priority="1">winner</xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "winner");
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-multiple-match="use-last"/><xsl:template match="r">first</xsl:template><xsl:template match="r">second</xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "second");
}
#[test]
fn compatible_mode_declarations_merge_and_conflicts_fail() {
    let s = Stylesheet::compile(&source(r#"<xsl:mode on-no-match="shallow-copy"/><xsl:mode on-multiple-match="use-last"/><xsl:mode on-no-match="shallow-copy"/>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "<r></r>");
    assert_eq!(
        Stylesheet::compile(&source(
            r#"<xsl:mode on-no-match="shallow-copy"/><xsl:mode on-no-match="deep-skip"/>"#
        ))
        .unwrap_err()
        .code,
        "XTSE0545"
    );
    assert_eq!(
        Stylesheet::compile(&source(r#"<xsl:mode on-no-match="invalid"/>"#))
            .unwrap_err()
            .code,
        "XTSE0020"
    );
    assert_eq!(
        Stylesheet::compile(&source(r#"<xsl:mode streamable="yes"/>"#))
            .unwrap_err()
            .code,
        "RUST_XSLT_UNSUPPORTED"
    );
}
#[test]
fn grounded_record_dispatch_obeys_mode_policy() {
    let s = Stylesheet::compile(&source(r#"<xsl:mode name="m" on-no-match="shallow-copy"/><xsl:template name="record"><xsl:apply-templates select="." mode="m"/></xsl:template><xsl:template match="n" mode="m"><changed/></xsl:template>"#)).unwrap();
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<r><record v='1'><n/></record></r>"),
        &mut output,
        &StreamOptions {
            record_path: "/r/record".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "<record v=\"1\"><changed></changed></record>"
    );
}

#[test]
fn warning_on_multiple_match_is_validated_as_a_mode_boolean() {
    let sheet = Stylesheet::compile(&source(
        r#"<xsl:mode warning-on-multiple-match="no"/><xsl:template match="r">ok</xsl:template>"#,
    ))
    .unwrap();
    assert_eq!(sheet.transform("<r/>").unwrap(), "ok");
    assert_eq!(
        Stylesheet::compile(&source(r#"<xsl:mode warning-on-multiple-match="maybe"/>"#))
            .unwrap_err()
            .code,
        "XTSE0020"
    );
}
