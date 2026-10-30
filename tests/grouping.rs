use rust_xslt::Stylesheet;
const SOURCE: &str = "<r><i k='a'>1</i><i k='b'>2</i><i k='a'>3</i><i k='a'>4</i><i k='c'>5</i><h>H1</h><i k='x'>6</i><h>H2</h><i k='y'>7</i></r>";
fn run(instruction: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/">{instruction}</xsl:template></xsl:stylesheet>"#))?
        .transform(SOURCE)
}
#[test]
fn group_by_collects_items_in_first_appearance_order() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k">[<xsl:value-of select="current-grouping-key()"/>:<xsl:value-of select="current-group()" separator=","/>]</xsl:for-each-group>"#).unwrap(),
        "[a:1,3,4][b:2][c:5][x:6][y:7]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="1 to 6" group-by="(., . + 1)[. le 6]">[<xsl:value-of select="current-grouping-key()"/>=<xsl:value-of select="count(current-group())"/>]</xsl:for-each-group>"#).unwrap(),
        "[1=1][2=2][3=2][4=2][5=2][6=2]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:value-of select="position()"/>/<xsl:value-of select="last()"/>;</xsl:for-each-group>"#).unwrap(),
        "1/5;2/5;3/5;4/5;5/5;"
    );
}
#[test]
fn adjacent_grouping_only_merges_consecutive_keys() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-adjacent="@k">[<xsl:value-of select="current-grouping-key()"/>:<xsl:value-of select="current-group()" separator=","/>]</xsl:for-each-group>"#).unwrap(),
        "[a:1][b:2][a:3,4][c:5][x:6][y:7]"
    );
    assert!(run(r#"<xsl:for-each-group select="r/i" group-adjacent="(1,2)"/>"#).is_err());
}
#[test]
fn starting_and_ending_patterns_split_node_sequences() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-starting-with="h">[<xsl:value-of select="count(current-group())"/>]</xsl:for-each-group>"#).unwrap(),
        "[5][2][2]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-ending-with="h">[<xsl:value-of select="count(current-group())"/>]</xsl:for-each-group>"#).unwrap(),
        "[6][2][1]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-starting-with="h"><xsl:value-of select="name(current-group()[1])"/>;</xsl:for-each-group>"#).unwrap(),
        "i;h;h;"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/*" group-starting-with="h"><xsl:value-of select="current-grouping-key()"/></xsl:for-each-group>"#).unwrap_err().code,
        "XTDE1071"
    );
    assert_eq!(
        run("<xsl:for-each-group select='1 to 3' group-starting-with='h'><xsl:value-of select='count(current-group())'/></xsl:for-each-group>").unwrap(),
        "3"
    );
}
#[test]
fn grouping_declaration_errors() {
    assert_eq!(
        run("<xsl:for-each-group select='r/i'/>").unwrap_err().code,
        "XTSE1080"
    );
    assert_eq!(
        run("<xsl:for-each-group select='r/i' group-by='1' group-adjacent='2'/>")
            .unwrap_err()
            .code,
        "XTSE1080"
    );
    assert_eq!(
        run("<xsl:value-of select='current-group()'/>")
            .unwrap_err()
            .code,
        "XTDE1061"
    );
}
#[test]
fn groups_can_be_sorted_by_keys_that_see_the_group() {
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:sort select="current-grouping-key()" order="descending"/>[<xsl:value-of select="current-grouping-key()"/>]</xsl:for-each-group>"#).unwrap(),
        "[y][x][c][b][a]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:sort select="count(current-group())" data-type="number" order="descending"/><xsl:sort select="current-grouping-key()"/>[<xsl:value-of select="current-grouping-key()"/>]</xsl:for-each-group>"#).unwrap(),
        "[a][b][c][x][y]"
    );
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:sort select="current-grouping-key()" order="descending"/><xsl:value-of select="position()"/>,</xsl:for-each-group>"#).unwrap(),
        "1,2,3,4,5,"
    );
}

#[test]
fn composite_grouping_uses_sequence_keys_including_empty_sequences() {
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2,3,4)" group-by="(if (. mod 2 = 0) then 0 else (), 9)" composite="yes"><xsl:value-of select="current-group()" separator=","/><xsl:text>;</xsl:text></xsl:for-each-group>"#).unwrap(), "1,3;2,4;");
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2,3)" group-adjacent="()" composite="yes"><xsl:value-of select="count(current-group())"/></xsl:for-each-group>"#).unwrap(), "3");
}

#[test]
fn explicit_codepoint_grouping_and_declaration_error_order() {
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2,1)" group-by="." collation="http://www.w3.org/2005/xpath-functions/collation/codepoint"><xsl:value-of select="count(current-group())"/></xsl:for-each-group>"#).unwrap(), "21");
    assert_eq!(
        run(r#"<xsl:for-each-group select="r/i" collation="unsupported"/>"#)
            .unwrap_err()
            .code,
        "XTSE1080"
    );
    assert_eq!(
        run(
            r#"<xsl:for-each-group select="r/i" group-starting-with="i" collation="unsupported"/>"#
        )
        .unwrap_err()
        .code,
        "XTSE1090"
    );
}

#[test]
fn modern_pattern_grouping_accepts_atomics_in_older_stylesheets() {
    let stylesheet = r#"<xsl:stylesheet version="2.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:for-each-group select="(1,2)" group-starting-with="/"><xsl:value-of select="count(current-group())"/></xsl:for-each-group></xsl:template></xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(stylesheet)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "2"
    );
    assert_eq!(
        Stylesheet::compile(&stylesheet.replace("version=\"2.0\"", "version=\"3.0\""))
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "2"
    );
}

#[test]
fn grouping_keys_atomize_arrays_and_reject_maps() {
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2,3)" group-by="[. mod 2,9]" composite="yes"><xsl:value-of select="count(current-group())"/></xsl:for-each-group>"#).unwrap(), "21");
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2)" group-adjacent="[1]"><xsl:value-of select="count(current-group())"/></xsl:for-each-group>"#).unwrap(), "2");
    assert_eq!(
        run(r#"<xsl:for-each-group select="(1,2)" group-by="map{'key':.}"/>"#)
            .unwrap_err()
            .code,
        "FOTY0013"
    );
}

#[test]
fn untyped_grouping_keys_are_exposed_as_strings_and_composites_retain_nan() {
    assert_eq!(run(r#"<xsl:for-each-group select="r/i" group-by="@k"><xsl:value-of select="current-grouping-key() instance of xs:string"/><xsl:text>;</xsl:text></xsl:for-each-group>"#).unwrap(), "true;true;true;true;true;");
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2,3)" group-by="(xs:double('NaN'), . mod 2)" composite="yes"><xsl:value-of select="count(current-grouping-key())"/><xsl:text>:</xsl:text><xsl:value-of select="count(current-group())"/><xsl:text>;</xsl:text></xsl:for-each-group>"#).unwrap(), "2:2;2:1;");
}

#[test]
fn composite_is_rejected_on_pattern_grouping_even_when_false() {
    for attribute in ["group-starting-with", "group-ending-with"] {
        for composite in ["yes", "no", "true", "false", "1", "0"] {
            assert_eq!(
                run(&format!(
                    r#"<xsl:for-each-group select="r/i" {attribute}="i" composite="{composite}"/>"#
                ))
                .unwrap_err()
                .code,
                "XTSE1090"
            );
        }
    }
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2,1)" group-by="." composite="no"><xsl:value-of select="count(current-group())"/></xsl:for-each-group>"#).unwrap(), "21");
    assert_eq!(run(r#"<xsl:for-each-group select="(1,1,2)" group-adjacent="." composite="false"><xsl:value-of select="count(current-group())"/></xsl:for-each-group>"#).unwrap(), "21");
}

#[test]
fn function_values_do_not_capture_or_inherit_grouping_context() {
    for (function, code) in [
        ("current-group", "XTDE1061"),
        ("current-grouping-key", "XTDE1071"),
    ] {
        for expression in [
            format!("let $f := {function}#0 return $f()"),
            format!("function() {{ {function}() }}()"),
            format!("for-each((1), function($n) {{ {function}() }})"),
        ] {
            assert_eq!(run(&format!(r#"<xsl:for-each-group select="(1,2,1)" group-by="."><xsl:sequence select="{expression}"/></xsl:for-each-group>"#)).unwrap_err().code, code, "{expression}");
        }
    }
    // Values explicitly saved in lexical variables remain available to closures.
    assert_eq!(run(r#"<xsl:for-each-group select="(1,2,1)" group-by="."><xsl:variable name="g" select="current-group()"/><xsl:variable name="k" select="current-grouping-key()"/><xsl:sequence select="function() { count($g) + $k }()"/></xsl:for-each-group>"#).unwrap(), "3 3");
}

#[test]
fn typed_group_sequences_in_tunnel_parameters_preserve_source_whitespace() {
    let stylesheet = r#"<xsl:stylesheet version="2.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><out><xsl:apply-templates/></out></xsl:template><xsl:template match="c"><xsl:variable name="t" as="element()*"><tt n="T"/><tt n="U"/><tt n="U"/></xsl:variable><xsl:variable name="o" as="element()*"><xsl:for-each-group select="$t" group-by="@n"><xsl:variable name="g" as="element()+" select="current-group()"/><xsl:sequence select="$g[@n='U']"/></xsl:for-each-group></xsl:variable><xsl:apply-templates><xsl:with-param name="o" tunnel="yes" select="$o"/></xsl:apply-templates></xsl:template><xsl:template match="t"><xsl:param name="o" tunnel="yes" as="element()*"/><xsl:copy><xsl:copy-of select="@*"/><xsl:copy-of select="$o"/></xsl:copy></xsl:template></xsl:stylesheet>"#;
    let result = Stylesheet::compile(stylesheet)
        .unwrap()
        .transform("<c>\n <t n='T'/>\n <t n='U'/>\n</c>")
        .unwrap();
    assert_eq!(
        result,
        "<out>\n <t n=\"T\"><tt n=\"U\"></tt><tt n=\"U\"></tt></t>\n <t n=\"U\"><tt n=\"U\"></tt><tt n=\"U\"></tt></t>\n</out>"
    );
}

#[test]
fn obsolete_group_binding_attributes_are_static_errors() {
    for attribute in ["bind-group", "bind-grouping-key"] {
        assert_eq!(
            run(&format!(
                r#"<xsl:for-each-group select="r/i" group-by="@k" {attribute}="g"/>"#
            ))
            .unwrap_err()
            .code,
            "XTSE0090"
        );
    }
}
