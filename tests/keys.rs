use rust_xslt::Stylesheet;
const SOURCE: &str = "<r><item id='a' g='x'>1</item><item id='b' g='y'>2</item><item id='c' g='x'>3</item><grp><item id='d' g='x'>4</item></grp></r>";
fn run(declarations: &str, select: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:k="urn:k"><xsl:output method="text"/>{declarations}<xsl:template match="/"><xsl:sequence select="{select}"/></xsl:template></xsl:stylesheet>"#))?
        .transform(SOURCE)
}
const KEYS: &str = r#"<xsl:key name="by-id" match="item" use="@id"/><xsl:key name="by-group" match="item" use="@g"/><xsl:key name="q" xmlns:k="urn:k" match="item" use="string(.)"/>"#;
#[test]
fn key_returns_matching_nodes_in_document_order() {
    for (select, expected) in [
        ("string(key('by-id', 'b'))", "2"),
        ("string-join(key('by-group', 'x'), ',')", "1,3,4"),
        ("string-join(key('by-group', ('y', 'x')), ',')", "1,2,3,4"),
        ("count(key('by-group', 'none'))", "0"),
        ("count(key('by-group', ()))", "0"),
        ("string(key('by-id', 'a')/@g)", "x"),
        ("string-join(key('by-group', 'x', r/grp), ',')", "4"),
        ("string-join(r/grp/key('by-group', 'x'), ',')", "1,3,4"),
        ("string(key('q', '3'))", "3"),
        ("count(key('by-id', 'a') | key('by-group', 'x'))", "3"),
    ] {
        assert_eq!(run(KEYS, select).unwrap(), expected, "{select}");
    }
}
#[test]
fn key_names_may_be_qualified_and_errors_are_reported() {
    let qualified = r#"<xsl:key name="k:ids" match="item" use="@id"/>"#;
    assert_eq!(run(qualified, "string(key('k:ids', 'c'))").unwrap(), "3");
    assert_eq!(
        run(KEYS, "key('missing', 'a')").unwrap_err().code,
        "XTDE1260"
    );
    assert!(Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:key name="x" match="a"/></xsl:stylesheet>"#).is_err());
}

#[test]
fn keys_compare_atomic_values_without_stringifying_typed_values() {
    let numeric = r#"<xsl:key name="n" match="item" use="number(.)"/>"#;
    for (query, expected) in [
        ("count(key('n', 3))", "1"),
        ("count(key('n', '3'))", "0"),
        ("count(key('n', r/item[3]))", "0"),
        ("count(key('n', [2, 3]))", "2"),
        ("count(key('n', true()))", "0"),
        ("count(key('n', number('NaN')))", "0"),
    ] {
        assert_eq!(run(numeric, query).unwrap(), expected, "{query}");
    }
    assert_eq!(run(KEYS, "count(key('q', 3))").unwrap(), "0");
    let nan = r#"<xsl:key name="n" match="item" use="number('NaN')"/>"#;
    assert_eq!(run(nan, "count(key('n', number('NaN')))").unwrap(), "0");
    let qnames = r#"<xsl:key name="n" match="item" use="QName('urn:n', 'p:n')"/>"#;
    assert_eq!(
        run(qnames, "count(key('n', QName('urn:n', 'q:n')))").unwrap(),
        "4"
    );
}

#[test]
fn keys_atomize_and_reject_function_items() {
    for query in ["key('by-id', map{})", "key('by-id', function(){1})"] {
        assert_eq!(run(KEYS, query).unwrap_err().code, "FOTY0013");
    }
    let declarations = r#"<xsl:key name="n" match="item" use="map{}"/>"#;
    assert_eq!(
        run(declarations, "key('n', 1)").unwrap_err().code,
        "FOTY0013"
    );
}

#[test]
fn key_declarations_use_globals_even_when_callers_shadow_them() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
      <xsl:output method="text"/>
      <xsl:variable name="v" select="'x'"/>
      <xsl:key name="k" match="item[@g=$v]" use="$v"/>
      <xsl:template match="/"><xsl:variable name="v" select="'y'"/>
        <xsl:value-of select="count(key('k', 'x'))"/>
      </xsl:template>
    </xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform(SOURCE)
            .unwrap(),
        "3"
    );
}

#[test]
fn backwards_keys_stringify_both_sides_even_for_modern_callers() {
    let declarations = r#"<xsl:key version="1.0" name="n" match="item" use="number(.)"/>"#;
    assert_eq!(run(declarations, "count(key('n', '3'))").unwrap(), "1");
    // One backwards-compatible declaration enables string comparison for the whole key.
    let mixed = format!(r#"{declarations}<xsl:key name="n" match="item" use="number(.)"/>"#);
    assert_eq!(run(&mixed, "count(key('n', '3'))").unwrap(), "1");
}

#[test]
fn subtree_key_includes_the_top_node_and_its_attributes() {
    assert_eq!(
        run(KEYS, "count(key('by-id', 'a', r/item[1]))").unwrap(),
        "1"
    );
    assert_eq!(
        run(KEYS, "count(key('by-id', 'b', r/item[1]))").unwrap(),
        "0"
    );
    let attributes = r#"<xsl:key name="a" match="@id" use="."/>"#;
    assert_eq!(
        run(attributes, "string(key('a', 'a', r/item[1]/@id))").unwrap(),
        "a"
    );
    assert_eq!(
        run(attributes, "count(key('a', 'b', r/item[1]/@id))").unwrap(),
        "0"
    );
}

#[test]
fn temporal_keys_compare_values_across_timezone_lexical_forms() {
    let declarations = r#"<xsl:key xmlns:xs="http://www.w3.org/2001/XMLSchema" name="date" match="item" use="xs:dateTime('2020-01-01T12:00:00Z')"/>"#;
    let query = "count(key('date', Q{http://www.w3.org/2001/XMLSchema}dateTime('2020-01-01T13:00:00+01:00')))";
    assert_eq!(run(declarations, query).unwrap(), "4");
    assert_eq!(
        run(declarations, "count(key('date', '2020-01-01T12:00:00Z'))").unwrap(),
        "0"
    );
}

#[test]
fn key_rejects_trees_without_document_roots() {
    let source = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
      <xsl:key name="k" match="item" use="@id"/>
      <xsl:template match="/"><xsl:variable name="tree" as="element()"><item id="a"/></xsl:variable>
        <xsl:sequence select="key('k', 'a', $tree)"/>
      </xsl:template>
    </xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(source)
            .unwrap()
            .transform(SOURCE)
            .unwrap_err()
            .code,
        "XTDE1270"
    );
}
