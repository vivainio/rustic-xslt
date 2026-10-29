use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:value-of select="{expression}" separator="|"/></xsl:template></xsl:stylesheet>"#
    )
}
fn evaluate(expression: &str) -> String {
    Stylesheet::compile(&source(expression))
        .unwrap()
        .transform("<r><a>1</a><b>2</b><c>3</c></r>")
        .unwrap()
}
#[test]
fn set_operators_normalize_order_and_remove_duplicates_by_identity() {
    for (expr, expected) in [
        ("(r/c,r/a,r/c) union (r/b,r/a)", "1|2|3"),
        ("(r/c,r/a,r/c) intersect (r/c,r/a)", "1|3"),
        ("(r/c,r/a,r/c,r/b) except r/b", "1|3"),
        ("r/a intersect r/b", ""),
        ("r/a except r/a", ""),
        ("(r/a union r/a) is r/a", "true"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn empty_operands_preserve_set_identities() {
    for (expr, expected) in [
        ("() union r/a", "1"),
        ("r/a union ()", "1"),
        ("() intersect r/a", ""),
        ("r/a intersect ()", ""),
        ("() except r/a", ""),
        ("r/a except ()", "1"),
    ] {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}
#[test]
fn intersect_and_except_bind_more_tightly_than_union_and_are_left_associative() {
    for (expr, expected) in [
        ("r/a union r/b intersect r/c", "1"),
        ("r/a | r/b except r/b", "1"),
        ("(r/a,r/b) except r/a intersect r/b", "2"),
        ("(r/a,r/b) except r/a except r/b", ""),
        ("r/a union r/b instance of node()+", "error"),
    ] {
        if expected == "error" {
            assert_eq!(
                Stylesheet::compile(&source(expr))
                    .unwrap()
                    .transform("<r><a/><b/></r>")
                    .unwrap_err()
                    .code,
                "XPTY0004"
            );
        } else {
            assert_eq!(evaluate(expr), expected, "{expr}");
        }
    }
}
#[test]
fn atomic_operands_fail_even_with_empty_other_operand() {
    for op in ["union", "intersect", "except"] {
        for expr in [
            format!("1 {op} ()"),
            format!("() {op} 1"),
            format!("r/a {op} (r/a, 1)"),
        ] {
            assert_eq!(
                Stylesheet::compile(&source(&expr))
                    .unwrap()
                    .transform("<r><a/></r>")
                    .unwrap_err()
                    .code,
                "XPTY0004",
                "{expr}"
            );
        }
    }
}
#[test]
fn set_operators_keep_distinct_constructed_nodes_and_attributes() {
    let style = source("1").replace(r#"<xsl:value-of select="1" separator="|"/>"#, r#"<xsl:variable name="a" as="element()"><a/></xsl:variable><xsl:variable name="b" as="element()"><a/></xsl:variable><xsl:variable name="v" as="attribute()+"><xsl:attribute name="a">v</xsl:attribute><xsl:attribute name="a">v</xsl:attribute></xsl:variable><xsl:value-of select="concat(count($a union $b), ':', count($a intersect $b), ':', count($v except $v[1]), ':', ($v intersect $v[1]) is $v[1])"/>"#);
    assert_eq!(
        Stylesheet::compile(&style)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "2:0:1:true"
    );
}
#[test]
fn set_membership_scans_consume_shared_budget() {
    let s = Stylesheet::compile(&source("(for $x in 1 to 80 return r) except ()")).unwrap();
    let opts = TransformOptions {
        limits: Limits {
            max_steps: 350,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap_err().code,
        "XPDY0130"
    );
}
