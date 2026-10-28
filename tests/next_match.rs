use rust_xslt::{StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn sheet(body: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/>{body}</xsl:stylesheet>"#)).unwrap()
}
#[test]
fn next_match_follows_priority_then_reverse_declaration_order() {
    let s = sheet(
        r#"<xsl:template match="r" priority="0">first</xsl:template><xsl:template match="r" priority="0">second:<xsl:next-match/></xsl:template><xsl:template match="r" priority="1">high:<xsl:next-match/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "high:second:first");
}
#[test]
fn union_branches_are_separate_rules_unless_priority_is_explicit() {
    for (priority, expected) in [("", "branch:branch:end"), (" priority='1'", "branch:end")] {
        let s = sheet(&format!(
            r#"<xsl:template match="r" priority="-1">end</xsl:template><xsl:template match="r|r"{priority}>branch:<xsl:next-match/></xsl:template>"#
        ));
        assert_eq!(s.transform("<r/>").unwrap(), expected);
    }
}
#[test]
fn union_branches_do_not_trigger_multiple_match_fail() {
    let s = sheet(
        r#"<xsl:mode on-multiple-match="fail"/><xsl:template match="r" priority="-1">end</xsl:template><xsl:template match="r|r">branch:<xsl:next-match/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "branch:branch:end");
}
#[test]
fn named_templates_preserve_the_callers_current_rule() {
    let s = sheet(
        r#"<xsl:template match="r" priority="0">end</xsl:template><xsl:template match="r" priority="1"><xsl:call-template name="helper"/></xsl:template><xsl:template name="helper">helper:<xsl:next-match/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap(), "helper:end");
}
#[test]
fn next_match_keeps_current_focus_and_mode_with_fresh_ordinary_arguments() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:apply-templates select="r/n" mode="m"><xsl:with-param name="p" select="'old'"/></xsl:apply-templates></xsl:template><xsl:template match="n" mode="m" priority="1"><xsl:param name="p"/><xsl:next-match><xsl:with-param name="q" select="concat($p, ':', position(), '/', last())"/><xsl:with-param name="t" tunnel="yes" select="'tunnel'"/></xsl:next-match></xsl:template><xsl:template match="n" mode="m"><xsl:param name="p" select="'default'"/><xsl:param name="q"/><xsl:param name="t" tunnel="yes"/><xsl:value-of select="concat($p, ':', $q, ':', $t, ';')"/></xsl:template>"#,
    );
    assert_eq!(
        s.transform("<r><n/><n/></r>").unwrap(),
        "default:old:1/2:tunnel;default:old:2/2:tunnel;"
    );
}
#[test]
fn next_match_without_a_current_rule_reports_a_dynamic_error() {
    let s = sheet(r#"<xsl:template name="entry"><xsl:next-match/></xsl:template>"#);
    assert_eq!(
        s.transform_with_options(
            "",
            &TransformOptions {
                initial_template: Some("entry".into()),
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "XTDE0560"
    );
    for body in [
        r#"<xsl:for-each select="."><xsl:next-match/></xsl:for-each>"#,
        r#"<xsl:copy select="."><xsl:next-match/></xsl:copy>"#,
    ] {
        let s = sheet(&format!(r#"<xsl:template match="r">{body}</xsl:template>"#));
        assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0560");
    }
}
#[test]
fn final_next_match_uses_the_modes_builtin_rule() {
    let s = sheet(
        r#"<xsl:mode on-no-match="text-only-copy"/><xsl:template match="r">prefix:<xsl:next-match/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r>text</r>").unwrap(), "prefix:text");
    let s = sheet(
        r#"<xsl:mode on-no-match="fail"/><xsl:template match="/"><xsl:next-match/></xsl:template>"#,
    );
    assert_eq!(s.transform("<r/>").unwrap_err().code, "XTDE0555");
}
#[test]
fn grounded_streams_can_continue_to_lower_priority_rules() {
    let s = sheet(
        r#"<xsl:template name="record"><xsl:apply-templates select="."/></xsl:template><xsl:template match="record" priority="1">high:<xsl:next-match/></xsl:template><xsl:template match="record"><xsl:value-of select="."/></xsl:template>"#,
    );
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<r><record>one</record><record>two</record></r>"),
        &mut output,
        &StreamOptions {
            record_path: "/r/record".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output, b"high:onehigh:two");
}

#[test]
fn long_next_match_chains_use_the_task_stack_and_shared_budget() {
    let mut body = String::new();
    for priority in 0..600 {
        body.push_str(&format!(
            r#"<xsl:template match="r" priority="{priority}">x<xsl:next-match/></xsl:template>"#
        ));
    }
    let s = sheet(&body);
    let mut opts = TransformOptions {
        limits: rust_xslt::Limits {
            max_depth: 1000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap(),
        "x".repeat(600)
    );
    opts.limits.max_steps = 100;
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap_err().code,
        "XPDY0130"
    );
}
