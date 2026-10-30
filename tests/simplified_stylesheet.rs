use rust_xslt::Stylesheet;

#[test]
fn literal_root_is_the_body_of_an_implicit_document_template() {
    let sheet = Stylesheet::compile(r#"<out xsl:version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" count="{count(r/i)}"><xsl:for-each-group select="r/i" group-by="@k"><g key="{current-grouping-key()}"><xsl:value-of select="current-group()" separator=","/></g></xsl:for-each-group></out>"#).unwrap();
    let result = sheet
        .transform("<r><i k='a'>1</i><i k='b'>2</i><i k='a'>3</i></r>")
        .unwrap();
    assert!(
        result.contains(r#"<out count="3"><g key="a">1,3</g><g key="b">2</g></out>"#),
        "{result}"
    );
    assert!(!result.contains("xsl:version"));
    assert!(!result.contains("xmlns:xsl"));
}

#[test]
fn simplified_stylesheets_preserve_namespaces_and_local_variables() {
    let sheet = Stylesheet::compile(r#"<p:out xmlns:p="urn:result" xmlns:d="urn:data" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xsl:version="3.0" xsl:exclude-result-prefixes="d"><xsl:variable name="v" select="d:r/@value"/><xsl:value-of select="$v"/></p:out>"#).unwrap();
    let result = sheet.transform("<r xmlns='urn:data' value='ok'/>").unwrap();
    assert!(
        result.contains(r#"<p:out xmlns:p="urn:result">ok</p:out>"#),
        "{result}"
    );
}

#[test]
fn simplified_stylesheets_validate_version_and_body() {
    assert_eq!(Stylesheet::compile("<out/>").unwrap_err().code, "XTSE0150");
    let prefix = r#"<out xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xsl:version="3.0">"#;
    assert_eq!(
        Stylesheet::compile(&format!("{prefix}<xsl:for-each-group select='r/i'/></out>"))
            .unwrap_err()
            .code,
        "XTSE1080"
    );
    assert_eq!(
        Stylesheet::compile(&format!("{prefix}<xsl:value-of select='$missing'/></out>"))
            .unwrap_err()
            .code,
        "XPST0008"
    );
    assert!(Stylesheet::compile(&format!("{prefix}<xsl:template match='r'/></out>")).is_err());
}

#[test]
fn simplified_version_one_uses_backwards_compatible_value_of() {
    let sheet = Stylesheet::compile(r#"<out xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xsl:version="1.0"><xsl:value-of select="r/i"/></out>"#).unwrap();
    let result = sheet.transform("<r><i>first</i><i>second</i></r>").unwrap();
    assert!(result.contains("<out>first</out>"), "{result}");
}

#[test]
fn literal_default_mode_does_not_change_the_implicit_template_mode() {
    let sheet = Stylesheet::compile(r#"<out xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xsl:version="3.0" xsl:default-mode="local">ok</out>"#).unwrap();
    assert!(sheet.transform("<r/>").unwrap().contains("<out>ok</out>"));
}
