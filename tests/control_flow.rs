use rust_xslt::{Limits, Stylesheet, TransformOptions, transform};
fn sheet(version: &str, body: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="{version}" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/>{body}</xsl:stylesheet>"#
    )
}

// Adapted from go-xslt TestForEachIfChoose.
#[test]
fn for_each_and_choose() {
    let s = sheet(
        "1.0",
        r#"<xsl:template match="/list"><xsl:for-each select="n"><xsl:choose><xsl:when test=". &gt; 1">big </xsl:when><xsl:otherwise>small </xsl:otherwise></xsl:choose></xsl:for-each></xsl:template>"#,
    );
    assert_eq!(
        transform(&s, "<list><n>1</n><n>5</n><n>0</n></list>").unwrap(),
        "small big small "
    );
}
// Adapted from go-xslt TestParamsAndCallTemplate.
#[test]
fn global_and_template_params() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:param name="greeting">Hello</xsl:param><xsl:template match="/"><xsl:call-template name="say"><xsl:with-param name="who" select="'World'"/></xsl:call-template></xsl:template><xsl:template name="say"><xsl:param name="who"/><xsl:value-of select="$greeting"/>, <xsl:value-of select="$who"/>!</xsl:template>"#)).unwrap();
    let options = TransformOptions {
        parameters: [("greeting".into(), "Hi".into())].into(),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &options).unwrap(),
        "Hi, World!"
    );
    assert_eq!(s.transform("<r/>").unwrap(), "Hello, World!");
}
#[test]
fn scopes_and_focus_do_not_leak() {
    let s = sheet(
        "3.0",
        r#"<xsl:variable name="x" select="'global'"/><xsl:template match="/"><xsl:variable name="x" select="'local'"/><xsl:for-each select="r/n"><xsl:variable name="x" select="concat(position(), '/', last())"/><xsl:value-of select="$x"/><xsl:text>;</xsl:text></xsl:for-each><xsl:value-of select="$x"/><xsl:call-template name="read"/></xsl:template><xsl:template name="read"><xsl:value-of select="$x"/></xsl:template>"#,
    );
    assert_eq!(
        transform(&s, "<r><n/><n/></r>").unwrap(),
        "1/2;2/2;localglobal"
    );
}
#[test]
fn default_params_and_initial_template() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template name="main"><xsl:param name="a" select="2"/><xsl:param name="b" select="$a + 3"/><xsl:value-of select="$b"/></xsl:template>"#)).unwrap();
    let options = TransformOptions {
        initial_template: Some("main".into()),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("<r/>", &options).unwrap(), "5");
}
#[test]
fn global_forward_references_and_cycle() {
    let s = sheet(
        "3.0",
        r#"<xsl:variable name="a" select="$b + 1"/><xsl:variable name="b" select="2"/><xsl:template match="/"><xsl:value-of select="$a"/></xsl:template>"#,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "3");
    let s = sheet(
        "3.0",
        r#"<xsl:variable name="a" select="$b"/><xsl:variable name="b" select="$a"/>"#,
    );
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XTDE0640");
}
#[test]
fn attribute_value_templates() {
    let s = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/r"><out id="{{{@id}}}" count="{count(item)}" quote="{concat('&quot;', '&amp;')}"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        transform(s, "<r id='7'><item/><item/></r>").unwrap(),
        "<out id=\"{7}\" count=\"2\" quote=\"&quot;&amp;\"></out>"
    );
}
#[test]
fn xpath_predicates_axes_union_and_functions() {
    let s = sheet(
        "3.0",
        r#"<xsl:template match="/"><xsl:for-each select="//n[@kind='x'][position()=last()]"><xsl:value-of select="concat(../@id, ':', normalize-space(.), ':', count(ancestor))"/></xsl:for-each><xsl:text>|</xsl:text><xsl:value-of select="count(r/n | r/n)"/><xsl:text>|</xsl:text><xsl:value-of select="sum(r/n)"/></xsl:template>"#,
    );
    assert_eq!(
        transform(
            &s,
            "<r id='A'><n kind='x'> 1 </n><n>2</n><n kind='x'> 3 </n></r>"
        )
        .unwrap(),
        "A:3:0|3|6"
    );
}
#[test]
fn positional_predicates_are_per_parent() {
    let s = sheet(
        "3.0",
        r#"<xsl:template match="/"><xsl:value-of select="//n[1]"/><xsl:text>|</xsl:text><xsl:value-of select="(//n)[1]"/></xsl:template>"#,
    );
    assert_eq!(
        transform(&s, "<r><g><n>A</n><n>B</n></g><g><n>C</n></g></r>").unwrap(),
        "A C|A"
    );
}
#[test]
fn xpath_namespaces_and_attribute_identity() {
    let s = sheet(
        "3.0",
        r#"<xsl:template xmlns:p="urn:p" match="/"><xsl:value-of select="r/p:n/@p:id"/><xsl:text>|</xsl:text><xsl:value-of select="count(r/p:n/@p:id | r/p:n/@id)"/></xsl:template>"#,
    );
    assert_eq!(
        transform(&s, "<r xmlns:p='urn:p'><p:n p:id='yes' id='no'/></r>").unwrap(),
        "yes|2"
    );
}
#[test]
fn short_circuit_and_numeric_precedence() {
    let s = sheet(
        "3.0",
        r#"<xsl:template match="/"><xsl:value-of select="2 + 3 * 4 - 5"/><xsl:text>|</xsl:text><xsl:value-of select="false() and count('bad'/n)"/><xsl:text>|</xsl:text><xsl:value-of select="not(())"/></xsl:template>"#,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "9|false|true");
}
#[test]
fn backwards_value_of_and_newer_sequences() {
    for (version, expected) in [("1.0", "A"), ("3.0", "A B")] {
        let s = sheet(
            version,
            r#"<xsl:template match="/"><xsl:value-of select="r/n"/></xsl:template>"#,
        );
        assert_eq!(transform(&s, "<r><n>A</n><n>B</n></r>").unwrap(), expected);
    }
}
#[test]
fn recursion_and_predicates_share_execution_budget() {
    let s = Stylesheet::compile(&sheet("3.0", r#"<xsl:template match="/"><xsl:call-template name="loop"/></xsl:template><xsl:template name="loop"><xsl:call-template name="loop"/></xsl:template>"#)).unwrap();
    assert_eq!(
        s.transform_with_limits(
            "<r/>",
            Limits {
                max_steps: 25,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0130"
    );
    let s = Stylesheet::compile(&sheet(
        "3.0",
        r#"<xsl:template match="/"><xsl:value-of select="count(//n[true()])"/></xsl:template>"#,
    ))
    .unwrap();
    let source = format!("<r>{}</r>", "<n/>".repeat(100));
    assert_eq!(
        s.transform_with_limits(
            &source,
            Limits {
                max_steps: 50,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XPDY0130"
    );
}
#[test]
fn invalid_expressions_and_structure_are_errors() {
    for expression in [
        "'unterminated",
        "a[",
        "concat('x')",
        "missing()",
        "p:x",
        "1 +",
        "foo//",
        "true(1)",
    ] {
        let s = sheet(
            "3.0",
            &format!(
                r#"<xsl:template match="/"><xsl:value-of select="{expression}"/></xsl:template>"#
            ),
        );
        assert!(Stylesheet::compile(&s).is_err(), "{expression}");
    }
    assert!(
        Stylesheet::compile(&sheet(
            "3.0",
            r#"<xsl:template match="/"><xsl:choose><xsl:otherwise/></xsl:choose></xsl:template>"#
        ))
        .is_err()
    );
}

#[test]
fn lexical_errors_are_reported_in_dead_branches() {
    for body in [
        r#"<xsl:template match="/"><xsl:if test="false()"><xsl:value-of select="$missing"/></xsl:if></xsl:template>"#,
        r#"<xsl:template match="/"><xsl:if test="false()"><xsl:call-template name="missing"/></xsl:if></xsl:template>"#,
        r#"<xsl:template match="/"><xsl:if test="true()"><xsl:variable name="private" select="1"/></xsl:if><xsl:value-of select="$private"/></xsl:template>"#,
    ] {
        assert!(Stylesheet::compile(&sheet("3.0", body)).is_err());
    }
}

#[test]
fn named_initial_template_can_run_without_a_source_document() {
    let options = TransformOptions {
        initial_template: Some("main".into()),
        ..Default::default()
    };
    let s = Stylesheet::compile(&sheet(
        "3.0",
        r#"<xsl:template name="main"><xsl:text>no source needed</xsl:text></xsl:template>"#,
    ))
    .unwrap();
    assert_eq!(
        s.transform_with_options("", &options).unwrap(),
        "no source needed"
    );
    for expression in [".", "/", "string()", "position()"] {
        let s = Stylesheet::compile(&sheet(
            "3.0",
            &format!(
                r#"<xsl:template name="main"><xsl:value-of select="{expression}"/></xsl:template>"#
            ),
        ))
        .unwrap();
        assert_eq!(
            s.transform_with_options("", &options).unwrap_err().code,
            "XPDY0002",
            "{expression}"
        );
    }
}
