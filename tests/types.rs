use rust_xslt::{Stylesheet, transform};
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" version="3.0"><xsl:output method="text"/>{body}</xsl:stylesheet>"##
    )
}
fn evaluate(e: &str) -> String {
    transform(
        &sheet(&format!(
            r##"<xsl:template match="/"><xsl:value-of select="{e}"/></xsl:template>"##
        )),
        "<r/>",
    )
    .unwrap_or_else(|error| panic!("{e}: {error}"))
}
#[test]
fn instance_of_tests_atomic_types_subtyping_and_cardinality() {
    for (e, want) in [
        ("1 instance of xs:integer", "true"),
        ("1 instance of xs:decimal", "true"),
        ("1 instance of xs:double", "false"),
        ("xs:float('0.1') instance of xs:float", "true"),
        ("xs:float('0.1') instance of xs:double", "false"),
        ("xs:untypedAtomic('x') instance of xs:string", "false"),
        ("(1,2) instance of xs:integer+", "true"),
        ("() instance of xs:integer?", "true"),
        ("() instance of empty-sequence()", "true"),
        ("() instance of item()", "false"),
        ("/ instance of document-node()", "true"),
        ("r instance of element()", "true"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
}
#[test]
fn casts_castable_and_treat_have_distinct_behavior() {
    for (e, want) in [
        ("'12' cast as xs:integer", "12"),
        ("'no' castable as xs:integer", "false"),
        ("'true' castable as xs:boolean", "true"),
        ("() cast as xs:integer?", ""),
        ("() castable as xs:integer", "false"),
        ("(1,2) castable as xs:integer", "false"),
        ("1 treat as xs:decimal", "1"),
        ("-1 cast as xs:string", "-1"),
        ("'3' cast as xs:integer * 2", "6"),
    ] {
        assert_eq!(evaluate(e), want, "{e}");
    }
    let s = sheet(
        r##"<xsl:template match="/"><xsl:value-of select="'12' treat as xs:integer"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XPDY0050");
}
#[test]
fn type_operator_and_union_precedence_follow_xpath_grammar() {
    assert_eq!(evaluate("(1 + 1) instance of xs:integer"), "true");
    let s = sheet(
        r##"<xsl:template match="/"><xsl:value-of select="1 + 1 instance of xs:integer"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XPTY0004");
    for e in [
        "1 instance of xs:integer instance of xs:boolean",
        "1 instance of xs:integer cast as xs:string",
    ] {
        assert!(
            Stylesheet::compile(&sheet(&format!(
                r##"<xsl:template match="/"><xsl:value-of select="{e}"/></xsl:template>"##
            )))
            .is_err()
        );
    }
}
#[test]
fn binding_types_apply_atomization_and_numeric_promotion() {
    let s = sheet(
        r##"<xsl:variable name="n" as="xs:integer" select="r/@n"/><xsl:template match="/"><xsl:variable name="d" as="xs:double" select="$n"/><xsl:value-of select="concat($n + 1, ':', $d instance of xs:double)"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r n='41'/>").unwrap(), "42:true");
    assert_eq!(transform(&s, "<r n='bad'/>").unwrap_err().code, "XTTE0570");
}
#[test]
fn typed_text_content_and_optional_empty_defaults() {
    let s = sheet(
        r##"<xsl:variable name="n" as="xs:integer">12</xsl:variable><xsl:variable name="empty" as="xs:integer?"/><xsl:template match="/"><xsl:value-of select="concat($n + 1, ':', empty($empty))"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r/>").unwrap(), "13:true");
}
#[test]
fn binding_cardinality_and_typed_string_to_integer_are_checked() {
    for binding in [
        r##"<xsl:variable name="x" as="xs:integer" select="(1,2)"/>"##,
        r##"<xsl:variable name="x" as="xs:integer" select="'12'"/>"##,
        r##"<xsl:variable name="x" as="xs:integer"/>"##,
    ] {
        let s = sheet(&format!(r##"{binding}<xsl:template match="/"/>"##));
        assert_eq!(
            transform(&s, "<r/>").unwrap_err().code,
            "XTTE0570",
            "{binding}"
        );
    }
}
#[test]
fn template_parameter_function_conversion_and_supplied_error_code() {
    let s = sheet(
        r##"<xsl:template match="/"><xsl:call-template name="target"><xsl:with-param name="n" select="r/@n"/></xsl:call-template></xsl:template><xsl:template name="target"><xsl:param name="n" as="xs:integer"/><xsl:value-of select="$n + 1"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r n='9'/>").unwrap(), "10");
    assert_eq!(transform(&s, "<r n='bad'/>").unwrap_err().code, "XTTE0590");
}
#[test]
fn node_sequence_types_do_not_atomize_nodes() {
    let s = sheet(
        r##"<xsl:variable name="nodes" as="element()+" select="r/n"/><xsl:template match="/"><xsl:value-of select="count($nodes)"/></xsl:template>"##,
    );
    assert_eq!(transform(&s, "<r><n/><n/></r>").unwrap(), "2");
    assert_eq!(transform(&s, "<r/>").unwrap_err().code, "XTTE0570");
}
#[test]
fn floats_retain_identity_through_arithmetic_and_promotion() {
    assert_eq!(
        evaluate("(xs:float('0.1') + xs:float('0.2')) instance of xs:float"),
        "true"
    );
    assert_eq!(
        evaluate("(xs:float('0.1') + 1e0) instance of xs:double"),
        "true"
    );
    assert_eq!(evaluate("xs:float('0.1')"), "0.1");
}
