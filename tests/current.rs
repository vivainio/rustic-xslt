use rust_xslt::Stylesheet;

fn run(declarations: &str, body: &str, source: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/>{declarations}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?.transform(source)
}

#[test]
fn current_retains_the_outer_item_across_predicates_and_path_steps() {
    let body = r#"<xsl:for-each select="r/ref"><xsl:value-of select="../item[@id=current()/@to]"/><xsl:value-of select="../item/(current()/@to)" separator=","/></xsl:for-each>"#;
    assert_eq!(
        run(
            "",
            body,
            "<r><item id='a'>A</item><item id='b'>B</item><ref to='b'/></r>"
        )
        .unwrap(),
        "Bb"
    );
    let atomic = r#"<xsl:for-each select="(2, 3)"><xsl:value-of select="(1,2,3)[. = current()]"/></xsl:for-each>"#;
    assert_eq!(run("", atomic, "<r/>").unwrap(), "23");
}

#[test]
fn current_is_reset_for_each_stylesheet_expression() {
    let body = r#"<xsl:for-each select="r/n"><xsl:variable name="saved" select="current()"/><xsl:for-each select="@id"><xsl:value-of select="concat(current(), ':', $saved/@id, ';')"/></xsl:for-each></xsl:for-each>"#;
    assert_eq!(
        run("", body, "<r><n id='a'/><n id='b'/></r>").unwrap(),
        "a:a;b:b;"
    );
}

#[test]
fn current_in_patterns_refers_to_the_candidate_not_its_ancestors() {
    let declarations = r#"<xsl:template match="r/n[preceding-sibling::n[@id = current()/@id]]">D</xsl:template><xsl:template match="n">N</xsl:template>"#;
    assert_eq!(
        run(
            declarations,
            r#"<xsl:apply-templates select="r/n"/>"#,
            "<r><n id='a'/><n id='b'/><n id='a'/></r>"
        )
        .unwrap(),
        "NND"
    );
}

#[test]
fn current_in_keys_uses_the_indexed_node() {
    let declarations = r#"<xsl:key name="k" match="n" use="../label[@id=current()/@id]"/>"#;
    let body = r#"<xsl:value-of select="key('k', 'second')/@id"/>"#;
    assert_eq!(
        run(
            declarations,
            body,
            "<r><label id='a'>first</label><label id='b'>second</label><n id='a'/><n id='b'/></r>"
        )
        .unwrap(),
        "b"
    );
}

#[test]
fn current_without_an_outer_context_raises_the_xslt_error() {
    let declarations = r#"<xsl:function name="f:no-context"><xsl:sequence select="(1,2)[. = current()]"/></xsl:function>"#;
    assert_eq!(
        run(
            declarations,
            r#"<xsl:sequence select="f:no-context()"/>"#,
            "<r/>"
        )
        .unwrap_err()
        .code,
        "XTDE1360"
    );
}

#[test]
fn current_function_references_retain_their_creation_item() {
    let body = r#"<xsl:for-each select="r/n"><xsl:variable name="get" select="current#0"/><xsl:variable name="lookup" select="function-lookup(QName('http://www.w3.org/2005/xpath-functions','current'), 0)"/><xsl:for-each select="@id"><xsl:value-of select="concat($get()/@id, $lookup()/@id, ':', local-name-from-QName(function-name($get)), ':', function-arity($get), ';')"/></xsl:for-each></xsl:for-each>"#;
    assert_eq!(
        run("", body, "<r><n id='a'/><n id='b'/></r>").unwrap(),
        "aa:current:0;bb:current:0;"
    );
}
