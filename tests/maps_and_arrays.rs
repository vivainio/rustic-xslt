use rust_xslt::Stylesheet;
fn run(expr: &str) -> Result<String, rust_xslt::Diagnostic> {
    let expr = expr
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;");
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:map="http://www.w3.org/2005/xpath-functions/map" xmlns:array="http://www.w3.org/2005/xpath-functions/array"><xsl:output method="text"/>
    <xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#))?
    .transform("<r><n k='a'>1</n><n k='b'>2</n></r>")
}
fn check(cases: &[(&str, &str)]) {
    for (expr, expected) in cases {
        assert_eq!(run(expr).unwrap(), *expected, "{expr}");
    }
}
#[test]
fn map_constructors_lookup_and_functions() {
    check(&[
        ("map { 'a': 1, 'b': 2 }?a", "1"),
        ("map{'a':1,'b':2}('b')", "2"),
        ("map { 1: 'x', 2: 'y' }?2", "y"),
        ("let $k := 'z' return map { $k: 5 }?z", "5"),
        ("map { 'a': (1, 2, 3) }?a", "1 2 3"),
        ("empty(map { 'a': 1 }?b)", "true"),
        ("map { 'a': 1, 'b': 2 }?*", "1 2"),
        ("map:size(map { 'a': 1, 'b': 2 })", "2"),
        ("map:keys(map { 'a': 1, 'b': 2 })", "a b"),
        ("map:contains(map { 'a': 1 }, 'a')", "true"),
        ("map:contains(map { 'a': 1 }, 'b')", "false"),
        ("map:get(map { 'a': 1 }, 'a')", "1"),
        ("map:get(map { 'a': 1 }, 'x')", ""),
        ("map:put(map { 'a': 1 }, 'a', 9)?a", "9"),
        ("map:size(map:put(map { 'a': 1 }, 'b', 2))", "2"),
        ("map:entry('k', 'v')?k", "v"),
        ("map:size(map:remove(map { 'a': 1, 'b': 2 }, 'a'))", "1"),
        (
            "map:keys(map:remove(map { 'a': 1, 'b': 2, 'c': 3 }, ('a', 'c')))",
            "b",
        ),
        ("map:size(map:merge((map { 'a': 1 }, map { 'b': 2 })))", "2"),
        ("map:merge((map { 'a': 1 }, map { 'a': 2 }))?a", "1"),
        (
            "map:merge((map { 'a': 1 }, map { 'a': 2 }), map { 'duplicates': 'use-last' })?a",
            "2",
        ),
        (
            "map:merge((map { 'a': 1 }, map { 'a': 2 }), map { 'duplicates': 'combine' })?a",
            "1 2",
        ),
        ("map:size(map:merge(()))", "0"),
        ("map { 'a': 1 }?a + map { 'a': 2 }?a", "3"),
        ("(map { 'x': map { 'y': 'deep' } })?x?y", "deep"),
        (
            "map:for-each(map { 'a': 1, 'b': 2 }, function($k, $v) { concat($k, $v) })",
            "a1 b2",
        ),
        (
            "for-each(map:keys(map { 'a': 1, 'b': 2 }), map { 'a': 'A', 'b': 'B' })",
            "A B",
        ),
        ("map { 'a': 1 } instance of map(*)", "true"),
        (
            "map { 'a': 1 } instance of map(xs:string, xs:integer)",
            "true",
        ),
        ("map { 'a': 1 } instance of function(*)", "true"),
        ("map { 'a': 1 } instance of array(*)", "false"),
        (
            "deep-equal(map { 'a': (1, 2) }, map { 'a': (1, 2) })",
            "true",
        ),
        (
            "deep-equal(map { 'a': 1, 'b': 2 }, map { 'b': 2, 'a': 1 })",
            "true",
        ),
        ("deep-equal(map { 'a': 1 }, map { 'a': 2 })", "false"),
    ]);
}
#[test]
fn array_constructors_lookup_and_functions() {
    check(&[
        ("[1, 2, 3]?2", "2"),
        ("[1, 2, 3](3)", "3"),
        ("[(1, 2), 3]?1", "1 2"),
        ("array { 1 to 3 }?3", "3"),
        ("array:size(array { (1, 2, 3) })", "3"),
        ("array:size([])", "0"),
        ("array:size([[1, 2], [3]])", "2"),
        ("[1, 2, 3]?*", "1 2 3"),
        ("[[1, 2], [3]]?*?*", "1 2 3"),
        ("array:get([1, 2, 3], 2)", "2"),
        ("array:put([1, 2, 3], 2, 'x')?2", "x"),
        ("array:append([1], 2)?2", "2"),
        ("array:subarray([1, 2, 3, 4], 2, 2)?*", "2 3"),
        ("array:subarray([1, 2, 3, 4], 3)?*", "3 4"),
        ("array:remove([1, 2, 3, 4], (1, 3))?*", "2 4"),
        ("array:insert-before([1, 3], 2, 2)?*", "1 2 3"),
        ("array:head([1, 2, 3])", "1"),
        ("array:tail([1, 2, 3])?*", "2 3"),
        ("array:reverse([1, 2, 3])?*", "3 2 1"),
        ("array:join(([1, 2], [3]))?*", "1 2 3"),
        ("array:flatten([1, [2, [3, 4]], 5])", "1 2 3 4 5"),
        (
            "array:for-each([1, 2, 3], function($x) { $x * 2 })?*",
            "2 4 6",
        ),
        (
            "array:filter([1, 2, 3, 4], function($x) { $x mod 2 = 0 })?*",
            "2 4",
        ),
        (
            "array:fold-left([1, 2, 3], 0, function($a, $b) { $a + $b })",
            "6",
        ),
        (
            "array:fold-right([1, 2, 3], '', function($a, $b) { concat($a, $b) })",
            "123",
        ),
        (
            "array:for-each-pair([1, 2], [10, 20], function($a, $b) { $a + $b })?*",
            "11 22",
        ),
        ("array:sort([3, 1, 2])?*", "1 2 3"),
        ("array:sort(['b', 'a'])?*", "a b"),
        ("array:sort([3, 1, 2], (), function($x) { -$x })?*", "3 2 1"),
        ("[1, 2] instance of array(*)", "true"),
        ("[1, 2] instance of array(xs:integer)", "true"),
        ("[1, 2] instance of map(*)", "false"),
        ("[1, 2] instance of function(*)", "true"),
        ("deep-equal([1, 2], [1, 2])", "true"),
        ("deep-equal([1, 2], [2, 1])", "false"),
        ("deep-equal([1, [2]], [1, [2]])", "true"),
        ("let $a := [1, 2, 3] return for-each(2 to 3, $a)", "2 3"),
        ("count([(), ()])", "1"),
        ("array:size([(), ()])", "2"),
    ]);
}
#[test]
fn lookup_operator_forms_and_errors() {
    check(&[
        (
            "let $m := map { 'a': 1, 'b': 2 } return $m?('a', 'b')",
            "1 2",
        ),
        ("let $m := map { 'a': 1 } return $m ! ?a", "1"),
        ("(map { 'a': 1 }, map { 'a': 2 })?a", "1 2"),
        ("let $a := [10, 20, 30] return $a?(2)", "20"),
        ("let $a := [10, 20, 30] return $a?(1 to 2)", "10 20"),
        ("(1 to 3) ! [., . * 2]?2", "2 4 6"),
        ("let $m := map { 'k': [1, 2] } return $m?k?2", "2"),
        ("let $m := map { 'k': [1, 2] } return $m('k')(1)", "1"),
    ]);
    for expr in [
        "map { 'a': 1, 'a': 2 }",
        "map { 1: 'int', 1.0: 'dec' }",
        "map { (1, 2): 3 }",
        "[1, 2]?3",
        "[1, 2]?0",
        "[1, 2]?a",
        "1?a",
        "array:get([1], 'x')",
        "array:head([])",
        "array:tail([])",
        "array:subarray([1, 2], 3, 2)",
        "map:merge((map { 'a': 1 }, map { 'a': 2 }), map { 'duplicates': 'reject' })",
        "map { 'a': 1 }(1, 2)",
        "map:size([1])",
        "array:size(map {})",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
}
#[test]
fn map_find_searches_nested_containers() {
    check(&[
        (
            "array:size(map:find(map { 'a': 1, 'b': map { 'a': 2 } }, 'a'))",
            "2",
        ),
        (
            "map:find(map { 'a': 1, 'b': map { 'a': 2 } }, 'a')?*",
            "1 2",
        ),
        (
            "map:find(([map { 'k': 'x' }], map { 'k': 'y' }), 'k')?*",
            "x y",
        ),
        ("array:size(map:find(map { 'a': 1 }, 'zz'))", "0"),
    ]);
}
#[test]
fn parse_json_builds_maps_arrays_and_atomic_values() {
    check(&[
        (
            r#"parse-json('{"a": 1, "b": [true, false, null, "x"]}')?a"#,
            "1",
        ),
        (
            r#"parse-json('{"a": 1, "b": [true, false, null, "x"]}')?b?4"#,
            "x",
        ),
        (
            r#"array:size(parse-json('{"b": [true, false, null, "x"]}')?b)"#,
            "4",
        ),
        (r#"empty(parse-json('{"b": [null]}')?b?1)"#, "true"),
        (r#"parse-json('"café 😀"')"#, "café 😀"),
        ("parse-json('12.5e1') instance of xs:double", "true"),
        ("parse-json('-0') instance of xs:double", "true"),
        ("parse-json('  [1, 2, 3]  ')?*", "1 2 3"),
        ("empty(parse-json('null'))", "true"),
        ("map:size(parse-json('{}'))", "0"),
        (r#"parse-json('{"a": 1, "a": 2}')?a"#, "1"),
        (
            r#"parse-json('{"a": 1, "a": 2}', map { 'duplicates': 'use-last' })?a"#,
            "2",
        ),
        (r#"parse-json('"\ud800"')"#, "\u{FFFD}"),
        ("empty(parse-json(()))", "true"),
    ]);
    for expr in [
        "parse-json('')",
        "parse-json('{')",
        "parse-json('[1,]')",
        "parse-json('01')",
        "parse-json('1.')",
        "parse-json(concat('\"a', codepoints-to-string(9), 'b\"'))",
        r#"parse-json('"\x"')"#,
        "parse-json('[1] 2')",
        "parse-json('nul')",
        r#"parse-json('{"a": 1, "a": 2}', map { 'duplicates': 'reject' })"#,
        "parse-json('1', map { 'duplicates': 'bogus' })",
    ] {
        assert!(run(expr).is_err(), "{expr}");
    }
    assert_eq!(run("parse-json('[')").unwrap_err().code, "FOJS0001");
}
#[test]
fn large_maps_stay_fast_and_keys_follow_same_key_equality() {
    check(&[
        (
            "map:size(map:merge(for $i in 1 to 5000 return map:entry($i, $i)))",
            "5000",
        ),
        (
            "let $m := map:merge(for $i in 1 to 3000 return map:entry(string($i), $i)) return $m('2999') + $m('1')",
            "3000",
        ),
        ("map:get(map { 1: 'a' }, 1.0)", "a"),
        ("map:get(map { 1: 'a' }, xs:double(1))", "a"),
        ("map:get(map { 'k': 'a' }, xs:untypedAtomic('k'))", "a"),
        (
            "map:contains(map { xs:double('NaN'): 1 }, xs:double('NaN'))",
            "true",
        ),
        ("map:get(map { 0.0e0: 'z' }, xs:double('-0'))", "z"),
        (
            "map:size(map:remove(map { 1: 'a', 2: 'b', 3: 'c' }, 2))",
            "2",
        ),
        (
            "map:get(map:remove(map { 1: 'a', 2: 'b', 3: 'c' }, 2), 3)",
            "c",
        ),
        (
            "map:keys(map:remove(map { 'x': 1, 'y': 2, 'z': 3 }, 'x'))",
            "y z",
        ),
    ]);
}
