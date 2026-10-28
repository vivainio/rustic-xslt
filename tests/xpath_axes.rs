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
