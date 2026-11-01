use rust_xslt::Stylesheet;
fn run(body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:err="http://www.w3.org/2005/xqt-errors" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:f="urn:f"><xsl:output method="text"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r><n>1</n></r>")
}
#[test]
fn successful_try_bodies_pass_their_output_through() {
    assert_eq!(
        run("a<xsl:try><xsl:value-of select='1 + 1'/><xsl:catch>caught</xsl:catch></xsl:try>b")
            .unwrap(),
        "a2b"
    );
    assert_eq!(
        run("<xsl:try select='r/n + 1'><xsl:catch select=\"'x'\"/></xsl:try>").unwrap(),
        "2"
    );
}
#[test]
fn caught_errors_discard_partial_output_and_expose_error_variables() {
    let body = r#"<xsl:try>partial<xsl:value-of select="1 div 0 idiv 0"/>more<xsl:catch>[<xsl:value-of select="local-name-from-QName($err:code)"/>|<xsl:value-of select="namespace-uri-from-QName($err:code)"/>|<xsl:value-of select="exists($err:description)"/>]</xsl:catch></xsl:try>"#;
    assert_eq!(
        run(body).unwrap(),
        "[FOAR0001|http://www.w3.org/2005/xqt-errors|true]"
    );
    let custom = r#"<xsl:try><xsl:value-of select="error(xs:QName('f:boom'), 'bad')"/><xsl:catch><xsl:value-of select="$err:description"/></xsl:catch></xsl:try>"#;
    assert_eq!(run(custom).unwrap(), "bad");
}
#[test]
fn catch_clauses_select_by_error_name() {
    let body = r#"<xsl:try><xsl:value-of select="error(xs:QName('f:boom'), 'bad')"/><xsl:catch errors="err:FOAR0001">arith</xsl:catch><xsl:catch errors="f:boom">boom</xsl:catch><xsl:catch>other</xsl:catch></xsl:try>"#;
    assert_eq!(run(body).unwrap(), "boom");
    let arith = body.replace("error(xs:QName('f:boom'), 'bad')", "1 div 0 idiv 0");
    assert_eq!(run(&arith).unwrap(), "arith");
    let fallback = r#"<xsl:try><xsl:value-of select="error()"/><xsl:catch errors="err:FOAR0001">no</xsl:catch><xsl:catch errors="*:FOER0000 err:XYZ">yes</xsl:catch></xsl:try>"#;
    assert_eq!(run(fallback).unwrap(), "yes");
    let uncaught = r#"<xsl:try><xsl:value-of select="error()"/><xsl:catch errors="err:FOAR0001">no</xsl:catch></xsl:try>"#;
    assert_eq!(run(uncaught).unwrap_err().code, "FOER0000");
}
#[test]
fn nested_tries_and_errors_inside_catch_blocks() {
    let nested = r#"<xsl:try><xsl:try><xsl:value-of select="error()"/><xsl:catch errors="err:FOAR0001">inner</xsl:catch></xsl:try><xsl:catch>outer</xsl:catch></xsl:try>"#;
    assert_eq!(run(nested).unwrap(), "outer");
    let rethrow = r#"<xsl:try><xsl:try><xsl:value-of select="error()"/><xsl:catch><xsl:value-of select="error(xs:QName('f:again'), 'x')"/></xsl:catch></xsl:try><xsl:catch errors="f:again">again</xsl:catch></xsl:try>"#;
    assert_eq!(run(rethrow).unwrap(), "again");
    assert!(run("<xsl:try>x</xsl:try>").is_err());
    assert!(run("<xsl:try><xsl:catch/>text</xsl:try>").is_err());
}

#[test]
fn try_catch_preserves_raw_items_and_discards_partial_results() {
    let success = r#"<xsl:variable name="v" as="item()*"><xsl:try><xsl:sequence select="(1, 2)"/><xsl:catch select="0"/></xsl:try></xsl:variable><xsl:value-of select="sum($v)"/>"#;
    assert_eq!(run(success).unwrap(), "3");
    let caught = r#"<xsl:variable name="v" as="item()*"><xsl:try><xsl:sequence select="(1, 2)"/><xsl:sequence select="error()"/><xsl:catch select="(3, 4)"/></xsl:try></xsl:variable><xsl:value-of select="sum($v)"/>"#;
    assert_eq!(run(caught).unwrap(), "7");
    let filtered = r#"<xsl:where-populated><xsl:try><xsl:sequence select="('', 'keep')"/><xsl:catch select="'bad'"/></xsl:try></xsl:where-populated>"#;
    assert_eq!(run(filtered).unwrap(), "keep");
    let conditional = r#"<xsl:sequence><xsl:try><xsl:sequence select="error()"/><xsl:catch select="()"/></xsl:try><xsl:on-empty select="'empty'"/></xsl:sequence>"#;
    assert_eq!(run(conditional).unwrap(), "empty");
    let structured = r#"<xsl:variable name="v" as="item()*"><xsl:try><node>text</node><xsl:sequence select="map{'k': 3}"/><xsl:catch select="()"/></xsl:try></xsl:variable><xsl:value-of select="concat($v[1], ':', $v[2]?k)"/>"#;
    assert_eq!(run(structured).unwrap(), "text:3");
}

#[test]
fn try_results_can_supply_attributes_and_atomic_items_to_an_element() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><out><xsl:try><xsl:attribute name="a">value</xsl:attribute><xsl:sequence select="(1, 2)"/><xsl:catch><xsl:attribute name="bad">bad</xsl:attribute></xsl:catch></xsl:try></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out a=\"value\">1 2</out>"
    );
    let caught = source.replace(
        r#"<xsl:sequence select="(1, 2)"/>"#,
        r#"<xsl:sequence select="error()"/>"#,
    );
    assert_eq!(
        Stylesheet::compile(&caught)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out bad=\"bad\"></out>"
    );
    let simple = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><out><xsl:attribute name="a"><xsl:try><xsl:sequence select="(1, 2)"/><xsl:catch select="'bad'"/></xsl:try></xsl:attribute></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(simple)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out a=\"12\"></out>"
    );
}
