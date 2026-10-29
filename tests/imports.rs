use rust_xslt::{Diagnostic, Stylesheet};
const HEAD: &str =
    r#"version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f""#;
fn module(name: &str) -> Result<String, Diagnostic> {
    let body = match name {
        "base.xsl" => {
            r#"<xsl:output method="text"/><xsl:variable name="v" select="'base'"/><xsl:template match="r">[base r:<xsl:apply-templates/>]</xsl:template><xsl:template match="b">base b</xsl:template><xsl:template name="hello">base hello</xsl:template><xsl:function name="f:who"><xsl:sequence select="'base'"/></xsl:function>"#
        }
        "mid.xsl" => {
            r#"<xsl:import href="base.xsl"/><xsl:variable name="v" select="'mid'"/><xsl:template match="r">[mid r:<xsl:apply-imports/>]</xsl:template><xsl:template name="hello">mid hello</xsl:template><xsl:function name="f:who"><xsl:sequence select="'mid'"/></xsl:function>"#
        }
        "other.xsl" => r#"<xsl:template match="b">other b</xsl:template>"#,
        "late.xsl" => r#"<xsl:import href="other.xsl"/>"#,
        _ => {
            return Err(Diagnostic {
                code: "XTSE0165".into(),
                message: format!("cannot read {name}"),
                phase: "compile",
                line: 0,
                column: 0,
            });
        }
    };
    Ok(format!(r#"<xsl:stylesheet {HEAD}>{body}</xsl:stylesheet>"#))
}
fn run(main: &str, source: &str) -> Result<String, Diagnostic> {
    Stylesheet::compile_with_resolver(
        &format!(r#"<xsl:stylesheet {HEAD}>{main}</xsl:stylesheet>"#),
        &module,
    )?
    .transform(source)
}
#[test]
fn importing_modules_outrank_imported_ones() {
    let main = r#"<xsl:import href="base.xsl"/><xsl:template match="b">main b</xsl:template>"#;
    assert_eq!(run(main, "<r><b/></r>").unwrap(), "[base r:main b]");
    let chain = r#"<xsl:import href="mid.xsl"/>"#;
    assert_eq!(
        run(chain, "<r><b/></r>").unwrap(),
        "[mid r:[base r:base b]]"
    );
}
#[test]
fn imported_variables_functions_and_named_templates_are_overridden() {
    let main = r#"<xsl:import href="mid.xsl"/><xsl:template match="/"><xsl:value-of select="$v, f:who()"/><xsl:call-template name="hello"/></xsl:template>"#;
    assert_eq!(run(main, "<r/>").unwrap(), "mid midmid hello");
    let override_all = r#"<xsl:import href="mid.xsl"/><xsl:variable name="v" select="'main'"/><xsl:function name="f:who"><xsl:sequence select="'main'"/></xsl:function><xsl:template name="hello">main hello</xsl:template><xsl:template match="/"><xsl:value-of select="$v, f:who()"/><xsl:call-template name="hello"/></xsl:template>"#;
    assert_eq!(run(override_all, "<r/>").unwrap(), "main mainmain hello");
}
#[test]
fn later_imports_outrank_earlier_ones_and_include_shares_precedence() {
    let two = r#"<xsl:import href="base.xsl"/><xsl:import href="other.xsl"/>"#;
    assert_eq!(run(two, "<r><b/></r>").unwrap(), "[base r:other b]");
    let included = r#"<xsl:import href="base.xsl"/><xsl:include href="other.xsl"/>"#;
    assert_eq!(run(included, "<r><b/></r>").unwrap(), "[base r:other b]");
}
#[test]
fn same_precedence_conflicts_and_misplaced_imports_are_errors() {
    let conflict = r#"<xsl:variable name="x" select="1"/><xsl:variable name="x" select="2"/>"#;
    assert_eq!(run(conflict, "<r/>").unwrap_err().code, "XTSE0630");
    let late = r#"<xsl:template match="/"/><xsl:import href="base.xsl"/>"#;
    assert_eq!(run(late, "<r/>").unwrap_err().code, "XTSE0200");
    let nested = r#"<xsl:include href="late.xsl"/>"#;
    assert!(run(nested, "<r/>").is_err());
    assert!(
        Stylesheet::compile(&format!(
            r#"<xsl:stylesheet {HEAD}><xsl:import href="base.xsl"/></xsl:stylesheet>"#
        ))
        .is_err()
    );
    let apply = r#"<xsl:template match="/"><xsl:apply-imports/></xsl:template>"#;
    assert!(run(apply, "<r/>").is_ok());
    let unknown = r#"<xsl:import href="missing.xsl"/>"#;
    assert_eq!(run(unknown, "<r/>").unwrap_err().code, "XTSE0165");
}
