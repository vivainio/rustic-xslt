use rust_xslt::{Diagnostic, Limits, Stylesheet, transform};
fn sheet(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{body}</xsl:stylesheet>"#
    )
}

// Adapted from go-xslt/internal/xslt/transform_test.go, TestValueOfAndApply.
#[test]
fn value_of_and_apply() {
    let s = sheet(
        r#"<xsl:template match="/catalog"><result><xsl:apply-templates select="book"/></result></xsl:template><xsl:template match="book"><item><xsl:value-of select="title"/></item></xsl:template>"#,
    );
    assert_eq!(
        transform(
            &s,
            "<catalog><book><title>A</title></book><book><title>B</title></book></catalog>"
        )
        .unwrap(),
        "<result><item>A</item><item>B</item></result>"
    );
}
#[test]
fn builtin_rules_and_escaping() {
    let s = sheet(
        r#"<xsl:template match="/"><out label="&quot;&amp;"><xsl:apply-templates/></out></xsl:template>"#,
    );
    assert_eq!(
        transform(&s, "<doc>Hello <b>&lt;world&gt;</b><!--ignored--></doc>").unwrap(),
        "<out label=\"&quot;&amp;\">Hello &lt;world&gt;</out>"
    );
}
#[test]
fn compile_rejects_unimplemented_semantics() {
    for body in [
        r#"<xsl:template match="/"><xsl:message/></xsl:template>"#,
        r#"<xsl:template match="/"><xsl:value-of select="unknown-function()"/></xsl:template>"#,
        r#"<xsl:template match="/" mode="m"/>"#,
        r#"<xsl:template match="/"><out xmlns="urn:unsupported"/></xsl:template>"#,
    ] {
        let err = Stylesheet::compile(&sheet(body)).unwrap_err();
        assert!(["RUST_XSLT_UNSUPPORTED", "XPST0017"].contains(&err.code.as_str()));
        assert!(err.line > 0);
    }
}
#[test]
fn static_default_priority_and_last_declaration() {
    let s = sheet(
        r#"<xsl:template match="/r"><a/></xsl:template><xsl:template match="r"><b/></xsl:template><xsl:template match="/r"><c/></xsl:template>"#,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "<c></c>");
}
#[test]
fn budgets_stop_self_application_and_output() {
    let s = Stylesheet::compile(&sheet(
        r#"<xsl:template match="/"><xsl:apply-templates select="."/></xsl:template>"#,
    ))
    .unwrap();
    assert_eq!(
        s.transform_with_limits(
            "<r/>",
            Limits {
                max_steps: 100,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0130"
    );
    let s = Stylesheet::compile(&sheet(
        r#"<xsl:template match="/"><xsl:text>12345</xsl:text></xsl:template>"#,
    ))
    .unwrap();
    assert!(
        s.transform_with_limits(
            "<r/>",
            Limits {
                max_output_bytes: 4,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn rejects_malformed_xml_and_external_entities() {
    let s = Stylesheet::compile(&sheet("")).unwrap();
    assert_eq!(s.transform("<r>").unwrap_err().code, "FODC0006");
    assert!(
        s.transform(r#"<!DOCTYPE r [<!ENTITY x SYSTEM "file:///etc/passwd">]><r>&x;</r>"#)
            .is_err()
    );
}
#[test]
fn compiled_sheet_is_shareable() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Stylesheet>();
    let s = Stylesheet::compile(&sheet("")).unwrap();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| s.transform("<r>ok</r>")))
            .collect();
        for h in handles {
            assert_eq!(h.join().unwrap().unwrap(), "ok");
        }
    });
}
#[test]
fn text_method_and_text_node_value() -> Result<(), Diagnostic> {
    let s = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output method="text"/><xsl:template match="/"><out><xsl:value-of select="r"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(transform(s, "<r>&lt;a&gt;</r>")?, "<a>");
    Ok(())
}

#[test]
fn text_comments_are_not_output_and_pattern_whitespace_is_ignored() {
    let s =
        sheet(r#"<xsl:template match=" / "><xsl:text>a<!--comment-->b</xsl:text></xsl:template>"#);
    assert_eq!(transform(&s, "<r/>").unwrap(), "ab");
}
