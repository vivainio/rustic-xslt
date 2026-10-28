use rust_xslt::{ParameterValue, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn source(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{body}</xsl:stylesheet>"#
    )
}
#[test]
fn initial_typed_templates_use_parameters_and_validate_element_results() {
    let s = Stylesheet::compile(&source(r#"<xsl:template name="entry" as="element()"><xsl:param name="v" as="xs:integer" required="yes"/><out><xsl:sequence select="$v + 1"/></out></xsl:template>"#)).unwrap();
    let opts = TransformOptions {
        initial_template: Some("entry".into()),
        initial_parameters: [("v".into(), ParameterValue::Integer("41".into()))].into(),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("", &opts).unwrap(),
        "<out>42</out>"
    );
}
#[test]
fn sole_sequence_template_returns_apply_function_conversion_without_stringifying() {
    for (body, expected) in [
        (
            r#"<xsl:template match="/" as="xs:decimal"><xsl:sequence select="999999999999999999999"/></xsl:template>"#,
            "999999999999999999999",
        ),
        (
            r#"<xsl:template match="/" as="xs:string"><xsl:sequence select="'text'"/></xsl:template>"#,
            "text",
        ),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(body))
                .unwrap()
                .transform("<r/>")
                .unwrap(),
            expected
        );
    }
    for body in [
        r#"<xsl:template match="/" as="xs:string"><xsl:sequence select="true()"/></xsl:template>"#,
        r#"<xsl:template match="/" as="xs:integer"><xsl:sequence select="'1'"/></xsl:template>"#,
    ] {
        assert_eq!(
            Stylesheet::compile(&source(body))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "XTTE0505"
        );
    }
}
#[test]
fn constructed_template_results_check_cardinality_and_empty_results() {
    for body in [
        r#"<xsl:template match="/" as="element()"><a/><b/></xsl:template>"#,
        r#"<xsl:template match="/" as="element()"/>"#,
    ] {
        assert_eq!(
            Stylesheet::compile(&source(body))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "XTTE0505"
        );
    }
    let s = Stylesheet::compile(&source(r#"<xsl:template match="/" as="element()*"><xsl:for-each select="1 to 3"><n><xsl:value-of select="."/></n></xsl:for-each></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "<n>1</n><n>2</n><n>3</n>");
}
#[test]
fn sole_document_template_results_retain_document_type() {
    let s = Stylesheet::compile(&source(r#"<xsl:template match="/" as="document-node()"><xsl:document><out/></xsl:document></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "<out></out>");
    let s = Stylesheet::compile(&source(r#"<xsl:template match="/" as="element()"><xsl:document><out/></xsl:document></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTTE0505");
}
#[test]
fn unconstrained_template_sequences_keep_normal_output_semantics() {
    let s = Stylesheet::compile(&source(r#"<xsl:template match="/" as="item()*"><xsl:sequence select="1"/><xsl:call-template name="other"/></xsl:template><xsl:template name="other"><xsl:sequence select="2"/></xsl:template>"#)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "1 2");
}
#[test]
fn unsupported_raw_result_shapes_are_rejected_explicitly() {
    let body = r#"<xsl:template match="/" as="xs:string"><xsl:sequence select="'a'"/><out/></xsl:template>"#;
    assert_eq!(
        Stylesheet::compile(&source(body)).unwrap_err().code,
        "RUST_XSLT_UNSUPPORTED"
    );
}
#[test]
fn streamed_typed_record_templates_share_the_conversion_path() {
    let s = Stylesheet::compile(&source(r#"<xsl:template name="record" as="element()"><out><xsl:value-of select="."/></out></xsl:template>"#)).unwrap();
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<r><n>A</n><n>B</n></r>"),
        &mut output,
        &StreamOptions {
            record_path: "/r/n".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output, b"<out>A</out><out>B</out>");
}
