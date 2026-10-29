use rust_xslt::Stylesheet;
fn run(expr: &str) -> Result<String, rust_xslt::Diagnostic> {
    let expr = expr
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;");
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:f="urn:f"><xsl:output method="text"/>
    <xsl:function name="f:inc"><xsl:param name="x"/><xsl:sequence select="$x + 1"/></xsl:function>
    <xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#))?
    .transform("<r><n>3</n><n>1</n><n>2</n></r>")
}
#[test]
fn inline_functions_close_over_variables_and_can_be_called_dynamically() {
    for (expr, expected) in [
        ("function($x) { $x * 2 }(21)", "42"),
        (
            "let $f := function($a, $b) { $a + $b } return $f(1, 2)",
            "3",
        ),
        (
            "let $n := 10, $add := function($x) { $x + $n } return $add(5)",
            "15",
        ),
        ("let $f := function() { 7 } return $f()", "7"),
        (
            "let $f := function($x as xs:integer) as xs:integer { $x + 1 } return $f(1)",
            "2",
        ),
        (
            "let $f := function($x) { function($y) { $x * $y } } return $f(3)(4)",
            "12",
        ),
        (
            "(function($x) { $x }, function($x) { $x + 1 })[2](10)",
            "11",
        ),
        (
            "let $f := function($n) { if ($n le 1) then 1 else $n } return $f(0)",
            "1",
        ),
        ("empty(function() {}())", "true"),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    for expr in [
        "let $f := function($x as xs:integer) { $x } return $f('a')",
        "let $f := function($x) { $x } return $f(1, 2)",
        "let $f := 1 return $f(1)",
        "let $f := function($x) as xs:string { $x } return $f(1)",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn named_function_references_cover_builtin_user_and_constructor_functions() {
    for (expr, expected) in [
        ("let $f := upper-case#1 return $f('abc')", "ABC"),
        ("let $f := concat#3 return $f('a', 'b', 'c')", "abc"),
        ("let $f := f:inc#1 return $f(1)", "2"),
        ("let $f := xs:integer#1 return $f('42') + 1", "43"),
        ("function-arity(substring#2)", "2"),
        ("function-arity(function($a, $b, $c) { 1 })", "3"),
        ("function-lookup(xs:QName('fn:abs'), 1)(-3)", "3"),
        ("empty(function-lookup(xs:QName('f:nope'), 1))", "true"),
        ("function-lookup(xs:QName('f:inc'), 1)(1)", "2"),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    assert!(run("nope#1").is_err());
    assert!(run("abs#5").is_err());
}
#[test]
fn higher_order_functions_apply_functions_to_sequences() {
    for (expr, expected) in [
        ("for-each(1 to 3, function($x) { $x * $x })", "1 4 9"),
        ("for-each(('a', 'b'), upper-case#1)", "A B"),
        ("filter(1 to 10, function($x) { $x mod 3 = 0 })", "3 6 9"),
        ("fold-left(1 to 5, 0, function($a, $b) { $a + $b })", "15"),
        (
            "fold-left(('a','b','c'), '', function($a, $b) { concat($a, $b) })",
            "abc",
        ),
        (
            "fold-right(('a','b','c'), '', function($a, $b) { concat($a, $b) })",
            "abc",
        ),
        (
            "fold-right(1 to 3, (), function($x, $acc) { $acc, $x })",
            "3 2 1",
        ),
        (
            "for-each-pair(1 to 3, (10, 20, 30), function($a, $b) { $a + $b })",
            "11 22 33",
        ),
        (
            "for-each-pair(1 to 3, (10, 20), function($a, $b) { $a + $b })",
            "11 22",
        ),
        (
            "sort(r/n, (), function($n) { number($n) })/string()",
            "1 2 3",
        ),
        ("sort(('bb', 'a', 'ccc'), (), string-length#1)", "a bb ccc"),
        ("sort((1, 2, 3), (), function($x) { -$x })", "3 2 1"),
        ("(1 to 3) => for-each(function($x) { $x + 1 })", "2 3 4"),
        (
            "(1 to 4) => filter(function($x) { $x > 2 }) => count()",
            "2",
        ),
        ("'abc' => upper-case() => string-length()", "3"),
        ("let $f := upper-case#1 return 'x' => $f()", "X"),
        ("'  a ' => normalize-space()", "a"),
        ("-3 => abs()", "3"),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    for expr in [
        "filter(1 to 3, function($x) { $x })",
        "for-each(1 to 3, 1)",
        "for-each(1 to 3, function($a, $b) { 1 })",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn function_types_support_instance_of_and_typed_parameters() {
    for (expr, expected) in [
        ("function($x) { $x } instance of function(*)", "true"),
        (
            "function($x) { $x } instance of function(item()*) as item()*",
            "true",
        ),
        (
            "function($x, $y) { $x } instance of function(item(), item()) as item()*",
            "true",
        ),
        (
            "function($x) { $x } instance of function(item(), item()) as item()*",
            "false",
        ),
        ("1 instance of function(*)", "false"),
        ("upper-case#1 instance of function(*)", "true"),
        ("(function() {}, 1) instance of item()*", "true"),
        (
            "count(for-each(1 to 3, function($x as xs:integer) as xs:integer { $x }))",
            "3",
        ),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
}
#[test]
fn partial_application_apply_and_function_name() {
    for (expr, expected) in [
        (
            "filter(('apple', 'pear', 'apricot'), starts-with(?, 'a'))",
            "apple apricot",
        ),
        ("fold-left(1 to 3, '', concat(?, '.', ?))", ".1.2.3"),
        (
            "let $add := function($a, $b) { $a + $b }, $inc := $add(1, ?) return $inc(41)",
            "42",
        ),
        ("let $f := substring(?, 2, ?) return $f('hello', 3)", "ell"),
        ("function-arity(substring(?, 2, ?))", "2"),
        ("apply(concat#3, ['a', 'b', 'c'])", "abc"),
        ("apply(function() { 'none' }, [])", "none"),
        ("string(function-name(upper-case#1))", "fn:upper-case"),
        ("string(function-name(f:inc#1))", "inc"),
        ("empty(function-name(function() { 1 }))", "true"),
        (
            "local-name-from-QName(function-name(xs:integer#1))",
            "integer",
        ),
    ] {
        assert_eq!(run(expr).unwrap(), expected, "{expr}");
    }
    for expr in [
        "apply(concat#3, ['a'])",
        "apply(1, [])",
        "starts-with(?, ?, ?, ?)",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
