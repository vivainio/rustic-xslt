use rust_xslt::{ParameterValue as P, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn sheet(body: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" xmlns:q="urn:p"><xsl:output method="text"/>{body}</xsl:stylesheet>"#)).unwrap()
}
#[test]
fn tunnels_pass_through_undeclared_templates_and_builtin_rules() {
    let s = sheet(
        r#"
      <xsl:template match="/"><xsl:call-template name="bridge"><xsl:with-param name="p:v" tunnel="yes" select="'passed'"/></xsl:call-template></xsl:template>
      <xsl:template name="bridge"><xsl:apply-templates/></xsl:template>
      <xsl:template match="leaf"><xsl:param name="q:v" tunnel="yes" required="yes"/><xsl:value-of select="$p:v"/></xsl:template>
    "#,
    );
    assert_eq!(
        s.transform("<r><middle><leaf/></middle></r>").unwrap(),
        "passed"
    );
}
#[test]
fn tunnel_overrides_are_scoped_to_the_called_branch() {
    let s = sheet(
        r#"
      <xsl:template name="entry"><xsl:call-template name="leaf"><xsl:with-param name="p:v" tunnel="yes" select="'inner'"/></xsl:call-template><xsl:call-template name="leaf"/></xsl:template>
      <xsl:template name="leaf"><xsl:param name="p:v" tunnel="yes"/><xsl:value-of select="concat($q:v, ';')"/></xsl:template>
    "#,
    );
    let opts = TransformOptions {
        initial_template: Some("entry".into()),
        initial_tunnel_parameters: [("Q{urn:p}v".into(), "outer".into())].into(),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("", &opts).unwrap(), "inner;outer;");
}
#[test]
fn ordinary_parameters_do_not_intercept_tunnel_values() {
    let s = sheet(
        r#"
      <xsl:template name="entry"><xsl:param name="p:v" select="'ordinary'"/><xsl:value-of select="concat($p:v, ':')"/><xsl:call-template name="leaf"/></xsl:template>
      <xsl:template name="leaf"><xsl:param name="p:v" tunnel="yes" required="yes"/><xsl:value-of select="$p:v"/></xsl:template>
    "#,
    );
    let opts = TransformOptions {
        initial_template: Some("entry".into()),
        initial_tunnel_parameters: [("Q{urn:p}v".into(), "tunnel".into())].into(),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("", &opts).unwrap(),
        "ordinary:tunnel"
    );
}
#[test]
fn tunnel_defaults_do_not_become_supplied_tunnel_values() {
    let s = sheet(
        r#"
      <xsl:template name="entry"><xsl:param name="v" tunnel="yes" select="'entry-default'"/><xsl:value-of select="concat($v, ':')"/><xsl:call-template name="leaf"/></xsl:template>
      <xsl:template name="leaf"><xsl:param name="v" tunnel="yes" select="'leaf-default'"/><xsl:value-of select="$v"/></xsl:template>
    "#,
    );
    assert_eq!(
        s.transform_with_options(
            "",
            &TransformOptions {
                initial_template: Some("entry".into()),
                ..Default::default()
            }
        )
        .unwrap(),
        "entry-default:leaf-default"
    );
}
#[test]
fn receiving_type_conversion_does_not_change_the_inherited_tunnel() {
    let s = sheet(
        r#"
      <xsl:template match="/"><xsl:call-template name="bridge"><xsl:with-param name="v" tunnel="yes" select="r/@v"/></xsl:call-template></xsl:template>
      <xsl:template name="bridge"><xsl:param name="v" tunnel="yes" as="xs:integer"/><xsl:value-of select="$v + 1"/><xsl:call-template name="leaf"/></xsl:template>
      <xsl:template name="leaf"><xsl:param name="v" tunnel="yes" as="attribute()"/><xsl:value-of select="$v instance of attribute()"/></xsl:template>
    "#,
    );
    assert_eq!(s.transform("<r v='9'/>").unwrap(), "10true");
}
#[test]
fn missing_required_tunnel_parameters_are_dynamic_errors() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:call-template name="leaf"/></xsl:template><xsl:template name="leaf"><xsl:param name="v" tunnel="yes" required="yes"/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0700");
}
#[test]
fn ordinary_and_tunnel_initial_parameters_can_have_the_same_name() {
    let s = sheet(
        r#"<xsl:template name="entry"><xsl:param name="v" required="yes"/><xsl:value-of select="concat($v, ':')"/><xsl:call-template name="leaf"/></xsl:template><xsl:template name="leaf"><xsl:param name="v" tunnel="yes" required="yes"/><xsl:value-of select="$v"/></xsl:template>"#,
    );
    let opts = TransformOptions {
        initial_template: Some("entry".into()),
        initial_parameters: [("v".into(), "ordinary".into())].into(),
        initial_tunnel_parameters: [("v".into(), "tunnel".into())].into(),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("", &opts).unwrap(),
        "ordinary:tunnel"
    );
}
#[test]
fn streamed_entry_tunnels_flow_through_argument_constructors() {
    let s = sheet(
        r#"
      <xsl:template name="record"><xsl:apply-templates select="n"><xsl:with-param name="arg"><xsl:call-template name="leaf"/></xsl:with-param></xsl:apply-templates></xsl:template>
      <xsl:template name="leaf"><xsl:param name="v" tunnel="yes" required="yes"/><xsl:value-of select="$v"/></xsl:template>
      <xsl:template match="n"><xsl:param name="arg"/><xsl:param name="v" tunnel="yes" required="yes"/><xsl:value-of select="concat($arg, ':', $v, ';')"/></xsl:template>
    "#,
    );
    let opts = StreamOptions {
        record_path: "/r/record".into(),
        initial_tunnel_parameters: [("v".into(), P::Integer("7".into()))].into(),
        ..Default::default()
    };
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<r><record><n/></record><record><n/></record></r>"),
        &mut output,
        &opts,
    )
    .unwrap();
    assert_eq!(output, b"7:7;7:7;");
}
#[test]
fn global_tunnel_parameters_are_invalid() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:param name="v" tunnel="yes"/></xsl:stylesheet>"#;
    assert_eq!(Stylesheet::compile(source).unwrap_err().code, "XTSE0020");
}
