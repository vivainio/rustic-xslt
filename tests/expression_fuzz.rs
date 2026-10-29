//! Deterministic structured fuzzing: random well-formed XPath expressions must fail
//! cleanly or finish within their budget, never panic. Set FUZZ_SEED to explore.
use rust_xslt::{Limits, Stylesheet, TransformOptions};
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() as usize) % items.len()]
    }
}
const ATOMS: &[&str] = &[
    "1",
    "0",
    "-1",
    "2.5",
    "1e3",
    "'a'",
    "''",
    "'2020-01-02'",
    "'abc'",
    "()",
    "r",
    "r/i",
    "r/i[1]",
    "$x",
    ".",
    "true()",
    "false()",
    "xs:date('2020-01-02')",
    "xs:dateTime('2020-01-02T03:04:05')",
    "xs:duration('P1D')",
    "xs:time('10:00:00')",
    "xs:gYear('2020')",
    "map{'a':1}",
    "[1,2,3]",
    "(1,2,3)",
    "1 to 3",
    "'a b c'",
    "xs:hexBinary('0A')",
    "xs:QName('x')",
    "xs:double('NaN')",
    "xs:integer(5)",
];
const UNARY: &[&str] = &[
    "string",
    "number",
    "count",
    "sum",
    "avg",
    "min",
    "max",
    "abs",
    "floor",
    "round",
    "ceiling",
    "boolean",
    "not",
    "exists",
    "empty",
    "reverse",
    "head",
    "tail",
    "upper-case",
    "lower-case",
    "normalize-space",
    "string-length",
    "data",
    "distinct-values",
    "node-name",
    "name",
    "local-name",
    "root",
    "path",
    "format-integer",
    "generate-id",
    "xs:integer",
    "xs:string",
    "xs:double",
    "xs:decimal",
    "xs:date",
    "xs:time",
    "xs:dateTime",
    "xs:duration",
    "xs:gYear",
    "xs:boolean",
    "xs:float",
    "year-from-date",
    "month-from-dateTime",
    "hours-from-time",
    "seconds-from-duration",
    "map:keys",
    "map:size",
    "array:size",
    "array:flatten",
    "array:reverse",
    "parse-json",
    "encode-for-uri",
    "iri-to-uri",
    "string-to-codepoints",
    "codepoints-to-string",
    "exactly-one",
    "one-or-more",
    "zero-or-one",
    "innermost",
    "outermost",
    "math:sqrt",
    "math:exp",
    "resolve-uri",
];
const BINARY: &[&str] = &[
    "concat",
    "contains",
    "starts-with",
    "ends-with",
    "substring-before",
    "substring-after",
    "compare",
    "deep-equal",
    "index-of",
    "insert-before",
    "subsequence",
    "remove",
    "matches",
    "tokenize",
    "string-join",
    "format-number",
    "format-date",
    "format-time",
    "format-dateTime",
    "xs:dateTime",
    "map:get",
    "map:contains",
    "array:get",
    "array:append",
    "sort",
    "for-each",
    "filter",
    "fold-left",
    "round-half-to-even",
    "substring",
    "translate",
    "adjust-date-to-timezone",
    "math:pow",
    "apply",
    "dateTime",
    "replace",
];
fn expr(rng: &mut Lcg, depth: usize) -> String {
    if depth == 0 {
        return rng.pick(ATOMS).to_owned();
    }
    match rng.next() % 12 {
        0 => format!("{}({})", rng.pick(UNARY), expr(rng, depth - 1)),
        1 | 2 => format!(
            "{}({}, {})",
            rng.pick(BINARY),
            expr(rng, depth - 1),
            expr(rng, depth - 1)
        ),
        3 => format!(
            "{}({}, {}, {})",
            rng.pick(BINARY),
            expr(rng, depth - 1),
            expr(rng, depth - 1),
            expr(rng, depth - 1)
        ),
        4 => format!(
            "({} {} {})",
            expr(rng, depth - 1),
            rng.pick(&[
                "+",
                "-",
                "*",
                "div",
                "idiv",
                "mod",
                "=",
                "!=",
                "lt",
                "eq",
                "<<",
                "||",
                "to",
                "and",
                "or",
                "union",
                "except",
                "intersect",
                "!",
                "is"
            ]),
            expr(rng, depth - 1)
        ),
        5 => format!(
            "if ({}) then {} else {}",
            expr(rng, depth - 1),
            expr(rng, depth - 1),
            expr(rng, depth - 1)
        ),
        6 => format!(
            "for $i in {} return {}",
            expr(rng, depth - 1),
            expr(rng, depth - 1)
        ),
        7 => format!("({}, {})", expr(rng, depth - 1), expr(rng, depth - 1)),
        8 => format!(
            "{} {}",
            expr(rng, depth - 1),
            rng.pick(&[
                "cast as xs:integer",
                "castable as xs:date",
                "instance of xs:string",
                "treat as item()*",
                "cast as xs:string?",
                "instance of map(*)",
                "cast as xs:gYearMonth"
            ])
        ),
        9 => format!(
            "function($a, $b) {{ {} }}({}, {})",
            expr(rng, depth - 1),
            expr(rng, depth - 1),
            expr(rng, depth - 1)
        ),
        10 => format!(
            "{}?{}",
            expr(rng, depth - 1),
            rng.pick(&["a", "1", "*", "(1, 2)"])
        ),
        _ => format!("{}[{}]", expr(rng, depth - 1), expr(rng, depth - 1)),
    }
}
#[test]
fn structured_random_expressions_never_panic_or_hang() {
    let seed: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(987654321);
    let mut rng = Lcg(seed);
    let limits = Limits {
        max_steps: 50_000,
        max_output_bytes: 200_000,
        ..Limits::default()
    };
    let mut compiled = 0;
    let mut ran = 0;
    for round in 0..15_000 {
        let depth = 1 + (rng.next() % 4) as usize;
        let e = expr(&mut rng, depth);
        let attr = e
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('"', "&quot;");
        let source = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:map="http://www.w3.org/2005/xpath-functions/map" xmlns:array="http://www.w3.org/2005/xpath-functions/array" xmlns:math="http://www.w3.org/2005/xpath-functions/math"><xsl:param name="x" select="1"/><xsl:template match="/"><xsl:sequence select="{attr}"/></xsl:template></xsl:stylesheet>"#
        );
        let outcome = std::panic::catch_unwind(|| {
            let mut stage = 0;
            if let Ok(sheet) = Stylesheet::compile(&source) {
                stage = 1;
                if sheet
                    .transform_with_options(
                        "<r><i>1</i><i>2</i></r>",
                        &TransformOptions {
                            limits,
                            ..TransformOptions::default()
                        },
                    )
                    .is_ok()
                {
                    stage = 2;
                }
            }
            stage
        });
        match outcome {
            Ok(stage) => {
                compiled += usize::from(stage >= 1);
                ran += usize::from(stage == 2);
            }
            Err(_) => panic!("panic on round {round}: {e}"),
        }
    }
    println!("compiled {compiled} ran {ran}");
}
