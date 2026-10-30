use rust_xslt::Stylesheet;
fn run(declarations: &str, body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{declarations}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r/>")
}
#[test]
fn use_when_excludes_instructions_and_literal_elements() {
    assert_eq!(
        run("", r#"<out><xsl:text use-when="false()">no</xsl:text><xsl:text use-when="true()">yes</xsl:text><a xsl:use-when="false()"/><b xsl:use-when="true()"/></out>"#).unwrap(),
        "<out>yes<b></b></out>"
    );
    assert_eq!(
        run("", r#"<xsl:value-of select="'x'" use-when="system-property('xsl:version') = '3.0'"/><xsl:value-of select="'y'" use-when="system-property('xsl:version') = '2.0'"/>"#).unwrap(),
        "x"
    );
}
#[test]
fn use_when_excludes_top_level_declarations() {
    let declarations = r#"<xsl:template match="/" use-when="false()">never</xsl:template>"#;
    assert_eq!(
        Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/>{declarations}<xsl:template match="/">kept</xsl:template></xsl:stylesheet>"#))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "kept"
    );
}
#[test]
fn static_variables_drive_conditions_and_remain_ordinary_globals() {
    let declarations = r#"<xsl:variable name="debug" static="yes" select="false()"/><xsl:variable name="level" static="yes" select="3"/>"#;
    assert_eq!(
        run(declarations, r#"<xsl:text use-when="$debug">debug</xsl:text><xsl:text use-when="$level gt 2">deep</xsl:text><xsl:value-of select="$level, $debug"/>"#).unwrap(),
        "deep3 false"
    );
    assert!(run("", r#"<xsl:text use-when="$missing">x</xsl:text>"#).is_err());
}
#[test]
fn availability_functions_report_features() {
    for (expr, expected) in [
        ("element-available('xsl:iterate')", "true"),
        ("element-available('xsl:bogus')", "false"),
        ("function-available('concat')", "true"),
        ("function-available('concat', 2)", "true"),
        ("function-available('fn:sort')", "true"),
        ("function-available('nothing')", "false"),
        ("function-available('math:pi', 0)", "true"),
        ("function-available('abs', 3)", "false"),
        ("type-available('xs:integer')", "true"),
        ("type-available('xs:gYear')", "true"),
        ("system-property('xsl:version')", "3.0"),
        ("system-property('xsl:is-schema-aware')", "no"),
        ("system-property('xsl:nonsense')", ""),
    ] {
        assert_eq!(
            run("", &format!(r#"<xsl:value-of select="{expr}"/>"#)).unwrap(),
            expected,
            "{expr}"
        );
    }
}

#[test]
fn element_available_resolves_call_site_namespaces() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:alias="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><out><xsl:variable name="name" select="'alias:value-of'"/><xsl:value-of select="element-available($name)"/><xsl:text> </xsl:text><alias:value-of select="element-available('xsl:value-of')" xmlns:xsl="urn:other"/><xsl:text> </xsl:text><xsl:value-of select="element-available('value-of')" xmlns="http://www.w3.org/1999/XSL/Transform"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out>true false true</out>"
    );
    let err = run(
        "",
        r#"<xsl:value-of select="element-available('unknown:value-of')"/>"#,
    )
    .unwrap_err();
    assert_eq!(err.code, "XTDE1440");
}
