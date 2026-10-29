use rust_xslt::{Stylesheet, TransformOptions};
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template name="entry" match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
#[test]
fn default_error_function_reports_code_and_description_without_focus() {
    for expr in [
        "error()",
        "error(())",
        "error((), 'description')",
        "error((), 'description', (1,2))",
    ] {
        let s = Stylesheet::compile(&source(expr)).unwrap();
        let opts = TransformOptions {
            initial_template: Some("entry".into()),
            ..Default::default()
        };
        let error = s.transform_with_options("", &opts).unwrap_err();
        assert_eq!(error.code, "FOER0000");
        assert_eq!(error.phase, "run");
        if expr.contains("description") {
            assert_eq!(error.message, "description");
        }
    }
}
#[test]
fn error_is_dynamic_and_lazy_branches_and_empty_domains_skip_it() {
    for (expr, expected) in [
        ("if (true()) then 1 else error()", "1"),
        ("false() and error()", "false"),
        ("true() or error()", "true"),
        ("for $x in () return error()", ""),
        ("some $x in () satisfies error()", "false"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap(),
            expected,
            "{expr}"
        );
    }
}
#[test]
fn error_validates_argument_types_and_arity() {
    for expr in [
        "error('FOER0000')",
        "error((),())",
        "error((),1)",
        "error((1,2))",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "XPTY0004",
            "{expr}"
        );
    }
    assert_eq!(
        Stylesheet::compile(&source("error((), 'x', (), ())"))
            .unwrap_err()
            .code,
        "XPST0017"
    );
}
#[test]
fn error_argument_evaluation_keeps_underlying_dynamic_errors() {
    let s = Stylesheet::compile(&source("error((), 'description', 1 div 0)")).unwrap();
    assert_eq!(s.transform("<r/>").unwrap_err().code, "FOAR0001");
}
