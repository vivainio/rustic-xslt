use rust_xslt::{StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" xmlns:b="urn:a" xmlns:c="urn:c"><xsl:output method="text"/>{body}</xsl:stylesheet>"##
    )
}
#[test]
fn aliases_resolve_for_bindings_calls_and_arguments() {
    let s = Stylesheet::compile(&sheet(r##"
      <xsl:variable name="a:g" select="$b:later"/>
      <xsl:variable name="a:later" select="'global'"/>
      <xsl:template match="/">
        <xsl:variable name="a:v" select="'local'"/>
        <xsl:call-template name="b:t"><xsl:with-param name="b:p" select="$b:v"/></xsl:call-template>
      </xsl:template>
      <xsl:template name="a:t"><xsl:param name="a:p"/><xsl:value-of select="concat($b:g, ':', $b:p)"/></xsl:template>
    "##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "global:local");
}
#[test]
fn rebindings_and_unqualified_names_are_distinct() {
    let s = Stylesheet::compile(&sheet(r##"
      <xsl:variable name="a:v" select="'A'"/><xsl:variable name="c:v" select="'C'"/>
      <xsl:variable name="v" select="'N'"/>
      <xsl:template match="/" xmlns="urn:default"><xsl:value-of select="$b:v"/><xsl:for-each select="*" xmlns:a="urn:c"><xsl:value-of select="$a:v"/><xsl:value-of select="$v"/></xsl:for-each></xsl:template>
    "##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "ACN");
}
#[test]
fn modes_and_current_mode_use_expanded_names() {
    let s = Stylesheet::compile(&sheet(
        r##"
      <xsl:template match="/" mode="a:m"><xsl:apply-templates mode="#current"/></xsl:template>
      <xsl:template match="r" mode="b:m"><xsl:text>A</xsl:text></xsl:template>
      <xsl:template match="r" mode="c:m"><xsl:text>C</xsl:text></xsl:template>
    "##,
    ))
    .unwrap();
    let mut opts = TransformOptions {
        initial_mode: Some("Q{urn:a}m".into()),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), "A");
    opts.initial_mode = Some("Q{urn:c}m".into());
    assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), "C");
}
#[test]
fn external_template_and_global_parameter_names() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:param name="a:p" select="'default'"/><xsl:template name="a:t"><xsl:value-of select="$b:p"/></xsl:template>"##)).unwrap();
    let opts = TransformOptions {
        initial_template: Some("Q{urn:a}t".into()),
        parameters: [("Q{urn:a}p".into(), "supplied".into())].into(),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("", &opts).unwrap(), "supplied");
}
#[test]
fn stream_entry_uses_expanded_names() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template name="a:record"><xsl:call-template name="b:t"/></xsl:template><xsl:template name="a:t"><xsl:value-of select="."/></xsl:template>"##)).unwrap();
    let opts = StreamOptions {
        record_path: "/r/n".into(),
        template: "Q{urn:a}record".into(),
        ..Default::default()
    };
    let mut output = vec![];
    s.stream_records(Cursor::new("<r><n>ok</n></r>"), &mut output, &opts)
        .unwrap();
    assert_eq!(output, b"ok");
}
#[test]
fn aliased_duplicates_are_rejected() {
    for body in [
        r##"<xsl:variable name="a:v"/><xsl:variable name="b:v"/>"##,
        r##"<xsl:template name="a:t"/><xsl:template name="b:t"/>"##,
        r##"<xsl:template name="t"><xsl:param name="a:p"/><xsl:param name="b:p"/></xsl:template>"##,
        r##"<xsl:template name="t"><xsl:call-template name="t"><xsl:with-param name="a:p"/><xsl:with-param name="b:p"/></xsl:call-template></xsl:template>"##,
    ] {
        assert!(Stylesheet::compile(&sheet(body)).is_err(), "{body}");
    }
}
#[test]
fn unbound_and_malformed_names_fail_statically() {
    for body in [
        r##"<xsl:variable name="z:v"/>"##,
        r##"<xsl:template name="z:t"/>"##,
        r##"<xsl:template match="/" mode="z:m"/>"##,
        r##"<xsl:template name="t"><xsl:call-template name="z:t"/></xsl:template>"##,
    ] {
        assert_eq!(
            Stylesheet::compile(&sheet(body)).unwrap_err().code,
            "XTSE0280"
        );
    }
    assert_eq!(
        Stylesheet::compile(&sheet(
            r##"<xsl:template name="t"><xsl:value-of select="$z:v"/></xsl:template>"##
        ))
        .unwrap_err()
        .code,
        "XPST0081"
    );
    assert_eq!(
        Stylesheet::compile(&sheet(r##"<xsl:variable name="a:b:c"/>"##))
            .unwrap_err()
            .code,
        "XTSE0020"
    );
}
#[test]
fn unresolved_qualified_variables_are_checked_in_dead_code() {
    assert_eq!(Stylesheet::compile(&sheet(r##"<xsl:template name="t"><xsl:if test="false()"><xsl:value-of select="$a:missing"/></xsl:if></xsl:template>"##)).unwrap_err().code, "XPST0008");
}
#[test]
fn invalid_external_prefix_is_rejected() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template name="a:t"/>"##)).unwrap();
    let opts = TransformOptions {
        initial_template: Some("a:t".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("", &opts).unwrap_err().code,
        "RUST_XSLT_INVALID_NAME"
    );
}
