use rust_xslt::Stylesheet;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression))
        .unwrap()
        .transform("<r/>")
        .unwrap()
}
#[test]
fn qname_constructor_preserves_prefix_expanded_identity_and_atomic_type() {
    for (expr, expected) in [
        ("QName('urn:p','p:n') instance of xs:QName", "true"),
        ("QName('urn:p','p:n') instance of xs:anyAtomicType", "true"),
        ("QName('urn:p','p:n') instance of xs:string", "false"),
        ("string(QName('urn:p','p:n'))", "p:n"),
        ("string(QName('urn:p','n'))", "n"),
        ("string(QName((),'n'))", "n"),
        ("QName('urn:p','p:n') eq QName('urn:p','q:n')", "true"),
        ("QName('urn:p','p:n') ne QName('urn:q','p:n')", "true"),
        ("QName('urn:p','p:n') = QName('urn:p','q:n')", "true"),
        (
            "deep-equal(QName('urn:p','p:n'), QName('urn:p','q:n'))",
            "true",
        ),
        ("deep-equal(QName('urn:p','p:n'), 'p:n')", "false"),
        ("xs:untypedAtomic(QName('urn:p','p:n'))", "p:n"),
        ("string(QName('urn:p','é:界'))", "é:界"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn qname_constructor_checks_xml_names_namespace_requirements_and_types() {
    for (expr, code) in [
        ("QName((),'p:n')", "FOCA0002"),
        ("QName('','p:n')", "FOCA0002"),
        ("QName('urn:p','p:a:b')", "FOCA0002"),
        ("QName('urn:p','1n')", "FOCA0002"),
        ("QName('urn:p','p: n')", "FOCA0002"),
        ("QName('urn:p','p:')", "FOCA0002"),
        ("QName('urn:p','')", "FOCA0002"),
        ("QName(1,'n')", "XPTY0004"),
        ("QName('urn:p',())", "XPTY0004"),
        ("QName(('urn:p','urn:q'),'n')", "XPTY0004"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
}
#[test]
fn qname_values_reject_order_boolean_and_numeric_operations() {
    for (expr, code) in [
        ("QName('urn:p','n') lt QName('urn:p','n')", "XPTY0004"),
        ("QName('urn:p','n') eq 'n'", "XPTY0004"),
        ("boolean(QName('urn:p','n'))", "FORG0006"),
        ("xs:integer(QName('urn:p','n'))", "XPTY0004"),
        ("xs:boolean(QName('urn:p','n'))", "XPTY0004"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
    assert_eq!(
        evaluate("QName('urn:p','n') castable as xs:integer"),
        "false"
    );
    assert_eq!(evaluate("QName('urn:p','n') cast as xs:string"), "n");
}
#[test]
fn qname_typed_template_results_bindings_and_copies_preserve_values() {
    let body = r#"<xsl:template name="q" as="xs:QName"><xsl:sequence select="QName('urn:p','p:n')"/></xsl:template><xsl:template match="/"><xsl:variable name="q" as="xs:QName"><xsl:call-template name="q"/></xsl:variable><xsl:variable name="v" as="xs:QName+"><xsl:sequence select="$q"/><xsl:copy-of select="$q"/><xsl:copy select="$q"/></xsl:variable><xsl:value-of select="concat(count($v), ':', $v[2] instance of xs:QName, ':', $v[3] eq $q)"/></xsl:template>"#;
    let style = source("1").replace(
        r#"<xsl:template match="/"><xsl:sequence select="1"/></xsl:template>"#,
        body,
    );
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "3:true:true"
    );
}
#[test]
fn custom_error_codes_retain_expanded_names_and_standard_error_short_names() {
    for (expr, code) in [
        (
            "error(QName('http://www.w3.org/2005/xqt-errors','e:FOER0000'),'description')",
            "FOER0000",
        ),
        (
            "error(QName('urn:errors','p:problem'),'description')",
            "Q{urn:errors}problem",
        ),
        ("error(QName('','FOER0000'),'description')", "Q{}FOER0000"),
    ] {
        let error = Stylesheet::compile(&source(expr))
            .unwrap()
            .transform("<r/>")
            .unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(error.message, "description");
    }
}
#[test]
fn qname_constructors_and_casts_resolve_static_namespaces() {
    for (expr, expected) in [
        ("xs:QName('p:n') eq QName('urn:p','q:n')", "true"),
        ("'p:n' cast as xs:QName eq QName('urn:p','q:n')", "true"),
        ("xs:QName(' n ') eq QName('','n')", "true"),
        (
            "xs:QName(QName('urn:other','p:n')) eq QName('urn:other','q:n')",
            "true",
        ),
        ("empty(xs:QName(()))", "true"),
        ("empty(() cast as xs:QName?)", "true"),
        ("'p:n' castable as xs:QName", "true"),
        ("'z:n' castable as xs:QName", "false"),
        ("1 castable as xs:QName", "false"),
        ("() castable as xs:QName", "false"),
        ("() castable as xs:QName?", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
    for (expr, code) in [
        ("xs:QName('z:n')", "FONS0004"),
        ("xs:QName('1n')", "FORG0001"),
        ("1 cast as xs:QName", "XPTY0004"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code
        );
    }
}
#[test]
fn qname_cast_scope_validation_traverses_the_operand() {
    assert_eq!(
        Stylesheet::compile(&source("xs:QName($missing)"))
            .unwrap_err()
            .code,
        "XPST0008"
    );
}

#[test]
fn qname_conversion_uses_its_declaring_scope_and_not_source_or_xml_default_namespaces() {
    let body = r#"<xsl:template match="/"><xsl:variable name="outer" select="xs:QName(r/@a)"/><xsl:variable name="inner" xmlns:p="urn:inner" select="xs:QName('p:n')"/><xsl:value-of select="concat($outer eq QName('urn:p','n'), ':', $inner eq QName('urn:inner','n'), ':', xs:QName('n') eq QName('','n'))"/></xsl:template>"#;
    let style = source("1")
        .replace(
            r#"<xsl:template match="/"><xsl:sequence select="1"/></xsl:template>"#,
            body,
        )
        .replace("version=\"3.0\"", "version=\"3.0\" xmlns=\"urn:literal\"");
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .transform("<r xmlns:p='urn:source' a='p:n'/>")
            .unwrap(),
        "true:true:true"
    );
}
#[test]
fn qname_casts_preserve_streaming_rejection_and_budget_errors() {
    use rust_xslt::{Limits, StreamOptions, TransformOptions};
    use std::io::Cursor;
    let style = source("xs:QName(string(root()))").replace("match=\"/\"", "name=\"record\"");
    let mut output = vec![];
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .stream_records(
                Cursor::new("not XML"),
                &mut output,
                &StreamOptions::default()
            )
            .unwrap_err()
            .code,
        "RUST_XSLT_STREAM_UNSUPPORTED"
    );
    assert!(output.is_empty());
    let expr = format!("'{}' castable as xs:QName", "n".repeat(100));
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 20,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        Stylesheet::compile(&source(&expr))
            .unwrap()
            .transform_with_options("<r/>", &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}
