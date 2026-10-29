use rust_xslt::{Diagnostic, Stylesheet};
fn main_sheet(body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:include href="lib.xsl"/>{body}</xsl:stylesheet>"#
    )
}
fn resolver(href: &str) -> Result<String, Diagnostic> {
    match href {
        "lib.xsl" => Ok(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:include href="deep.xsl"/><xsl:variable name="greeting" select="'hi'"/><xsl:template name="hello">hello </xsl:template><xsl:function name="f:twice"><xsl:param name="x"/><xsl:sequence select="$x * 2"/></xsl:function></xsl:stylesheet>"#.into()),
        "deep.xsl" => Ok(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="b">B<xsl:apply-templates/></xsl:template></xsl:stylesheet>"#.into()),
        "loop.xsl" => Ok(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:include href="loop.xsl"/></xsl:stylesheet>"#.into()),
        "bad.xsl" => Ok("<notxslt/>".into()),
        _ => Err(Diagnostic {
            code: "XTSE0165".into(),
            message: format!("cannot read {href}"),
            phase: "compile",
            line: 0,
            column: 0,
        }),
    }
}
#[test]
fn included_modules_contribute_all_declaration_kinds() {
    let sheet = Stylesheet::compile_with_resolver(
        &main_sheet(r#"<xsl:template match="/"><xsl:call-template name="hello"/><xsl:value-of select="$greeting, f:twice(21)"/><xsl:apply-templates select="r/b"/></xsl:template>"#),
        &resolver,
    )
    .unwrap();
    assert_eq!(sheet.transform("<r><b/></r>").unwrap(), "hello hi 42B");
}
#[test]
fn include_errors_are_reported() {
    let without = Stylesheet::compile(&main_sheet("")).unwrap_err();
    assert_eq!(without.code, "RUST_XSLT_UNSUPPORTED");
    let missing = main_sheet("").replace("lib.xsl", "missing.xsl");
    assert_eq!(
        Stylesheet::compile_with_resolver(&missing, &resolver)
            .unwrap_err()
            .code,
        "XTSE0165"
    );
    let looping = main_sheet("").replace("lib.xsl", "loop.xsl");
    assert_eq!(
        Stylesheet::compile_with_resolver(&looping, &resolver)
            .unwrap_err()
            .code,
        "XTSE0180"
    );
    let bad = main_sheet("").replace("lib.xsl", "bad.xsl");
    assert!(Stylesheet::compile_with_resolver(&bad, &resolver).is_err());
}
