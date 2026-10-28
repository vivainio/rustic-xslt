use rust_xslt::{ParameterValue as P, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn sheet(body: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p"><xsl:output method="text"/>{body}</xsl:stylesheet>"#)).unwrap()
}
#[test]
fn global_and_initial_parameters_with_same_name_remain_independent() {
    let s = sheet(
        r#"<xsl:param name="p:v" required="yes"/><xsl:variable name="global" select="$p:v"/><xsl:template name="entry"><xsl:param name="p:v" required="yes"/><xsl:value-of select="concat($global, ':', $p:v)"/></xsl:template>"#,
    );
    let opts = TransformOptions {
        initial_template: Some("entry".into()),
        parameters: [("Q{urn:p}v".into(), "global".into())].into(),
        initial_parameters: [("Q{urn:p}v".into(), "initial".into())].into(),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("", &opts).unwrap(),
        "global:initial"
    );
}
#[test]
fn initial_mode_parameters_reach_the_selected_template() {
    let s = sheet(
        r#"<xsl:template match="/" mode="p:m"><xsl:param name="v" required="yes" as="xs:integer"/><xsl:value-of select="$v + 1"/></xsl:template>"#,
    );
    let opts = TransformOptions {
        initial_mode: Some("Q{urn:p}m".into()),
        initial_parameters: [("v".into(), 41_i64.into())].into(),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), "42");
}
#[test]
fn typed_global_parameters_retain_atomic_identity() {
    let s = sheet(
        r#"<xsl:param name="v" as="item()?" required="yes"/><xsl:template match="/"><xsl:value-of select="concat($v, ':', $v instance of xs:integer, ':', $v instance of xs:decimal, ':', $v instance of xs:float, ':', $v instance of xs:double, ':', $v instance of xs:boolean, ':', empty($v))"/></xsl:template>"#,
    );
    for (value, expected) in [
        (
            P::Integer("123456789012345678901234567890".into()),
            "123456789012345678901234567890:true:true:false:false:false:false",
        ),
        (
            P::Decimal("1.25".into()),
            "1.25:false:true:false:false:false:false",
        ),
        (P::Float(2.5), "2.5:false:false:true:false:false:false"),
        (P::Double(2.5), "2.5:false:false:false:true:false:false"),
        (P::Boolean(true), "true:false:false:false:false:true:false"),
        (P::Empty, ":false:false:false:false:false:true"),
    ] {
        let opts = TransformOptions {
            typed_parameters: [("v".into(), value)].into(),
            ..Default::default()
        };
        assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), expected);
    }
}
#[test]
fn initial_untyped_values_convert_but_strings_retain_their_type() {
    let s = sheet(
        r#"<xsl:template name="entry"><xsl:param name="v" as="xs:integer" required="yes"/><xsl:value-of select="$v + 1"/></xsl:template>"#,
    );
    let mut opts = TransformOptions {
        initial_template: Some("entry".into()),
        initial_parameters: [("v".into(), P::UntypedAtomic("41".into()))].into(),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("", &opts).unwrap(), "42");
    opts.initial_parameters.insert("v".into(), "41".into());
    assert_eq!(
        s.transform_with_options("", &opts).unwrap_err().code,
        "XTTE0590"
    );
}
#[test]
fn invalid_and_oversized_numeric_parameters_are_bounded() {
    let s = sheet(r#"<xsl:template name="entry"/>"#);
    for (value, expected) in [
        (P::Integer("bad".into()), "FORG0001"),
        (P::Decimal("1e2".into()), "FORG0001"),
        (P::Integer("1".repeat(10001)), "XPDY0130"),
    ] {
        let opts = TransformOptions {
            initial_template: Some("entry".into()),
            initial_parameters: [("v".into(), value)].into(),
            ..Default::default()
        };
        assert_eq!(
            s.transform_with_options("", &opts).unwrap_err().code,
            expected
        );
    }
}
#[test]
fn aliased_external_parameter_duplicates_are_errors() {
    let s = sheet(r#"<xsl:template name="entry"/>"#);
    for opts in [
        TransformOptions {
            initial_template: Some("entry".into()),
            parameters: [("v".into(), "one".into()), ("Q{}v".into(), "two".into())].into(),
            ..Default::default()
        },
        TransformOptions {
            initial_template: Some("entry".into()),
            parameters: [("v".into(), "one".into())].into(),
            typed_parameters: [("Q{}v".into(), "two".into())].into(),
            ..Default::default()
        },
        TransformOptions {
            initial_template: Some("entry".into()),
            initial_parameters: [("v".into(), "one".into()), ("Q{}v".into(), "two".into())].into(),
            ..Default::default()
        },
    ] {
        assert_eq!(
            s.transform_with_options("", &opts).unwrap_err().code,
            "RUST_XSLT_INVALID_PARAMETER"
        );
    }
}
#[test]
fn streamed_record_templates_accept_atomic_entry_arguments() {
    let s = sheet(
        r#"<xsl:param name="v" as="xs:integer" required="yes"/><xsl:variable name="g" select="$v"/><xsl:template name="record"><xsl:param name="v" as="xs:integer" required="yes"/><xsl:value-of select="concat($g, ':', $v + position(), ';')"/></xsl:template>"#,
    );
    let opts = StreamOptions {
        record_path: "/r/n".into(),
        typed_parameters: [("v".into(), 10_i64.into())].into(),
        initial_parameters: [("v".into(), 20_i64.into())].into(),
        ..Default::default()
    };
    let mut output = vec![];
    s.stream_records(Cursor::new("<r><n/><n/></r>"), &mut output, &opts)
        .unwrap();
    assert_eq!(output, b"10:21;10:22;");
}
