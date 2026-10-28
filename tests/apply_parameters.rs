use rust_xslt::{StreamOptions, Stylesheet};
use std::io::Cursor;
fn sheet(version: &str, body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="{version}" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" xmlns:b="urn:a"><xsl:output method="text"/>{body}</xsl:stylesheet>"#
    )
}
#[test]
fn sorted_application_keeps_argument_caller_focus_and_lexical_scope() {
    let s = Stylesheet::compile(&sheet("3.0", r#"
      <xsl:template match="/r"><xsl:variable name="v" select="'outer'"/>
        <xsl:apply-templates select="n"><xsl:sort select="."/>
          <xsl:with-param name="a:p" select="concat($v, ':', name(), ':', position(), ':', last())"/>
        </xsl:apply-templates>
      </xsl:template>
      <xsl:template match="n"><xsl:param name="b:p"/><xsl:value-of select="concat($a:p, ':', ., ':', position(), ';')"/></xsl:template>
    "#)).unwrap();
    assert_eq!(
        s.transform("<r><n>B</n><n>A</n></r>").unwrap(),
        "outer:r:1:1:A:1;outer:r:1:1:B:2;"
    );
}
#[test]
fn constructor_argument_is_shared_and_defaults_apply_per_template() {
    let s = Stylesheet::compile(&sheet("3.0", r#"
      <xsl:template match="/r"><xsl:apply-templates select="*">
        <xsl:with-param name="p"><value><xsl:value-of select="@v"/></value></xsl:with-param>
      </xsl:apply-templates></xsl:template>
      <xsl:template match="n"><xsl:param name="p"/><xsl:param name="q" select="'default'"/><xsl:value-of select="concat($p/value, ':', $q, ';')"/></xsl:template>
      <xsl:template match="ignored">ignored;</xsl:template>
    "#)).unwrap();
    assert_eq!(
        s.transform("<r v='caller'><n/><n/><ignored/></r>").unwrap(),
        "caller:default;caller:default;ignored;"
    );
}
#[test]
fn built_in_rules_forward_parameters_in_modern_versions() {
    for (version, expected) in [("3.0", "supplied"), ("1.0", "default")] {
        let s = Stylesheet::compile(&sheet(version, r#"
          <xsl:template match="/"><xsl:apply-templates select="r"><xsl:with-param name="p" select="'supplied'"/></xsl:apply-templates></xsl:template>
          <xsl:template match="n"><xsl:param name="p" select="'default'"/><xsl:value-of select="$p"/></xsl:template>
        "#)).unwrap();
        assert_eq!(
            s.transform("<r><middle><n/></middle></r>").unwrap(),
            expected
        );
    }
}
#[test]
fn ordinary_parameters_do_not_leak_into_explicit_nested_applications() {
    let s = Stylesheet::compile(&sheet("3.0", r#"
      <xsl:template match="/"><xsl:apply-templates select="r"><xsl:with-param name="p" select="'supplied'"/></xsl:apply-templates></xsl:template>
      <xsl:template match="r"><xsl:param name="p"/><xsl:value-of select="$p"/><xsl:apply-templates/></xsl:template>
      <xsl:template match="n"><xsl:param name="p" select="'default'"/><xsl:value-of select="$p"/></xsl:template>
    "#)).unwrap();
    assert_eq!(s.transform("<r><n/></r>").unwrap(), "supplieddefault");
}
#[test]
fn argument_names_and_sort_order_are_checked_statically() {
    for body in [
        r#"<xsl:apply-templates><xsl:with-param name="a:p"/><xsl:with-param name="b:p"/></xsl:apply-templates>"#,
        r#"<xsl:apply-templates><xsl:with-param name="p"/><xsl:sort/></xsl:apply-templates>"#,
        r#"<xsl:apply-templates>bad</xsl:apply-templates>"#,
    ] {
        assert!(
            Stylesheet::compile(&sheet(
                "3.0",
                &format!(r#"<xsl:template match="/">{body}</xsl:template>"#)
            ))
            .is_err()
        );
    }
    let error = Stylesheet::compile(&sheet("3.0", r#"<xsl:template match="/"><xsl:if test="false()"><xsl:apply-templates><xsl:with-param name="p" select="$missing"/></xsl:apply-templates></xsl:if></xsl:template>"#)).unwrap_err();
    assert_eq!(error.code, "XPST0008");
}
#[test]
fn argument_bindings_do_not_enter_each_others_scope() {
    let error = Stylesheet::compile(&sheet("3.0", r#"<xsl:template match="/"><xsl:apply-templates><xsl:with-param name="p" select="'value'"/><xsl:with-param name="q" select="$p"/></xsl:apply-templates></xsl:template>"#)).unwrap_err();
    assert_eq!(error.code, "XPST0008");
}
#[test]
fn supplied_parameter_type_mismatch_uses_template_parameter_error() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template match="/"><xsl:apply-templates select="r"><xsl:with-param name="p" select="'bad'"/></xsl:apply-templates></xsl:template><xsl:template match="r"><xsl:param name="p" as="xs:integer"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0590");
}
#[test]
fn streaming_checks_apply_argument_constructors_before_input() {
    for value in [
        r#"select="/outside"/"#,
        r#"><xsl:value-of select="../outside"/></xsl:with-param"#,
    ] {
        let body = format!(
            r#"<xsl:template name="record"><xsl:apply-templates select="*"><xsl:with-param name="p" {value}></xsl:apply-templates></xsl:template><xsl:template match="n"><xsl:param name="p"/><xsl:value-of select="$p"/></xsl:template>"#
        );
        let s = Stylesheet::compile(&sheet("3.0", &body)).unwrap();
        let mut output = vec![];
        let error = s
            .stream_records(
                Cursor::new("not XML"),
                &mut output,
                &StreamOptions {
                    record_path: "/r/record".into(),
                    ..Default::default()
                },
            )
            .unwrap_err();
        assert_eq!(error.code, "RUST_XSLT_STREAM_UNSUPPORTED");
        assert!(output.is_empty());
    }
}

#[test]
fn streamed_record_local_constructor_arguments_work() {
    let s = Stylesheet::compile(&sheet("3.0", r#"
      <xsl:template name="record"><xsl:apply-templates select="n">
        <xsl:with-param name="a:p"><value><xsl:value-of select="@v"/></value></xsl:with-param>
      </xsl:apply-templates></xsl:template>
      <xsl:template match="n"><xsl:param name="b:p"/><xsl:value-of select="concat($a:p/value, ':', ., ';')"/></xsl:template>
    "#)).unwrap();
    let mut output = vec![];
    let stats = s
        .stream_records(
            Cursor::new(
                "<r><record v='A'><n>1</n><n>2</n></record><record v='B'><n>3</n></record></r>",
            ),
            &mut output,
            &StreamOptions {
                record_path: "/r/record".into(),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(stats.records, 2);
    assert_eq!(output, b"A:1;A:2;B:3;");
}
