use rust_xslt::{Stylesheet, TransformOptions};
fn run(declarations: &str, templates: &str, body: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema"><xsl:output method="text"/>{declarations}<xsl:template name="entry">{body}</xsl:template>{templates}</xsl:stylesheet>"#))?
        .transform_with_options("<r/>", &TransformOptions { initial_template: Some("entry".into()), ..Default::default() })
}

#[test]
fn predicate_patterns_match_atomic_values_and_bare_dot_is_the_low_priority_fallback() {
    let templates = r#"<xsl:template match=".">F</xsl:template><xsl:template match="*">N</xsl:template><xsl:template match=".[. instance of xs:integer]">[<xsl:value-of select="."/>]</xsl:template>"#;
    assert_eq!(
        run(
            "",
            templates,
            r#"<xsl:apply-templates select="(1, 'a', r)"/>"#
        )
        .unwrap(),
        "[1]FN"
    );
}

#[test]
fn predicate_focus_is_singleton_but_template_focus_is_the_input_sequence() {
    let templates = r#"<xsl:template match=".[position() = 1][last() = 1][1]"><xsl:value-of select="concat(position(), '/', last(), ';')"/></xsl:template><xsl:template match=".[2]" priority="2">wrong</xsl:template>"#;
    assert_eq!(
        run("", templates, r#"<xsl:apply-templates select="1 to 3"/>"#).unwrap(),
        "1/3;2/3;3/3;"
    );
}

#[test]
fn default_predicate_priority_and_next_match_work_for_atomic_items() {
    let templates = r#"<xsl:template match=".[. instance of xs:integer]" priority="1.1">A<xsl:next-match/></xsl:template><xsl:template match=".[. gt 0]">B<xsl:next-match/></xsl:template><xsl:template match=".[. instance of xs:integer]" priority="0.9">C<xsl:next-match/></xsl:template><xsl:template match=".">D</xsl:template>"#;
    assert_eq!(
        run("", templates, r#"<xsl:apply-templates select="17"/>"#).unwrap(),
        "ABCD"
    );
}

#[test]
fn predicate_patterns_match_maps_arrays_and_function_items() {
    let templates = r#"<xsl:template match=".[. instance of function(*)]">F</xsl:template><xsl:template match=".[. instance of map(*)]">M</xsl:template><xsl:template match=".[. instance of array(*)]">A</xsl:template>"#;
    assert_eq!(
        run(
            "",
            templates,
            r#"<xsl:apply-templates select="(map{'k':1}, [2,3], function(){4})"/>"#
        )
        .unwrap(),
        "MAF"
    );
}

#[test]
fn built_in_policies_handle_atomic_and_function_items() {
    assert_eq!(
        run(
            "",
            "",
            r#"<xsl:apply-templates select="(1, [2,[3]], map{}, function(){4})"/>"#
        )
        .unwrap(),
        "123"
    );
    for policy in ["deep-copy", "shallow-copy"] {
        assert_eq!(
            run(
                &format!(r#"<xsl:mode on-no-match="{policy}"/>"#),
                "",
                r#"<xsl:apply-templates select="[1,2]"/>"#
            )
            .unwrap(),
            "1 2"
        );
        assert_eq!(
            run(
                &format!(r#"<xsl:mode on-no-match="{policy}"/>"#),
                "",
                r#"<xsl:apply-templates select="map{}"/>"#
            )
            .unwrap_err()
            .code,
            "SENR0001"
        );
    }
    assert_eq!(
        run(
            r#"<xsl:mode on-no-match="deep-skip"/>"#,
            "",
            r#"<xsl:apply-templates select="[1,2]"/>"#
        )
        .unwrap(),
        ""
    );
    assert_eq!(
        run(
            r#"<xsl:mode on-no-match="fail"/>"#,
            "",
            r#"<xsl:apply-templates select="1"/>"#
        )
        .unwrap_err()
        .code,
        "XTDE0555"
    );
    let templates = r#"<xsl:template match=".[. instance of xs:integer]"><xsl:value-of select="."/></xsl:template>"#;
    assert_eq!(
        run(
            r#"<xsl:mode on-no-match="shallow-skip"/>"#,
            templates,
            r#"<xsl:apply-templates select="[1,[2]]"/>"#
        )
        .unwrap(),
        "12"
    );
}

#[test]
fn array_builtin_dispatch_forwards_parameters_to_member_templates() {
    let templates = r#"<xsl:template match=".[. instance of xs:integer]"><xsl:param name="tag"/><xsl:value-of select="concat($tag, .)"/></xsl:template>"#;
    assert_eq!(run("", templates, r#"<xsl:apply-templates select="[1,2]"><xsl:with-param name="tag" select="'X'"/></xsl:apply-templates>"#).unwrap(), "X1X2");
}

#[test]
fn copied_function_items_remain_raw_when_captured_in_typed_variables() {
    let body = r#"<xsl:variable name="m" as="map(*)"><xsl:apply-templates select="map{'k': 7}"/></xsl:variable><xsl:value-of select="$m?k"/>"#;
    assert_eq!(
        run(r#"<xsl:mode on-no-match="deep-copy"/>"#, "", body).unwrap(),
        "7"
    );
}

#[test]
fn grouping_patterns_work_with_atomic_populations() {
    for (attribute, expected) in [
        ("group-starting-with", "1,2|3,4,5|6,7|"),
        ("group-ending-with", "1,2,3|4,5,6|7|"),
    ] {
        let body = format!(
            r#"<xsl:for-each-group select="1 to 7" {attribute}=".[. mod 3 = 0]"><xsl:value-of select="current-group()" separator=","/>|</xsl:for-each-group>"#
        );
        assert_eq!(run("", "", &body).unwrap(), expected);
    }
}

#[test]
fn predicate_variables_are_checked_statically() {
    assert_eq!(
        run("", r#"<xsl:template match=".[false() and $missing]"/>"#, "")
            .unwrap_err()
            .code,
        "XPST0008"
    );
    assert_eq!(
        run(
            "",
            "",
            r#"<xsl:for-each-group select="()" group-starting-with=".[$missing]"/>"#
        )
        .unwrap_err()
        .code,
        "XPST0008"
    );
}
