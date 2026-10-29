use rust_xslt::Stylesheet;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression))
        .unwrap()
        .transform("<r xmlns:p='urn:p' a=' x '/>")
        .unwrap()
}
#[test]
fn uri_values_preserve_type_collapse_xml_whitespace_and_cast_to_strings() {
    for (expr, expected) in [
        ("xs:anyURI(' a   b ') instance of xs:anyURI", "true"),
        ("xs:anyURI('a') instance of xs:anyAtomicType", "true"),
        ("xs:anyURI('a') instance of xs:string", "false"),
        ("xs:string(xs:anyURI(' a   b '))", "a b"),
        ("xs:anyURI('a&#x9; b&#xA;c')", "a b c"),
        ("' a  b ' cast as xs:anyURI", "a b"),
        ("empty(xs:anyURI(()))", "true"),
        ("xs:anyURI(r/@a)", "x"),
        ("xs:anyURI('urn:é/🙂')", "urn:é/🙂"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn uri_comparisons_and_string_function_promotion_use_codepoints() {
    for (expr, expected) in [
        ("xs:anyURI('a') eq 'a'", "true"),
        ("xs:anyURI('a') lt xs:anyURI('b')", "true"),
        ("deep-equal(xs:anyURI('a'), 'a')", "true"),
        ("upper-case(xs:anyURI('abc'))", "ABC"),
        ("compare(xs:anyURI('a'),'b')", "-1"),
        ("min((xs:anyURI('b'),'a')) instance of xs:string", "true"),
        ("max((xs:anyURI('a'),'b'))", "b"),
        ("number(xs:anyURI('1'))", "NaN"),
        ("boolean(xs:anyURI('a'))", "true"),
        ("boolean(xs:anyURI(''))", "false"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn uri_function_conversion_to_string_preserves_binding_types() {
    let body = r#"<xsl:template name="s" as="xs:string"><xsl:param name="v" as="xs:string"/><xsl:sequence select="$v"/></xsl:template><xsl:template match="/"><xsl:variable name="u" as="xs:anyURI"><xsl:sequence select="xs:anyURI(' a ')"/></xsl:variable><xsl:variable name="s" as="xs:string"><xsl:call-template name="s"><xsl:with-param name="v" select="$u"/></xsl:call-template></xsl:variable><xsl:value-of select="concat($u instance of xs:anyURI, ':', $s instance of xs:string, ':', $s)"/></xsl:template>"#;
    let style = source("1").replace(
        r#"<xsl:template match="/"><xsl:sequence select="1"/></xsl:template>"#,
        body,
    );
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "true:true:a"
    );
}
#[test]
fn namespace_functions_return_uri_atomic_values() {
    for (expr, expected) in [
        (
            "namespace-uri-from-QName(QName('urn:p','p:n')) instance of xs:anyURI",
            "true",
        ),
        (
            "namespace-uri-from-QName(QName('','n')) instance of xs:anyURI",
            "true",
        ),
        ("namespace-uri-from-QName(QName('urn:p','n'))", "urn:p"),
        ("empty(namespace-uri-from-QName(()))", "true"),
        ("namespace-uri(r) instance of xs:anyURI", "true"),
        ("namespace-uri(()) instance of xs:anyURI", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn uri_invalid_casts_and_boolean_use_report_type_errors() {
    for (expr, code) in [
        ("xs:anyURI(1)", "XPTY0004"),
        ("xs:integer(xs:anyURI('1'))", "XPTY0004"),
        ("xs:anyURI(QName('urn:p','n'))", "XPTY0004"),
        ("xs:anyURI('a') eq QName('urn:p','a')", "XPTY0004"),
        ("namespace-uri-from-QName('n')", "XPTY0004"),
        ("namespace-uri-from-QName(r)", "XPTY0117"),
        ("error(r)", "XPTY0117"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
    assert_eq!(evaluate("xs:anyURI('1') castable as xs:integer"), "false");
}

#[test]
fn node_names_and_element_namespace_lookup() {
    for (expr, expected) in [
        ("node-name(r) eq QName('', 'r')", "true"),
        ("empty(node-name(()))", "true"),
        ("empty(node-name(/))", "true"),
        ("namespace-uri-for-prefix('p', r)", "urn:p"),
        ("empty(namespace-uri-for-prefix('missing', r))", "true"),
        ("exists(in-scope-prefixes(r)[. = 'xml'])", "true"),
        ("resolve-QName('p:n', r) eq QName('urn:p', 'n')", "true"),
        ("empty(resolve-QName((), r))", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}

#[test]
fn namespace_lookup_rejects_invalid_names_and_argument_types() {
    for (expr, code) in [
        ("resolve-QName('missing:n', r)", "FONS0004"),
        ("resolve-QName('bad::name', r)", "FOCA0002"),
        ("resolve-QName('', r)", "FOCA0002"),
        ("resolve-QName(1, r)", "XPTY0004"),
        ("namespace-uri-for-prefix(1, r)", "XPTY0004"),
        ("in-scope-prefixes(/)", "XPTY0004"),
        ("node-name('r')", "XPTY0004"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
    assert_eq!(
        evaluate("namespace-uri-for-prefix('xml', r)"),
        "http://www.w3.org/XML/1998/namespace"
    );
}
#[test]
fn resolve_uri_follows_rfc_3986_reference_resolution() {
    let run = |expression: &str| {
        Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#))
            .unwrap()
            .transform("<r/>")
    };
    let base = "http://a/b/c/d;p?q";
    for (relative, expected) in [
        ("g:h", "g:h"),
        ("g", "http://a/b/c/g"),
        ("./g", "http://a/b/c/g"),
        ("g/", "http://a/b/c/g/"),
        ("/g", "http://a/g"),
        ("//g", "http://g"),
        ("?y", "http://a/b/c/d;p?y"),
        ("g?y#s", "http://a/b/c/g?y#s"),
        ("#s", "http://a/b/c/d;p?q#s"),
        ("", "http://a/b/c/d;p?q"),
        (".", "http://a/b/c/"),
        ("..", "http://a/b/"),
        ("../g", "http://a/b/g"),
        ("../../../g", "http://a/g"),
        ("g/../h", "http://a/b/c/h"),
    ] {
        assert_eq!(
            run(&format!("resolve-uri('{relative}', '{base}')")).unwrap(),
            expected,
            "{relative}"
        );
    }
    assert_eq!(run("resolve-uri('http://x/y')").unwrap(), "http://x/y");
    assert_eq!(run("empty(resolve-uri((), 'http://x/'))").unwrap(), "true");
    assert_eq!(
        run("resolve-uri('a', 'http://x') instance of xs:anyURI").unwrap(),
        "true"
    );
    for (expression, code) in [
        ("resolve-uri('a')", "FONS0005"),
        ("resolve-uri('a', 'b/c')", "FORG0002"),
        ("resolve-uri('a b', 'http://x/')", "FORG0002"),
        ("resolve-uri('%zz', 'http://x/')", "FORG0002"),
    ] {
        assert_eq!(run(expression).unwrap_err().code, code, "{expression}");
    }
}
