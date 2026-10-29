use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn sheet(expr: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#)).unwrap()
}
#[test]
fn node_extrema_remove_duplicates_and_restore_document_order() {
    let xml = "<r a='v'><a><b/></a><c/></r>";
    for (expr, expected) in [
        ("innermost((r/c, r, r/a/b, r/a, r/c)) ! name(.)", "b c"),
        ("outermost((r/c, r, r/a/b, r/a, r/c)) ! name(.)", "r"),
        ("innermost((r, r/@a)) ! name(.)", "a"),
        ("outermost((r, r/@a)) ! name(.)", "r"),
        ("count(innermost((r/a, r/a)))", "1"),
        ("innermost((/, r)) is r", "true"),
        ("outermost((r, /)) is /", "true"),
        ("empty(innermost(()))", "true"),
        ("empty(outermost(()))", "true"),
        ("unordered((r/c, r/a))[1] is r/c", "true"),
    ] {
        assert_eq!(sheet(expr).transform(xml).unwrap(), expected, "{expr}");
    }
}
#[test]
fn has_children_checks_node_content_without_atomization() {
    let xml = "<r a='v'><a/><b>text</b><!--c--><?pi v?></r>";
    for (expr, expected) in [
        ("has-children(/)", "true"),
        ("has-children(r)", "true"),
        ("has-children(r/a)", "false"),
        ("has-children(r/@a)", "false"),
        ("has-children(r/comment())", "false"),
        ("has-children(r/processing-instruction())", "false"),
        ("has-children(r/b/text())", "false"),
        ("has-children(())", "false"),
        ("r/b/has-children()", "true"),
    ] {
        assert_eq!(sheet(expr).transform(xml).unwrap(), expected, "{expr}");
    }
    for expr in [
        "has-children('x')",
        "has-children((/,r))",
        "innermost((r,1))",
        "outermost(1)",
    ] {
        assert_eq!(
            sheet(expr).transform(xml).unwrap_err().code,
            "XPTY0004",
            "{expr}"
        );
    }
}
#[test]
fn node_extrema_respect_detached_constructed_roots() {
    let s = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output method="text"/><xsl:template match="/"><xsl:variable name="nodes" as="element()*"><a><b/></a><c/></xsl:variable><xsl:sequence select="count(outermost(($nodes, $nodes/b)))"/><xsl:sequence select="innermost(($nodes, $nodes/b)) ! name(.)"/></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(s).unwrap().transform("<r/>").unwrap(),
        "2 b c"
    );
}
#[test]
fn node_selection_consumes_shared_work_budget() {
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 60,
            ..Default::default()
        },
        ..Default::default()
    };
    let xml = format!("<r>{}</r>", "<n/>".repeat(100));
    assert_eq!(
        sheet("innermost(r/n)")
            .transform_with_options(&xml, &opts)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}

#[test]
fn lang_follows_nearest_xml_language_and_attribute_owners() {
    let xml = "<r xml:lang='en-US'><a id='x'/><b xml:lang='fr'><c/></b><d xml:lang=''/></r>";
    for (expr, expected) in [
        ("lang('EN', r/a)", "true"),
        ("lang('en-US', r/a/@id)", "true"),
        ("lang('en', r/b/c)", "false"),
        ("lang('fr', r/b/c)", "true"),
        ("lang('en', r/d)", "false"),
        ("lang((), r/d)", "true"),
        ("r/a/lang('en')", "true"),
        ("lang('en', /)", "false"),
    ] {
        assert_eq!(sheet(expr).transform(xml).unwrap(), expected, "{expr}");
    }
    for expr in ["lang(1, r)", "lang('en', ())", "lang('en', 'x')"] {
        assert_eq!(
            sheet(expr).transform(xml).unwrap_err().code,
            "XPTY0004",
            "{expr}"
        );
    }
}
#[test]
fn generated_ids_distinguish_node_identity_and_expanded_attribute_names() {
    let xml = "<r xmlns:p='u' a='v' p:a='v'><a/><a/></r>";
    for expr in [
        "generate-id(r) eq generate-id(r)",
        "generate-id(r/a[1]) ne generate-id(r/a[2])",
        "generate-id(r/@a) ne generate-id(r/@p:a)",
        "generate-id(r/@a) ne generate-id(r)",
        "generate-id(()) eq ''",
        "r/generate-id() eq generate-id(r)",
    ] {
        // The QName is declared where the attribute test is compiled.
        let style = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="u"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#
        );
        assert_eq!(
            Stylesheet::compile(&style).unwrap().transform(xml).unwrap(),
            "true",
            "{expr}"
        );
    }
    let id = sheet("generate-id(r/@a)").transform(xml).unwrap();
    assert!(id.starts_with(|c: char| c.is_ascii_alphabetic()));
    assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
}
#[test]
fn processing_instruction_test_accepts_unquoted_ncname_targets() {
    let run = |expression: &str| {
        Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#))
            .unwrap()
            .transform("<r><?a x?><?b y?></r>")
            .unwrap()
    };
    assert_eq!(run("r/processing-instruction(a)/name()"), "a");
    assert_eq!(run("r/processing-instruction('b')/name()"), "b");
    assert_eq!(run("count(r/processing-instruction())"), "2");
    assert_eq!(
        run("(r/processing-instruction() except r/processing-instruction(a))/name()"),
        "b"
    );
}
#[test]
fn path_describes_nodes_with_positions_and_expanded_names() {
    let run = |expression: &str| {
        Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:q="urn:q"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#))
            .unwrap()
            .transform("<r xmlns:q='urn:q'><a/><q:a k='1'/><a>t<!--c--><?pi x?>u</a></r>")
            .unwrap()
    };
    for (expression, expected) in [
        ("path(/)", "/"),
        ("path(/r)", "/Q{}r[1]"),
        ("path(/r/a[2])", "/Q{}r[1]/Q{}a[2]"),
        ("path(/r/q:a)", "/Q{}r[1]/Q{urn:q}a[1]"),
        ("path(/r/*[2]/@k)", "/Q{}r[1]/Q{urn:q}a[1]/@k"),
        ("path(/r/a[2]/text()[2])", "/Q{}r[1]/Q{}a[2]/text()[2]"),
        ("path(/r/a[2]/comment())", "/Q{}r[1]/Q{}a[2]/comment()[1]"),
        (
            "path(/r/a[2]/processing-instruction())",
            "/Q{}r[1]/Q{}a[2]/processing-instruction(pi)[1]",
        ),
        ("empty(path(()))", "true"),
    ] {
        assert_eq!(run(expression), expected, "{expression}");
    }
}
