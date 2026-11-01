use rust_xslt::Stylesheet;
fn run(body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r><i>1</i></r>")
}
#[test]
fn on_empty_runs_only_when_the_rest_is_empty() {
    assert_eq!(
        run("<out><xsl:for-each select='r/missing'><x/></xsl:for-each><xsl:on-empty><none/></xsl:on-empty></out>").unwrap(),
        "<out><none></none></out>"
    );
    assert_eq!(
        run("<out><xsl:for-each select='r/i'><x/></xsl:for-each><xsl:on-empty><none/></xsl:on-empty></out>").unwrap(),
        "<out><x></x></out>"
    );
    assert_eq!(
        run("<out><xsl:value-of select=\"'t'\"/><xsl:on-empty select=\"'empty'\"/></out>").unwrap(),
        "<out>t</out>"
    );
}
#[test]
fn on_non_empty_follows_non_empty_content() {
    assert_eq!(
        run("<out><xsl:for-each select='r/i'><x/></xsl:for-each><xsl:on-non-empty><done/></xsl:on-non-empty></out>").unwrap(),
        "<out><x></x><done></done></out>"
    );
    assert_eq!(
        run("<out><xsl:if test='false()'><x/></xsl:if><xsl:on-non-empty><done/></xsl:on-non-empty><xsl:on-empty>nothing</xsl:on-empty></out>").unwrap(),
        "<out>nothing</out>"
    );
    assert_eq!(
        run("<out><xsl:on-empty>always</xsl:on-empty></out>").unwrap(),
        "<out>always</out>"
    );
}

#[test]
fn attributes_and_empty_nodes_are_not_vacuous() {
    for value in ["", "value"] {
        let body = format!(
            r#"<out><xsl:attribute name="a">{value}</xsl:attribute><xsl:on-empty><fallback/></xsl:on-empty></out>"#
        );
        assert_eq!(run(&body).unwrap(), format!(r#"<out a="{value}"></out>"#));
    }
    assert_eq!(
        run(r#"<out><xsl:comment/><xsl:on-empty><fallback/></xsl:on-empty></out>"#).unwrap(),
        "<out><!----></out>"
    );
    assert_eq!(run(r#"<out><xsl:sequence select="('', ())"/><xsl:on-empty><fallback/></xsl:on-empty></out>"#).unwrap(), "<out><fallback></fallback></out>");
}

#[test]
fn vacuous_values_survive_without_an_on_empty_instruction() {
    let body = r#"<xsl:variable name="v" as="item()*"><xsl:sequence select="('', [])"/><xsl:on-non-empty select="'unexpected'"/></xsl:variable><out><xsl:value-of select="count($v)"/></out>"#;
    assert_eq!(run(body).unwrap(), "<out>2</out>");
    let body = r#"<xsl:variable name="v" as="item()*"><xsl:sequence select="('', [])"/><xsl:on-empty/></xsl:variable><out><xsl:value-of select="count($v)"/></out>"#;
    assert_eq!(run(body).unwrap(), "<out>0</out>");
}

#[test]
fn on_non_empty_keeps_its_position_and_local_variable_scope() {
    let body = r#"<out><xsl:variable name="v" select="'header'"/><xsl:on-non-empty><xsl:value-of select="$v"/></xsl:on-non-empty><middle/><xsl:on-non-empty><tail/></xsl:on-non-empty></out>"#;
    assert_eq!(
        run(body).unwrap(),
        "<out>header<middle></middle><tail></tail></out>"
    );
    let body = r#"<out><xsl:on-non-empty><first/></xsl:on-non-empty><xsl:on-non-empty><second/></xsl:on-non-empty><middle/></out>"#;
    assert_eq!(
        run(body).unwrap(),
        "<out><first></first><second></second><middle></middle></out>"
    );
    let body = r#"<out><xsl:variable name="v" select="'fallback'"/><xsl:if test="false()"><middle/></xsl:if><xsl:on-empty><xsl:value-of select="$v"/></xsl:on-empty></out>"#;
    assert_eq!(run(body).unwrap(), "<out>fallback</out>");
}

#[test]
fn sequence_constructors_preserve_scope_and_conditional_content() {
    let body = r#"<out><xsl:sequence><xsl:variable name="v" select="'header'"/><xsl:on-non-empty select="$v"/><xsl:for-each select="r/i"><item/></xsl:for-each><xsl:on-empty><none/></xsl:on-empty></xsl:sequence></out>"#;
    assert_eq!(run(body).unwrap(), "<out>header<item></item></out>");
    let body = r#"<out><xsl:sequence><xsl:for-each select="r/missing"><item/></xsl:for-each><xsl:on-empty><none/></xsl:on-empty></xsl:sequence></out>"#;
    assert_eq!(run(body).unwrap(), "<out><none></none></out>");
    assert_eq!(run("<out><xsl:sequence/></out>").unwrap(), "<out></out>");
    assert_eq!(
        run(
            r#"<out><xsl:sequence><xsl:fallback><bad/></xsl:fallback><item/></xsl:sequence></out>"#
        )
        .unwrap(),
        "<out><item></item></out>"
    );
}

#[test]
fn sequence_select_allows_fallback_but_rejects_constructor_content() {
    let body = r#"<out><xsl:sequence select="'ok'"><xsl:fallback><bad/></xsl:fallback></xsl:sequence><xsl:on-non-empty select="'!'"/></out>"#;
    assert_eq!(run(body).unwrap(), "<out>ok !</out>");
    for body in [
        r#"<out><xsl:sequence select="1"><bad/></xsl:sequence></out>"#,
        r#"<out><xsl:on-empty select="1">text</xsl:on-empty></out>"#,
        r#"<out><xsl:on-non-empty select="1"><bad/></xsl:on-non-empty></out>"#,
        r#"<out><xsl:sequence xml:space="preserve" select="1"> </xsl:sequence></out>"#,
    ] {
        assert_eq!(run(body).unwrap_err().code, "XTSE3185");
    }
}

#[test]
fn on_empty_must_be_the_last_and_only_on_empty_in_its_constructor() {
    for body in [
        r#"<out><xsl:on-empty/><xsl:on-empty/></out>"#,
        r#"<out><xsl:on-empty/><xsl:on-non-empty/></out>"#,
        r#"<out><xsl:on-empty/><next/></out>"#,
        r#"<out><xsl:on-empty/>text</out>"#,
        r#"<out xml:space="preserve"><xsl:on-empty/> </out>"#,
    ] {
        assert_eq!(run(body).unwrap_err().code, "XTSE0010", "{body}");
    }
    let body = r#"<out><xsl:sequence><xsl:on-empty><ok/></xsl:on-empty><!-- ignored --> <xsl:fallback><bad/></xsl:fallback></xsl:sequence></out>"#;
    assert_eq!(run(body).unwrap(), "<out><ok></ok></out>");
    let body = r#"<out><xsl:sequence><xsl:on-empty><ok/></xsl:on-empty><bad xsl:use-when="false()"/></xsl:sequence></out>"#;
    assert_eq!(run(body).unwrap(), "<out><ok></ok></out>");
}
