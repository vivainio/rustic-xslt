use rust_xslt::{StreamOptions, Stylesheet, transform};
use std::io::Cursor;
fn sheet(expression: &str) -> String {
    format!(
        r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" version="3.0"><xsl:output method="text"/><xsl:template match="/"><xsl:value-of select="{expression}"/></xsl:template></xsl:stylesheet>"##
    )
}
const SOURCE: &str =
    "<r><before/><group id='g'><a/><b id='B'><inside/></b><c/></group><after/></r>";

#[test]
fn descendant_name_predicates_keep_each_parent_and_current_context() {
    let style = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output method="text"/><xsl:template match="/"><xsl:for-each select="/r/g"><xsl:value-of select="string-join(//n[@k=current()/@k][1]/@id, ',')"/><xsl:text>|</xsl:text></xsl:for-each></xsl:template></xsl:stylesheet>"#;
    let source = "<r><g k='a'><n k='a' id='a1'/><n k='a' id='a2'/><n k='b' id='b1'/></g><g k='b'><n k='a' id='a3'/><n k='b' id='b2'/><n k='b' id='b3'/></g></r>";
    let compiled = Stylesheet::compile(style).unwrap();
    assert_eq!(compiled.transform(source).unwrap(), "a1,a3|b1,b2|");
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_temporary_bytes: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        compiled.transform_with_options(source, &options).unwrap(),
        "a1,a3|b1,b2|"
    );
}

#[test]
fn repeated_root_descendant_queries_fit_the_work_budget() {
    let style = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output method="text"/><xsl:template match="/"><xsl:for-each select="/r/g"><xsl:value-of select="count(//n[@ref=current()/@id][1])"/></xsl:for-each></xsl:template></xsl:stylesheet>"#;
    let source = format!(
        "<r>{}</r>",
        (0..200)
            .map(|i| format!("<g id='{i}'><n ref='{i}'/></g>"))
            .collect::<String>()
    );
    let sheet = Stylesheet::compile(style).unwrap();
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 650_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&source, &options).unwrap(),
        "1".repeat(200)
    );
}

#[test]
fn descendant_name_cache_keeps_documents_and_namespaces_separate() {
    let style = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" xmlns:q="urn:q" version="3.0"><xsl:output method="text"/><xsl:template match="/"><xsl:for-each select="(doc('a'), doc('b'), doc('a'))"><xsl:value-of select="//p:n/@id"/><xsl:value-of select="//q:n/@id"/><xsl:text>|</xsl:text></xsl:for-each></xsl:template></xsl:stylesheet>"#;
    let options = rust_xslt::TransformOptions {
        resolver: Some(rust_xslt::ResourceResolver::new(|uri| match uri {
            "a" => Ok("<r xmlns:p='urn:p' xmlns:q='urn:q'><p:n id='A'/><q:n id='a'/></r>".into()),
            "b" => Ok("<r xmlns:p='urn:p' xmlns:q='urn:q'><p:n id='B'/><q:n id='b'/></r>".into()),
            _ => panic!("unexpected URI: {uri}"),
        })),
        ..Default::default()
    };
    assert_eq!(
        Stylesheet::compile(style)
            .unwrap()
            .transform_with_options("<r/>", &options)
            .unwrap(),
        "Aa|Bb|Aa|"
    );
}
#[test]
fn reverse_axes_apply_predicates_before_document_order_normalization() {
    for (expression, want) in [
        ("r/group/b/ancestor::*[1]/@id", "g"),
        ("r/group/b/ancestor-or-self::*[1]/@id", "B"),
        ("name(r/group/b/preceding-sibling::*[1])", "a"),
        ("name(r/group/b/following-sibling::*[1])", "c"),
        ("name(r/group/b/preceding::*[1])", "a"),
        ("name(r/group/b/following::*[1])", "c"),
        ("name(r/group/b/@id/following::*[1])", "inside"),
        ("r/group/b/@id/ancestor::*[1]/@id", "B"),
        ("name((r/group/b/ancestor::*)[1])", "r"),
    ] {
        assert_eq!(
            transform(&sheet(expression), SOURCE).unwrap(),
            want,
            "{expression}"
        );
    }
}
#[test]
fn following_and_preceding_exclude_descendants_and_ancestors() {
    for (expression, want) in [
        ("count(r/group/b/following::*)", "2"),
        ("count(r/group/b/preceding::*)", "2"),
        ("count(r/group/b/@id/preceding::*)", "2"),
        ("count(r/group/b/ancestor::*)", "2"),
        ("count(r/group/b/ancestor::node())", "3"),
    ] {
        assert_eq!(
            transform(&sheet(expression), SOURCE).unwrap(),
            want,
            "{expression}"
        );
    }
}
#[test]
fn namespace_wildcards_and_named_kind_tests() {
    let source = "<r xmlns:p='urn:p' xmlns:q='urn:q'><p:n p:id='a'/><q:n q:id='b'/><n id='c'/><!--comment--><?chosen yes?><?other no?></r>";
    for (expression, want) in [
        ("count(r/p:*)", "1"),
        ("count(r/*:n)", "3"),
        ("r/p:n/@p:*", "a"),
        ("r/*/@*:id", "a b c"),
        ("count(r/element())", "3"),
        ("count(r/element(p:n))", "1"),
        ("count(r/p:n/attribute::attribute(p:id))", "1"),
        ("r/processing-instruction('chosen')", "yes"),
        ("count(self::document-node())", "1"),
    ] {
        assert_eq!(
            transform(&sheet(expression), source).unwrap(),
            want,
            "{expression}"
        );
    }
}
#[test]
fn conditional_expressions_are_lazy_and_nest_correctly() {
    for (expression, want) in [
        ("if (true()) then 'yes' else count('bad'/n)", "yes"),
        (
            "if (false()) then 'no' else if (1 = 1) then 2 + 3 else 4",
            "5",
        ),
        ("concat('x', if (true()) then 'a' else 'b')", "xa"),
    ] {
        assert_eq!(
            transform(&sheet(expression), "<r/>").unwrap(),
            want,
            "{expression}"
        );
    }
    assert!(Stylesheet::compile(&sheet("if (true()) 'missing then' else 'bad'")).is_err());
    assert_eq!(
        Stylesheet::compile(&sheet("if (true()) then 'ok' else $missing"))
            .unwrap_err()
            .code,
        "XPST0008"
    );
}
#[test]
fn stream_safety_checks_all_new_axes_and_conditional_branches() {
    for expression in [
        "ancestor::*",
        "following::*",
        "preceding::*",
        "following-sibling::*",
        "preceding-sibling::*",
        "if (true()) then 'safe' else ancestor::*",
    ] {
        let s = sheet(expression).replace("match=\"/\"", "name=\"record\"");
        let s = Stylesheet::compile(&s).unwrap();
        assert_eq!(
            s.stream_records(
                Cursor::new("<records/>"),
                &mut vec![],
                &StreamOptions::default()
            )
            .unwrap_err()
            .code,
            "RUST_XSLT_STREAM_UNSUPPORTED",
            "{expression}"
        );
    }
}
#[test]
fn wildcard_pattern_priority_is_between_any_and_specific_name() {
    let s = r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" version="3.0"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="r/*"/></xsl:template><xsl:template match="*">any;</xsl:template><xsl:template match="p:*">namespace;</xsl:template><xsl:template match="p:n">name;</xsl:template></xsl:stylesheet>"##;
    assert_eq!(
        transform(s, "<r xmlns:p='urn:p'><x/><p:x/><p:n/></r>").unwrap(),
        "any;namespace;name;"
    );
}
