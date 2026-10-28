use rust_xslt::{Stylesheet, TransformOptions};
fn source(version: &str, body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="{version}" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:a"><xsl:output method="text"/>{body}</xsl:stylesheet>"#
    )
}
#[test]
fn nested_comments_are_whitespace_between_tokens() {
    let s = Stylesheet::compile(&source("3.0", r#"<xsl:template match="/"><xsl:sequence select="(: outer (: inner :) :) 1(:a:)to(:b:)3"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "1 2 3");
}
#[test]
fn comment_markers_inside_strings_and_braced_uris_are_literal() {
    let s = Stylesheet::compile(&source("3.0", r#"<xsl:variable name="Q{urn:(:uri:)}v" select="'value'"/><xsl:template match="/"><xsl:value-of select="concat('(:string:)', $Q{urn:(:uri:)}v)"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "(:string:)value");
}
#[test]
fn avt_comments_do_not_interpret_braces_or_quotes() {
    let s = Stylesheet::compile(&source("3.0", r#"<xsl:template match="/"><xsl:value-of select="(1,2)" separator="{(: } {' (: nested :) :) '--'}"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "1--2");
}
#[test]
fn empty_and_comment_only_avts_are_allowed_in_version_three() {
    for separator in ["{}", "{ (: comment :) }"] {
        let body = format!(
            r#"<xsl:template match="/"><xsl:value-of select="(1,2)" separator="{separator}"/></xsl:template>"#
        );
        assert_eq!(
            Stylesheet::compile(&source("3.0", &body))
                .unwrap()
                .transform("<r/>")
                .unwrap(),
            "12"
        );
        assert_eq!(
            Stylesheet::compile(&source("2.0", &body)).unwrap_err().code,
            "XPST0003"
        );
    }
}
#[test]
fn unterminated_and_excessively_nested_comments_fail_without_recursion() {
    let comment = "(:".repeat(129) + &":)".repeat(129);
    for expression in ["(: open".to_owned(), comment] {
        let body = format!(
            r#"<xsl:template match="/"><xsl:sequence select="{expression}1"/></xsl:template>"#
        );
        assert_eq!(
            Stylesheet::compile(&source("3.0", &body)).unwrap_err().code,
            "XPST0003"
        );
    }
    let body = r#"<xsl:template match="/"><out value="{(: open }"/></xsl:template>"#;
    assert_eq!(
        Stylesheet::compile(&source("3.0", body)).unwrap_err().code,
        "XPST0003"
    );
}
#[test]
fn braced_uris_collapse_only_xml_whitespace() {
    let s = Stylesheet::compile(&source("3.0", r#"<xsl:param name="Q{  urn:a  }v"/><xsl:template name="entry"><xsl:value-of select="$p:v"/></xsl:template>"#)).unwrap();
    let opts = TransformOptions {
        initial_template: Some("entry".into()),
        parameters: [("Q{ urn:a }v".into(), "value".into())].into(),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("", &opts).unwrap(), "value");
    let s = Stylesheet::compile(&source(
        "3.0",
        r#"<xsl:template match="/"><xsl:value-of select="count(Q{ urn:a }*)"/></xsl:template>"#,
    ))
    .unwrap();
    assert_eq!(s.transform("<r xmlns='urn:a'/>").unwrap(), "1");
}
#[test]
fn non_xml_whitespace_is_not_ignored_by_the_xpath_lexer() {
    let body = "<xsl:template match=\"/\"><xsl:sequence select=\"1\u{a0}+ 2\"/></xsl:template>";
    assert_eq!(
        Stylesheet::compile(&source("3.0", body)).unwrap_err().code,
        "XPST0003"
    );
}

#[test]
fn xmlns_namespace_is_forbidden_in_xpath_eqnames() {
    for expression in [
        "/Q{ http://www.w3.org/2000/xmlns/ }n",
        "/Q{http://www.w3.org/2000/xmlns/}*",
        "$Q{http://www.w3.org/2000/xmlns/}v",
    ] {
        let body = format!(
            r#"<xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template>"#
        );
        assert_eq!(
            Stylesheet::compile(&source("3.0", &body)).unwrap_err().code,
            "XQST0070"
        );
    }
}
