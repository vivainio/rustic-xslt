use rust_xslt::{Stylesheet, TransformOptions};
fn sheet(version: &str, body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="{version}" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" xmlns:q="urn:p"><xsl:output method="text"/>{body}</xsl:stylesheet>"#
    )
}
#[test]
fn global_required_parameter_and_external_expanded_name() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:param name="p:v" required="yes"/><xsl:template match="/"><xsl:value-of select="$q:v"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0050");
    let options = TransformOptions {
        parameters: [("Q{urn:p}v".into(), "supplied".into())].into(),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &options).unwrap(),
        "supplied"
    );
}
#[test]
fn required_parameter_on_initial_and_matched_templates() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template name="entry" match="/"><xsl:param name="p:v" required="yes"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0700");
    assert_eq!(
        s.transform_with_options(
            "",
            &TransformOptions {
                initial_template: Some("entry".into()),
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XTDE0700"
    );
}
#[test]
fn named_calls_check_required_parameters_even_in_dead_branches() {
    let e = Stylesheet::compile(&sheet("3.0", r#"<xsl:template name="target"><xsl:param name="p:v" required="yes"/></xsl:template><xsl:template match="/"><xsl:if test="false()"><xsl:call-template name="target"/></xsl:if></xsl:template>"#)).unwrap_err();
    assert_eq!(e.code, "XTSE0690");
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template name="target"><xsl:param name="p:v" required="yes"/><xsl:value-of select="$q:v"/></xsl:template><xsl:template match="/"><xsl:call-template name="target"><xsl:with-param name="q:v" select="'ok'"/></xsl:call-template></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "ok");
}
#[test]
fn unexpected_named_arguments_are_rejected_except_backwards_mode() {
    let body = r#"<xsl:template name="target">ok</xsl:template><xsl:template match="/"><xsl:call-template name="target"><xsl:with-param name="p:v" select="'ignored'"/></xsl:call-template></xsl:template>"#;
    assert_eq!(
        Stylesheet::compile(&sheet("3.0", body)).unwrap_err().code,
        "XTSE0680"
    );
    assert_eq!(
        Stylesheet::compile(&sheet("1.0", body))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "ok"
    );
}
#[test]
fn required_parameters_cannot_have_defaults() {
    for body in [
        r#"<xsl:param name="v" required="yes" select="'bad'"/>"#,
        r#"<xsl:template name="t"><xsl:param name="v" required="yes">bad</xsl:param></xsl:template>"#,
    ] {
        assert_eq!(
            Stylesheet::compile(&sheet("3.0", body)).unwrap_err().code,
            "XTSE0010"
        );
    }
    for body in [
        r#"<xsl:variable name="v" required="no"/>"#,
        r#"<xsl:template name="t"><xsl:call-template name="t"><xsl:with-param name="v" required="yes"/></xsl:call-template></xsl:template>"#,
    ] {
        assert_eq!(
            Stylesheet::compile(&sheet("3.0", body)).unwrap_err().code,
            "XTSE0090"
        );
    }
}
#[test]
fn boolean_required_spellings_and_invalid_values() {
    for value in [" yes ", " true ", " 1 "] {
        let s = Stylesheet::compile(&sheet(
            "3.0",
            &format!(r#"<xsl:param name="v" required="{value}"/>"#),
        ))
        .unwrap();
        assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0050");
    }
    for value in ["no", "false", "0"] {
        let s = Stylesheet::compile(&sheet("3.0", &format!(r#"<xsl:param name="v" required="{value}" select="'default'"/><xsl:template match="/"><xsl:value-of select="$v"/></xsl:template>"#))).unwrap();
        assert_eq!(s.transform("<r/>").unwrap(), "default");
    }
    assert_eq!(
        Stylesheet::compile(&sheet("3.0", r#"<xsl:param name="v" required="TRUE"/>"#))
            .unwrap_err()
            .code,
        "XTSE0020"
    );
}
#[test]
fn parameter_default_and_supplied_type_errors_are_distinct() {
    for default in [r#"select="'bad'"/"#, r#">bad</xsl:param"#] {
        let body = format!(
            r#"<xsl:template match="/"><xsl:param name="v" as="xs:integer" {default}></xsl:template>"#
        );
        let s = Stylesheet::compile(&sheet("3.0", &body)).unwrap();
        assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0600");
    }
    let s = Stylesheet::compile(&sheet(
        "3.0",
        r#"<xsl:param name="v" as="xs:integer" select="1"/><xsl:template match="/"/>"#,
    ))
    .unwrap();
    let opts = TransformOptions {
        parameters: [("v".into(), "bad".into())].into(),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap_err().code,
        "XTTE0590"
    );
}
#[test]
fn binding_and_template_duplicates_have_static_codes() {
    for (body, code) in [
        (
            r#"<xsl:variable name="p:v"/><xsl:param name="q:v"/>"#,
            "XTSE0630",
        ),
        (
            r#"<xsl:template name="p:t"/><xsl:template name="q:t"/>"#,
            "XTSE0660",
        ),
        (
            r#"<xsl:template name="t"><xsl:param name="p:v"/><xsl:param name="q:v"/></xsl:template>"#,
            "XTSE0580",
        ),
    ] {
        assert_eq!(
            Stylesheet::compile(&sheet("3.0", body)).unwrap_err().code,
            code
        );
    }
}

#[test]
fn typed_parameters_without_defaults_are_implicitly_required() {
    for (version, code) in [("2.0", "XTDE0610"), ("3.0", "XTDE0700")] {
        for body in [
            r#"<xsl:param name="v" as="xs:integer"/><xsl:template name="entry"/>"#,
            r#"<xsl:template name="entry"><xsl:param name="v" as="xs:integer"/></xsl:template>"#,
        ] {
            let s = Stylesheet::compile(&sheet(version, body)).unwrap();
            let opts = TransformOptions {
                initial_template: Some("entry".into()),
                ..Default::default()
            };
            assert_eq!(s.transform_with_options("", &opts).unwrap_err().code, code);
        }
    }
}
#[test]
fn optional_empty_types_and_explicit_empty_defaults_are_distinct() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template match="/"><xsl:param name="v" as="xs:integer?"/><xsl:value-of select="empty($v)"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "true");
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template match="/"><xsl:param name="v" as="xs:integer" select="()"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0600");
}
#[test]
fn named_call_omitting_an_implicit_requirement_fails_dynamically() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template name="target"><xsl:param name="v" as="xs:integer"/></xsl:template><xsl:template match="/"><xsl:call-template name="target"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0700");
}
