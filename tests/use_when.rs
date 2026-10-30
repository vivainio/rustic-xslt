use rust_xslt::Stylesheet;
fn run(declarations: &str, body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:math="http://www.w3.org/2005/xpath-functions/math" exclude-result-prefixes="math"><xsl:output omit-xml-declaration="yes"/>{declarations}<xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
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

#[test]
fn element_available_function_reference_captures_namespaces() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><xsl:variable name="f" select="element-available#1"/><xsl:variable name="p" select="element-available(?)"/><out><xsl:value-of xmlns:a="urn:other" select="$f('a:value-of'), $p('a:value-of'), local-name-from-QName(function-name($f))"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out>true true element-available</out>"
    );
}

#[test]
fn element_available_dynamic_lookup_captures_namespaces() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><xsl:variable name="f" select="function-lookup(QName('http://www.w3.org/2005/xpath-functions','element-available'), 1)"/><xsl:variable name="lookup" select="function-lookup#2"/><xsl:variable name="g" select="$lookup(QName('http://www.w3.org/2005/xpath-functions','element-available'), 1)"/><out><xsl:value-of xmlns:a="urn:other" select="$f('a:value-of'), $g('a:value-of'), function-lookup(QName('http://www.w3.org/2005/xpath-functions','element-available'), 1)('xsl:value-of')"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out>true true true</out>"
    );
}

#[test]
fn system_property_resolves_names_and_captures_context() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><xsl:variable name="f" select="system-property#1"/><xsl:variable name="g" select="function-lookup(QName('http://www.w3.org/2005/xpath-functions','system-property'), 1)"/><out><xsl:value-of select="system-property('a:version')"/><xsl:text>|</xsl:text><xsl:value-of xmlns:a="urn:other" select="$f('a:version'), $g('a:version'), system-property('a:version')"/><xsl:text>|</xsl:text><xsl:value-of xmlns="http://www.w3.org/1999/XSL/Transform" select="system-property('version')"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out>3.0|3.0 3.0 |</out>"
    );
    let error = run(
        "",
        r#"<xsl:value-of select="system-property('missing:version')"/>"#,
    )
    .unwrap_err();
    assert_eq!(error.code, "XTDE1390");
}

#[test]
fn type_available_resolves_static_namespaces() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="http://www.w3.org/2001/XMLSchema" exclude-result-prefixes="a"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><xsl:variable name="f" select="type-available#1"/><xsl:variable name="g" select="function-lookup(QName('http://www.w3.org/2005/xpath-functions','type-available'),1)"/><out><xsl:value-of select="type-available('a:string')"/><xsl:text> </xsl:text><xsl:value-of xmlns:a="urn:other" select="$f('a:string'), $g('a:string'), type-available('a:string')"/><xsl:text> </xsl:text><xsl:value-of xmlns="http://www.w3.org/2001/XMLSchema" select="type-available('string')"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out>true true true false true</out>"
    );
    let error = run(
        "",
        r#"<xsl:value-of select="type-available('missing:string')"/>"#,
    )
    .unwrap_err();
    assert_eq!(error.code, "XTDE1428");
}

#[test]
fn function_available_resolves_namespaces_and_user_functions() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:fun" xmlns:a="http://www.w3.org/2005/xpath-functions" exclude-result-prefixes="f a"><xsl:output omit-xml-declaration="yes"/><xsl:function name="f:test"><xsl:param name="v"/><xsl:sequence select="$v"/></xsl:function><xsl:template match="/"><xsl:variable name="ref" select="function-available#2"/><xsl:variable name="lookup" select="function-lookup(QName('http://www.w3.org/2005/xpath-functions','function-available'),2)"/><out><xsl:value-of select="function-available('a:concat',2), function-available('f:test',1), function-available('f:test',2), function-available('true',-1)"/><xsl:text> </xsl:text><xsl:value-of xmlns:a="urn:other" select="$ref('a:concat',2), $lookup('a:concat',2), function-available('a:concat',2)"/></out></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out>true true false false true true false</out>"
    );
    let error = run(
        "",
        r#"<xsl:value-of select="function-available('missing:foo')"/>"#,
    )
    .unwrap_err();
    assert_eq!(error.code, "XTDE1400");
}

#[test]
fn function_available_requires_integer_arity() {
    for expr in [
        "function-available('concat', -1)",
        "function-available('true', 999999999999999999999999999999)",
    ] {
        assert_eq!(
            run("", &format!(r#"<xsl:value-of select="{expr}"/>"#)).unwrap(),
            "false"
        );
    }
    for expr in [
        "function-available('true', ())",
        "function-available('true', '0')",
        "function-available('true', 0.0)",
    ] {
        assert_eq!(
            run("", &format!(r#"<xsl:value-of select="{expr}"/>"#))
                .unwrap_err()
                .code,
            "XPTY0004"
        );
    }
    assert_eq!(
        run(
            "",
            r#"<xsl:value-of select="function-available('true', /r/@arity)"/>"#
        )
        .unwrap_err()
        .code,
        "XPTY0004"
    );
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><xsl:value-of select="function-available('true', /r/@arity)"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r arity='0'/>")
            .unwrap(),
        "true"
    );
}

#[test]
fn availability_names_trim_whitespace_and_reject_invalid_qnames() {
    for (name, code) in [
        ("element-available", "XTDE1440"),
        ("system-property", "XTDE1390"),
        ("type-available", "XTDE1428"),
        ("function-available", "XTDE1400"),
    ] {
        for text in ["bad::name", "missing:name"] {
            let body = format!(r#"<xsl:value-of select="{name}('{text}')"/>"#);
            assert_eq!(run("", &body).unwrap_err().code, code, "{name} {text:?}");
        }
    }
}

#[test]
fn availability_names_accept_surrounding_xml_whitespace() {
    for (expr, expected) in [
        ("element-available(' xsl:text ')", "true"),
        ("system-property(' xsl:version ')", "3.0"),
        ("type-available(' xs:string ')", "true"),
        ("function-available(' concat ', 2)", "true"),
    ] {
        assert_eq!(
            run("", &format!(r#"<xsl:value-of select="{expr}"/>"#)).unwrap(),
            expected
        );
    }
}

#[test]
fn extension_attributes_on_xslt_elements_are_ignored() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:e="urn:extension" xmlns:f="urn:function" e:hint="yes"><xsl:output method="text" e:hint="yes"/><xsl:function name="f:test" e:hint="yes"><xsl:sequence select="'ok'" e:hint="yes"/></xsl:function><xsl:template match="/" e:hint="yes"><xsl:value-of select="f:test()" e:hint="yes"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "ok"
    );
}
