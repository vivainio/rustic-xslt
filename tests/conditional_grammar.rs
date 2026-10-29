use rust_xslt::Stylesheet;
fn source(expression: &str) -> String {
    format!(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expression}"/></xsl:template></xsl:stylesheet>"#
    )
}
#[test]
fn conditional_expressions_require_parentheses_in_operator_operands() {
    for expr in [
        "1 + if (true()) then 2 else 3",
        "true() and if (true()) then true() else false()",
        "false() or if (true()) then true() else false()",
        "'a' || if (true()) then 'b' else 'c'",
        "1 to if (true()) then 2 else 3",
        "-if (true()) then 2 else 3",
        "1 ! if (true()) then 2 else 3",
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr)).unwrap_err().code,
            "XPST0003",
            "{expr}"
        );
    }
}
#[test]
fn parenthesized_conditionals_and_exprsingle_positions_remain_valid() {
    for (expr, expected) in [
        ("1 + (if (true()) then 2 else 3)", "3"),
        ("true() and (if (true()) then true() else false())", "true"),
        ("'a' || (if (true()) then 'b' else 'c')", "ab"),
        ("if (false()) then 1 else if (true()) then 2 else 3", "2"),
        ("concat(if (true()) then 'a' else 'b', 'c')", "ac"),
        ("(0, if (true()) then 1 else 2)", "0 1"),
        (
            "for $x in (1,2) return if ($x = 1) then 'a' else 'b'",
            "a b",
        ),
        ("let $x := if (true()) then 1 else 2 return $x", "1"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(expr))
                .unwrap()
                .transform("<r/>")
                .unwrap(),
            expected,
            "{expr}"
        );
    }
}
#[test]
fn conditional_keywords_remain_ordinary_element_names_outside_conditional_syntax() {
    let expr = "r/if/name(), r/some/name(), r/every/name()";
    assert_eq!(
        Stylesheet::compile(&source(expr))
            .unwrap()
            .transform("<r><if/><some/><every/></r>")
            .unwrap(),
        "if some every"
    );
}
