use rust_xslt::{Limits, Stylesheet, TransformOptions};
fn sheet(expr: &str) -> Stylesheet {
    let expr = expr
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;");
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#)).unwrap()
}
#[test]
fn regex_flags_unicode_name_classes_and_subtraction_follow_xpath_rules() {
    for (expr, expected) in [
        ("matches('abc','b')", "true"),
        ("matches('AbC','abc','i')", "true"),
        ("matches('abc','a b c','x')", "true"),
        ("matches('a#b','a#b','x')", "true"),
        ("matches('a.b','a.b','q')", "true"),
        ("matches('abc','a.b','q')", "false"),
        (r"matches('é:name','^\i\c*$')", "true"),
        (r"matches('bc','^[a-z-[aeiou]]+$')", "true"),
        (r"matches('ba','^[a-z-[aeiou]]+$')", "false"),
        (r"matches('é','\p{L}')", "true"),
        ("matches((), '^$')", "true"),
        ("matches(codepoints-to-string(13), '.')", "false"),
        ("matches(codepoints-to-string(13), '.', 's')", "true"),
        (r"matches(codepoints-to-string(160), '\s')", "false"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
}
#[test]
fn replace_uses_xpath_capture_numbers_and_literal_quoting() {
    for (expr, expected) in [
        ("replace('abracadabra','bra','*')", "a*cada*"),
        ("replace('ab','(a)(b)','$2$1')", "ba"),
        ("replace('ab','(a)(b)','$23')", "b3"),
        ("replace('ab','(a)(b)','$9')", ""),
        ("replace('ab','(a)(b)','$0')", "ab"),
        (r"replace('a','a','\$')", "$"),
        (r"replace('a','a','\\')", r"\"),
        ("replace('a.b','a.b','$1','q')", "$1"),
        ("replace((), 'a', 'b')", ""),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
}
#[test]
fn tokenize_preserves_boundary_empty_tokens_and_has_whitespace_form() {
    for (expr, expected) in [
        ("string-join(tokenize(',a,,b,', ','),'|')", "|a||b|"),
        ("string-join(tokenize('  a  b  '),'|')", "a|b"),
        ("count(tokenize('', ','))", "0"),
        ("count(tokenize(()))", "0"),
        ("string-join(tokenize('AaB','a','i'),'|')", "||B"),
    ] {
        assert_eq!(sheet(expr).transform("<r/>").unwrap(), expected, "{expr}");
    }
}
#[test]
fn regex_errors_distinguish_flags_syntax_empty_matches_and_replacements() {
    for (expr, code) in [
        ("matches('a','a','z')", "FORX0001"),
        ("matches('a','[')", "FORX0002"),
        (r"matches('a','\b')", "FORX0002"),
        ("matches('a','(?i)a')", "FORX0002"),
        ("replace('a','a*','b')", "FORX0003"),
        ("tokenize('', '')", "FORX0003"),
        ("replace('a','a','$')", "FORX0004"),
        (r"replace('a','a','\x')", "FORX0004"),
        ("matches(1,'a')", "XPTY0004"),
        ("matches('a',())", "XPTY0004"),
    ] {
        assert_eq!(
            sheet(expr).transform("<r/>").unwrap_err().code,
            code,
            "{expr}"
        );
    }
}
#[test]
fn regex_matching_and_replacement_expansion_use_the_shared_budget() {
    let options = TransformOptions {
        limits: Limits {
            max_steps: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    for expr in [
        "matches(string-join(1 to 20), 'aaaaaaaaaa')",
        "replace('aaaaaaaaaa','a','xxxxxxxxxxxxxxxxxxxx')",
    ] {
        assert_eq!(
            sheet(expr)
                .transform_with_options("<r/>", &options)
                .unwrap_err()
                .code,
            "XPDY0130"
        );
    }
}
