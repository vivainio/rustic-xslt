use rust_xslt::Stylesheet;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression))
        .unwrap()
        .transform("<r a=' n '/>")
        .unwrap()
}
#[test]
fn string_restrictions_apply_their_whitespace_facets() {
    for (expr, expected) in [
        ("xs:normalizedString(' a&#x9; b&#xA;c ')", " a  b c "),
        ("xs:token(' a&#x9; b&#xA;c ')", "a b c"),
        ("xs:NCName(' n ')", "n"),
        ("xs:language(' en-US ')", "en-US"),
        ("xs:ID(' n ')", "n"),
        ("xs:IDREF(' n ')", "n"),
        ("xs:ENTITY(' n ')", "n"),
        ("xs:Name(' p::n ')", "p::n"),
        ("xs:NMTOKEN(' 123:n ')", "123:n"),
        ("xs:token('')", ""),
        ("empty(xs:NCName(()))", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn string_restriction_hierarchy_tracks_types_rather_than_lexical_shape() {
    for (expr, expected) in [
        ("xs:ID('n') instance of xs:NCName", "true"),
        ("xs:NCName('n') instance of xs:Name", "true"),
        ("xs:Name('n') instance of xs:token", "true"),
        ("xs:language('en') instance of xs:normalizedString", "true"),
        ("xs:NCName('n') instance of xs:string", "true"),
        ("xs:NMTOKEN('n') instance of xs:Name", "false"),
        ("'n' instance of xs:NCName", "false"),
        ("xs:NCName('n') instance of xs:ID", "false"),
        ("xs:string(xs:NCName('n')) instance of xs:NCName", "false"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn lexical_restrictions_validate_unicode_xml_names_and_language_tags() {
    for expr in [
        "xs:NCName('p:n')",
        "xs:NCName('1n')",
        "xs:ID('a b')",
        "xs:ENTITY('')",
        "xs:NMTOKEN('a b')",
        "xs:Name('1n')",
        "xs:language('123')",
        "xs:language('abcdefghi')",
        "xs:language('en--US')",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "FORG0001",
            "{expr}"
        );
    }
    assert_eq!(evaluate("xs:NCName('é界')"), "é界");
    assert_eq!(evaluate("xs:Name(':é:界')"), ":é:界");
    assert_eq!(evaluate("'p:n' castable as xs:NCName"), "false");
    assert_eq!(
        evaluate("'n' cast as xs:NCName instance of xs:NCName"),
        "true"
    );
}
#[test]
fn derived_strings_participate_in_primitive_string_operations_and_casts() {
    for (expr, expected) in [
        ("xs:NCName('n') eq 'n'", "true"),
        ("deep-equal(xs:token('n'), 'n')", "true"),
        ("upper-case(xs:NCName('name'))", "NAME"),
        ("boolean(xs:token(''))", "false"),
        ("xs:integer(xs:token('12'))", "12"),
        ("xs:anyURI(xs:token('urn:p')) instance of xs:anyURI", "true"),
        ("xs:QName(xs:Name('p:n')) eq QName('urn:p','n')", "true"),
        (
            "min((xs:NCName('a'),xs:token('b'))) instance of xs:NCName",
            "true",
        ),
        (
            "min((xs:NCName('a'),xs:anyURI('b'))) instance of xs:NCName",
            "false",
        ),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn qname_accessors_return_ncname_subtypes_and_preserve_empty_results() {
    for (expr, expected) in [
        ("prefix-from-QName(QName('urn:p','p:n'))", "p"),
        (
            "prefix-from-QName(QName('urn:p','p:n')) instance of xs:NCName",
            "true",
        ),
        (
            "local-name-from-QName(QName('urn:p','p:n')) instance of xs:NCName",
            "true",
        ),
        ("local-name-from-QName(QName('urn:p','p:é'))", "é"),
        ("empty(prefix-from-QName(QName('urn:p','n')))", "true"),
        ("empty(local-name-from-QName(()))", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
    assert_eq!(
        Stylesheet::compile(&source("prefix-from-QName(r)"))
            .unwrap()
            .transform("<r/>")
            .unwrap_err()
            .code,
        "XPTY0117"
    );
}
#[test]
fn typed_bindings_accept_string_subtypes_without_erasing_them() {
    let body = r#"<xsl:template match="/"><xsl:variable name="n" as="xs:NCName" select="r/@a"/><xsl:variable name="s" as="xs:string"><xsl:copy-of select="$n"/></xsl:variable><xsl:variable name="v" as="xs:token+"><xsl:sequence select="xs:NCName('n')"/><xsl:sequence select="xs:ID('id')"/></xsl:variable><xsl:value-of select="concat($s instance of xs:NCName, ':', $v[2] instance of xs:ID, ':', $n)"/></xsl:template>"#;
    let style = source("1").replace(
        r#"<xsl:template match="/"><xsl:sequence select="1"/></xsl:template>"#,
        body,
    );
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .transform("<r a=' n '/>")
            .unwrap(),
        "true:true:n"
    );
}
