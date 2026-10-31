use rust_xslt::Stylesheet;
fn run(body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:variable name="g" select="100"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform("<r><n>1</n><n>2</n></r>")
}
#[test]
fn evaluate_runs_dynamic_xpath_with_context_and_parameters() {
    assert_eq!(run(r#"<xsl:evaluate xpath="'1 + 2'"/>"#).unwrap(), "3");
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'sum(n)'" context-item="r"/>"#).unwrap(),
        "3"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'$a * $b'"><xsl:with-param name="a" select="6"/><xsl:with-param name="b" select="7"/></xsl:evaluate>"#).unwrap(),
        "42"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'$g + 1'"/>"#).unwrap_err().code,
        "XTDE3160"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="&quot;concat('p', 'q')&quot;"/>"#).unwrap(),
        "pq"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'('"/>"#).unwrap_err().code,
        "XTDE3160"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'.'"/>"#).unwrap_err().code,
        "XPDY0002"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'1'" context-item="()"/>"#).unwrap(),
        "1"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'.'" context-item="()"/>"#)
            .unwrap_err()
            .code,
        "XPDY0002"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'1'" context-item="(1, 2)"/>"#)
            .unwrap_err()
            .code,
        "XTTE3210"
    );
    assert!(run(r#"<xsl:evaluate xpath="()"/>"#).is_err());
    assert_eq!(
        run(r#"<xsl:evaluate xpath="42"/>"#).unwrap_err().code,
        "XPTY0004"
    );
}
#[test]
fn where_populated_drops_empty_results() {
    assert_eq!(
        run("<out><xsl:where-populated><b/></xsl:where-populated></out>").unwrap(),
        "<out><b></b></out>"
    );
    assert_eq!(
        run("<out><xsl:where-populated><xsl:value-of select=\"()\"/></xsl:where-populated></out>")
            .unwrap(),
        "<out></out>"
    );
    assert_eq!(run("<out><xsl:where-populated><xsl:if test=\"false()\"><b/></xsl:if></xsl:where-populated></out>").unwrap(), "<out></out>");
    assert_eq!(
        run("<out><xsl:where-populated>text</xsl:where-populated></out>").unwrap(),
        "<out>text</out>"
    );
}
#[test]
fn namespace_instruction_declares_namespaces_on_the_current_element() {
    assert_eq!(
        run(r#"<out><xsl:namespace name="p" select="'urn:p'"/><xsl:namespace name="q">urn:q</xsl:namespace></out>"#).unwrap(),
        r#"<out xmlns:p="urn:p" xmlns:q="urn:q"></out>"#
    );
    assert_eq!(
        run(r#"<xsl:namespace name="p" select="'urn:p'"/>"#)
            .unwrap_err()
            .code,
        "XTDE0410"
    );
    assert_eq!(
        run(r#"<out><xsl:namespace name="1bad" select="'u'"/></out>"#)
            .unwrap_err()
            .code,
        "XTDE0920"
    );
}

#[test]
fn evaluate_target_uses_xpath_function_library() {
    for expr in [
        "system-property('xsl:version')",
        "element-available('xsl:text')",
        "current()",
        "system-property#1",
    ] {
        let body = format!(r#"<xsl:evaluate xpath="&quot;{expr}&quot;"/>"#);
        assert_eq!(run(&body).unwrap_err().code, "XTDE3160", "{expr}");
    }
    assert_eq!(
        run(r#"<xsl:evaluate xpath="&quot;concat('a','b')&quot;"/>"#).unwrap(),
        "ab"
    );
    assert_eq!(
        run(r#"<xsl:value-of select="system-property('xsl:version')"/>"#).unwrap(),
        "3.0"
    );
    let stylesheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:function name="f:test" visibility="public"><xsl:sequence select="42"/></xsl:function><xsl:template match="/"><xsl:evaluate xpath="'f:test()'"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(stylesheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "42"
    );
}

#[test]
fn evaluate_checks_target_variable_scope_before_execution() {
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'if (false()) then $g else 1'"/>"#)
            .unwrap_err()
            .code,
        "XTDE3160"
    );
    assert_eq!(run(r#"<xsl:evaluate xpath="'let $v := 4 return $v + $p'"><xsl:with-param name="p" select="3"/></xsl:evaluate>"#).unwrap(), "7");
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:variable name="global" select="41"/><xsl:function name="f:test" visibility="public"><xsl:sequence select="$global + 1"/></xsl:function><xsl:template match="/"><xsl:evaluate xpath="'f:test()'"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "42"
    );
}

#[test]
fn evaluate_with_params_maps_bind_qnames_and_override_children() {
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p + $q'" with-params="map{QName('', 'p'): 5, QName('', 'q'): 7}"><xsl:with-param name="p" select="1"/></xsl:evaluate>"#).unwrap(), "12");
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'$p'" with-params="map{QName('', 'p'): (1, 2)}"/>"#).unwrap(),
        "1 2"
    );
    for value in ["()", "42", "map{'p': 1}"] {
        let body = format!(r#"<xsl:evaluate xpath="'1'" with-params="{value}"/>"#);
        assert_eq!(run(&body).unwrap_err().code, "XTTE3165", "{value}");
    }
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'$p'" with-params="map{}"/>"#)
            .unwrap_err()
            .code,
        "XTDE3160"
    );
}

#[test]
fn evaluate_namespace_context_supplies_dynamic_namespaces() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:wrong" xpath-default-namespace="urn:wrong"><xsl:output method="text"/><xsl:template match="/"><xsl:evaluate xpath="'concat(string(p:a), string(b))'" context-item="/*" namespace-context="/*"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform(r#"<r xmlns:p="urn:right" xmlns="urn:default"><p:a>one</p:a><b>two</b></r>"#)
            .unwrap(),
        "onetwo"
    );
    for value in ["()", "1", "(/*, /*)"] {
        let body = format!(r#"<xsl:evaluate xpath="'1'" namespace-context="{value}"/>"#);
        assert_eq!(run(&body).unwrap_err().code, "XTTE3170", "{value}");
    }
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'1'" namespace-context="$missing"/>"#)
            .unwrap_err()
            .code,
        "XPST0008"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'p:a'" namespace-context="/"/>"#)
            .unwrap_err()
            .code,
        "XTDE3160"
    );
}

#[test]
fn evaluate_converts_typed_parameters_and_results() {
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p instance of xs:double'"><xsl:with-param name="p" as="xs:double" select="1"/></xsl:evaluate>"#).unwrap(), "true");
    assert_eq!(run(r#"<xsl:variable name="v" as="item()*"><xsl:evaluate xpath="'1'" as="xs:double"/></xsl:variable><xsl:value-of select="$v instance of xs:double"/>"#).unwrap(), "true");
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p'"><xsl:with-param name="p" as="xs:integer" select="'bad'"/></xsl:evaluate>"#).unwrap_err().code, "XTTE0590");
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'(1, 2)'" as="xs:integer"/>"#)
            .unwrap_err()
            .code,
        "XPTY0004"
    );
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p instance of xs:string'" with-params="map{QName('', 'p'): 'override'}"><xsl:with-param name="p" as="xs:integer" select="1"/></xsl:evaluate>"#).unwrap(), "true");
}

#[test]
fn evaluate_target_disables_xpath_one_compatibility() {
    let sheet = r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:evaluate xpath="&quot;1 = '1'&quot;"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap_err()
            .code,
        "XPTY0004"
    );
}

#[test]
fn evaluate_checks_function_visibility_in_unexecuted_branches() {
    for visibility in ["", "visibility='private'", "visibility='hidden'"] {
        for target in ["f:test()", "if (false()) then f:test() else 1", "f:test#0"] {
            let sheet = format!(
                r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:function name="f:test" {visibility}><xsl:sequence select="42"/></xsl:function><xsl:template match="/"><xsl:evaluate xpath="'{target}'"/></xsl:template></xsl:stylesheet>"#
            );
            assert_eq!(
                Stylesheet::compile(&sheet)
                    .unwrap()
                    .transform("<r/>")
                    .unwrap_err()
                    .code,
                "XTDE3160",
                "{visibility} {target}"
            );
        }
    }
}

#[test]
fn evaluate_function_lookup_uses_the_target_function_library() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:function name="f:private"><xsl:sequence select="1"/></xsl:function><xsl:function name="f:public" visibility="public"><xsl:sequence select="2"/></xsl:function><xsl:template match="/"><xsl:evaluate xpath="&quot;(exists(function-lookup(QName('urn:f', 'f:private'), 0)), exists(function-lookup(QName('urn:f', 'f:public'), 0)), exists(function-lookup(QName('http://www.w3.org/2005/xpath-functions', 'current'), 0)), (function-lookup#2)(QName('urn:f', 'f:public'), 0)())&quot;"/><xsl:text> </xsl:text><xsl:value-of select="exists(function-lookup(QName('urn:f', 'f:private'), 0))"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "false true false 2 true"
    );
    let deferred = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:output method="text"/><xsl:function name="f:private"><xsl:sequence select="1"/></xsl:function><xsl:template match="/"><xsl:variable name="lookup" as="function(*)"><xsl:evaluate xpath="'function-lookup#2'"/></xsl:variable><xsl:value-of select="exists($lookup(QName('urn:f', 'f:private'), 0))"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(deferred)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "false"
    );
}

#[test]
fn evaluate_base_uri_is_available_to_dynamic_xpath_and_function_items() {
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'static-base-uri()'" base-uri="https://example.test/a/"/>"#)
            .unwrap(),
        "https://example.test/a/"
    );
    assert_eq!(run(r#"<xsl:evaluate xpath="&quot;resolve-uri('item')&quot;" base-uri="https://example.test/{'a'}/"/>"#).unwrap(), "https://example.test/a/item");
    assert_eq!(run(r#"<xsl:variable name="f" as="function(*)"><xsl:evaluate xpath="'static-base-uri#0'" base-uri="https://example.test/a/"/></xsl:variable><xsl:value-of select="$f()"/>"#).unwrap(), "https://example.test/a/");
    assert_eq!(run(r#"<xsl:variable name="f" as="function(*)"><xsl:evaluate xpath="'resolve-uri#1'" base-uri="https://example.test/a/"/></xsl:variable><xsl:value-of select="$f('b')"/>"#).unwrap(), "https://example.test/a/b");
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'1'" base-uri="{$missing}"/>"#)
            .unwrap_err()
            .code,
        "XPST0008"
    );
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:evaluate xpath="&quot;unparsed-text('data.txt')&quot;" base-uri="https://example.test/a/"/></xsl:template></xsl:stylesheet>"#;
    let options = rust_xslt::TransformOptions {
        resolver: Some(rust_xslt::ResourceResolver::new(|uri| {
            assert_eq!(uri, "https://example.test/a/data.txt");
            Ok("loaded".into())
        })),
        ..Default::default()
    };
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform_with_options("<r/>", &options)
            .unwrap(),
        "loaded"
    );
}

#[test]
fn evaluate_inherits_stylesheet_xml_base() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xml:base="https://example.test/root/"><xsl:output method="text"/><xsl:template match="/" xml:base="nested/"><xsl:evaluate xpath="&quot;resolve-uri('data')&quot;"/><xsl:text> </xsl:text><xsl:evaluate xpath="'static-base-uri()'" xml:base="../other/"/><xsl:text> </xsl:text><xsl:evaluate xpath="'static-base-uri()'" base-uri="https://override.test/"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "https://example.test/root/nested/data https://example.test/root/other/ https://override.test/"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'empty(static-base-uri())'"/>"#).unwrap(),
        "true"
    );
}

#[test]
fn evaluate_base_uri_avt_reads_source_node_base() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:evaluate xpath="&quot;resolve-uri('file')&quot;" base-uri="{base-uri(/*/*)}"/></xsl:template></xsl:stylesheet>"#;
    let source = r#"<r xml:base="https://example.test/a/"><p xml:base="b/"/></r>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform(source)
            .unwrap(),
        "https://example.test/a/b/file"
    );
}

#[test]
fn stylesheet_function_argument_errors_use_xpath_type_error() {
    for body in [
        r#"<xsl:sequence select="f:typed('bad')"/>"#,
        r#"<xsl:sequence select="(f:typed#1)('bad')"/>"#,
        r#"<xsl:evaluate xpath="&quot;f:typed('bad')&quot;"/>"#,
    ] {
        let sheet = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f"><xsl:function name="f:typed" visibility="public"><xsl:param name="p" as="xs:integer"/><xsl:sequence select="$p"/></xsl:function><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#
        );
        assert_eq!(
            Stylesheet::compile(&sheet)
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            "XPTY0004",
            "{body}"
        );
    }
}

#[test]
fn evaluate_rejects_duplicate_and_tunnel_parameters() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:p" xmlns:b="urn:p"><xsl:template match="/"><xsl:evaluate xpath="'1'"><xsl:with-param name="a:p" select="1"/><xsl:with-param name="b:p" select="2"/></xsl:evaluate></xsl:template></xsl:stylesheet>"#;
    assert_eq!(Stylesheet::compile(sheet).unwrap_err().code, "XTSE0670");
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p'"><xsl:with-param name="p" select="1" tunnel="yes"/></xsl:evaluate>"#).unwrap_err().code, "XTSE0020");
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p'"><xsl:with-param name="p" select="1" tunnel="no"/></xsl:evaluate>"#).unwrap(), "1");
}

#[test]
fn evaluate_schema_aware_attribute_is_checked_as_an_avt() {
    for value in [
        "yes",
        "no",
        "true",
        "false",
        "1",
        "0",
        "{'yes'}",
        " { 'no' } ",
    ] {
        let body = format!(r#"<xsl:evaluate xpath="'xs:integer(3)'" schema-aware="{value}"/>"#);
        assert_eq!(run(&body).unwrap(), "3", "{value}");
    }
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'1'" schema-aware="invalid"/>"#)
            .unwrap_err()
            .code,
        "XTSE0020"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'1'" schema-aware="{'invalid'}"/>"#)
            .unwrap_err()
            .code,
        "XTDE0030"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'1'" schema-aware="{$missing}"/>"#)
            .unwrap_err()
            .code,
        "XPST0008"
    );
}

#[test]
fn evaluate_accepts_constructed_parameters() {
    assert_eq!(run(r#"<xsl:evaluate xpath="'name($p/*)' "><xsl:with-param name="p"><element>text</element></xsl:with-param></xsl:evaluate>"#).unwrap(), "element");
    assert_eq!(run(r#"<xsl:evaluate xpath="'sum($p)' "><xsl:with-param name="p" as="xs:integer*"><xsl:sequence select="1, 2"/><xsl:sequence select="3"/></xsl:with-param></xsl:evaluate>"#).unwrap(), "6");
    assert_eq!(run(r#"<xsl:variable name="p" select="10"/><xsl:evaluate xpath="'$a + $b'"><xsl:with-param name="a" as="xs:integer"><xsl:sequence select="$p"/></xsl:with-param><xsl:with-param name="b" as="xs:integer"><xsl:sequence select="$p + 1"/></xsl:with-param></xsl:evaluate>"#).unwrap(), "21");
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p'" with-params="map{QName('', 'p'): 9}"><xsl:with-param name="p"><element/></xsl:with-param></xsl:evaluate>"#).unwrap(), "9");
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p'"><xsl:with-param name="p" as="xs:integer"><xsl:sequence select="'bad'"/></xsl:with-param></xsl:evaluate>"#).unwrap_err().code, "XTTE0590");
    assert_eq!(run(r#"<xsl:evaluate xpath="'$p'"><xsl:with-param name="p" as="element(expected)"><other/></xsl:with-param></xsl:evaluate>"#).unwrap_err().code, "XTTE0590");
}

#[test]
fn evaluate_ignores_fallback_and_excluded_parameters() {
    let body = r#"<xsl:evaluate xpath="'$p'"><xsl:with-param name="p" select="error()" use-when="false()"/><xsl:fallback><xsl:sequence select="error()"/></xsl:fallback><xsl:with-param name="p" select="7"/></xsl:evaluate>"#;
    assert_eq!(run(body).unwrap(), "7");
    let excluded = r#"<xsl:evaluate xpath="'1'"><xsl:with-param name="p" select="$missing" use-when="false()"/></xsl:evaluate>"#;
    assert_eq!(run(excluded).unwrap(), "1");
}

#[test]
fn evaluate_inherits_stylesheet_module_uri() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xml:base="sub/"><xsl:output method="text"/><xsl:include href="modules/part.xsl"/><xsl:template match="/"><xsl:evaluate xpath="'static-base-uri()'"/><xsl:text> </xsl:text><xsl:call-template name="part"/></xsl:template></xsl:stylesheet>"#;
    let included = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xml:base="nested/"><xsl:template name="part"><xsl:evaluate xpath="&quot;resolve-uri('data.xml')&quot;"/></xsl:template></xsl:stylesheet>"#;
    let compiled = Stylesheet::compile_with_base_uri_and_resolver(
        sheet,
        "https://example.test/root/main.xsl",
        &|uri| {
            assert_eq!(uri, "https://example.test/root/sub/modules/part.xsl");
            Ok(included.into())
        },
    )
    .unwrap();
    assert_eq!(
        compiled.transform("<r/>").unwrap(),
        "https://example.test/root/sub/ https://example.test/root/sub/modules/nested/data.xml"
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'empty(static-base-uri())'"/>"#).unwrap(),
        "true"
    );
    assert_eq!(
        Stylesheet::compile_with_base_uri(sheet, "relative.xsl")
            .unwrap_err()
            .code,
        "RUST_XSLT_INVALID_URI"
    );
    assert!(
        Stylesheet::compile_with_base_uri("<broken", "https://example.test/error.xsl").is_err()
    );
    assert_eq!(
        run(r#"<xsl:evaluate xpath="'empty(static-base-uri())'"/>"#).unwrap(),
        "true"
    );
}
