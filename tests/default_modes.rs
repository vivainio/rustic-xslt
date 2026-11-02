use rust_xslt::{Stylesheet, TransformOptions};

#[test]
fn dispatch_skips_other_modes_and_keeps_all_mode_next_match() {
    let unrelated: String = (1..=120)
        .map(|i| format!(r#"<xsl:template match="n" mode="m{i}">wrong</xsl:template>"#))
        .collect();
    let sheet = Stylesheet::compile(&format!(r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="r/n" mode="m0"/></xsl:template><xsl:template match="n" mode="#all" priority="0">X</xsl:template><xsl:template match="n" mode="m0" priority="1">M<xsl:next-match/></xsl:template>{unrelated}</xsl:stylesheet>"##)).unwrap();
    let input = format!("<r>{}</r>", "<n/>".repeat(200));
    let options = TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 20_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&input, &options).unwrap(),
        "MX".repeat(200)
    );
}

#[test]
fn mode_dispatch_works_when_the_cache_has_no_space() {
    let sheet = Stylesheet::compile(r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="r/n" mode="m"/></xsl:template><xsl:template match="n" mode="#all">X</xsl:template></xsl:stylesheet>"##).unwrap();
    let options = TransformOptions {
        limits: rust_xslt::Limits {
            max_temporary_bytes: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet
            .transform_with_options("<r><n/><n/></r>", &options)
            .unwrap(),
        "XX"
    );
}
fn source(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:p="urn:p" xmlns:q="urn:p" default-mode="p:m"><xsl:output method="text"/>{body}</xsl:stylesheet>"##
    )
}
#[test]
fn default_mode_controls_initial_and_implicit_dispatch() {
    let s = Stylesheet::compile(&source(r##"<xsl:template match="/"><xsl:apply-templates/></xsl:template><xsl:template match="r">default</xsl:template><xsl:template match="r" mode="#unnamed">unnamed</xsl:template>"##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "default");
    assert_eq!(
        s.transform_with_options(
            "<r/>",
            &TransformOptions {
                initial_mode: Some("#unnamed".into()),
                ..Default::default()
            }
        )
        .unwrap(),
        "unnamed"
    );
}
#[test]
fn inherited_prefixes_resolve_at_the_default_mode_declaration() {
    let s = Stylesheet::compile(&source(r##"<xsl:template match="/" xmlns:p="urn:other"><xsl:apply-templates mode="#default"/></xsl:template><xsl:template match="r" mode="q:m">outer</xsl:template><xsl:template match="r" mode="p:m" xmlns:p="urn:other">wrong</xsl:template>"##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "outer");
}
#[test]
fn local_default_modes_apply_to_template_and_literal_scopes() {
    let s = Stylesheet::compile(&source(r##"<xsl:template match="/" default-mode="#unnamed" mode="p:m"><wrapper xsl:default-mode="Q{urn:inner}m"><xsl:apply-templates/></wrapper><xsl:apply-templates mode="#default"/></xsl:template><xsl:template match="r" mode="Q{urn:inner}m">inner</xsl:template><xsl:template match="r" mode="#unnamed">unnamed</xsl:template>"##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "innerunnamed");
}
#[test]
fn invalid_default_modes_and_duplicate_aliases_are_static_errors() {
    for (body, expected) in [
        (
            r##"<xsl:template match="/" default-mode="z:m"/>"##,
            "XTSE0280",
        ),
        (
            r##"<xsl:template match="/" default-mode="#default"/>"##,
            "XTSE0020",
        ),
        (r##"<xsl:template match="/" mode="p:m q:m"/>"##, "XTSE0550"),
        (r##"<xsl:template match="/" mode="#all p:m"/>"##, "XTSE0550"),
        (r##"<xsl:template match="/" mode=" "/>"##, "XTSE0550"),
    ] {
        assert_eq!(
            Stylesheet::compile(&source(body)).unwrap_err().code,
            expected
        );
    }
}
#[test]
fn current_mode_is_distinct_from_a_locally_changed_default() {
    let s = Stylesheet::compile(&source(r##"<xsl:template match="/"><xsl:apply-templates default-mode="#unnamed" mode="#current"/></xsl:template><xsl:template match="r">current</xsl:template><xsl:template match="r" mode="#unnamed">wrong</xsl:template>"##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "current");
}

#[test]
fn all_mode_templates_do_not_declare_arbitrary_initial_modes() {
    let s = Stylesheet::compile(&source(
        r##"<xsl:template match="/" mode="#all">wrong</xsl:template>"##,
    ))
    .unwrap();
    let opts = TransformOptions {
        initial_mode: Some("Q{urn:unknown}m".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap_err().code,
        "XTDE0045"
    );
}
#[test]
fn explicit_and_implicit_named_modes_are_eligible_by_expanded_name() {
    let s = Stylesheet::compile(&source(r##"<xsl:mode name="p:explicit"/><xsl:template match="/" mode="q:implicit">implicit</xsl:template>"##)).unwrap();
    for name in ["Q{urn:p}explicit", "Q{urn:p}implicit"] {
        let opts = TransformOptions {
            initial_mode: Some(name.into()),
            ..Default::default()
        };
        assert!(s.transform_with_options("<r/>", &opts).is_ok());
    }
}
#[test]
fn named_private_modes_are_internal_but_default_modes_remain_eligible() {
    let s = Stylesheet::compile(&source(r##"<xsl:mode name="p:private" visibility="private"/><xsl:mode name="p:m" visibility="private"/><xsl:template match="/"><xsl:apply-templates select="r" mode="q:private"/></xsl:template><xsl:template match="r" mode="q:private">internal</xsl:template>"##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "internal");
    let opts = TransformOptions {
        initial_mode: Some("Q{urn:p}private".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap_err().code,
        "XTDE0045"
    );
    let opts = TransformOptions {
        initial_mode: Some("Q{urn:p}m".into()),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), "internal");
}
#[test]
fn mode_visibility_conflicts_and_invalid_unnamed_visibility_are_static() {
    assert_eq!(Stylesheet::compile(&source(r##"<xsl:mode name="p:m" visibility="public"/><xsl:mode name="q:m" visibility="private"/>"##)).unwrap_err().code, "XTSE0545");
    assert_eq!(
        Stylesheet::compile(&source(r##"<xsl:mode visibility="public"/>"##))
            .unwrap_err()
            .code,
        "XTSE0020"
    );
}
