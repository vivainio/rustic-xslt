use rust_xslt::Stylesheet;
const DOC: &str = "<doc><chap id='c1'><p>a</p><sec><p>b</p><p>c</p></sec></chap><chap id='c2'><q>d</q><p>e</p></chap></doc>";
fn run(templates: &str) -> Result<String, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:key name="k" match="chap" use="@id"/>{templates}<xsl:template match="text()"/><xsl:template match="*" priority="-10"/></xsl:stylesheet>"#))?
        .transform(DOC)
}
#[test]
fn descendant_separators_match_any_depth() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="chap//p">[<xsl:value-of select="."/>]</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "[a][b][c][e]");
    let rooted = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="//sec/p">S<xsl:value-of select="."/></xsl:template><xsl:template match="/doc//q">Q</xsl:template>"#;
    assert_eq!(run(rooted).unwrap(), "SbScQ");
    let chain = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="doc//sec//p">D</xsl:template>"#;
    assert_eq!(run(chain).unwrap(), "DD");
}

#[test]
fn simple_boolean_pattern_predicates_do_not_scan_all_siblings() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="r/*"/></xsl:template><xsl:template match="*"/><xsl:template match="*[. = 'hit']">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let mut document = String::from("<r>");
    for i in 0..200 {
        document.push_str(if i == 123 {
            "<n>hit</n>"
        } else {
            "<n>miss</n>"
        });
    }
    document.push_str("</r>");
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 30_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&document, &options).unwrap(),
        "X"
    );
    let positional = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="r/*"/></xsl:template><xsl:template match="*"/><xsl:template match="*[position() = last()]">L</xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(positional.transform("<r><n/><n/><n/></r>").unwrap(), "L");
}

#[test]
fn explicit_descendant_axes_match_any_depth_in_patterns() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="doc"><xsl:apply-templates/></xsl:template><xsl:template match="doc/descendant::foo">D</xsl:template><xsl:template match="doc/descendant-or-self::bar">S</xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(
        sheet
            .transform("<doc><foo/><x><foo/><bar/></x></doc>")
            .unwrap(),
        "DDS"
    );
}

#[test]
fn parenthesized_node_pattern_filters_use_the_selected_sequence() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="*"/><xsl:template match="(doc/descendant::foo)[2]"><xsl:value-of select="@id"/></xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(
        sheet
            .transform("<doc><foo id='a'/><x><foo id='b'/></x><foo id='c'/></doc>")
            .unwrap(),
        "b"
    );
}

#[test]
fn attribute_kind_tests_use_the_implicit_attribute_axis() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:value-of select="count(doc/attribute())"/><xsl:apply-templates select="//@*"/></xsl:template><xsl:template match="document-node()/doc/attribute()[1]">:<xsl:value-of select="name()"/></xsl:template><xsl:template match="child::attribute()" priority="10">wrong</xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(sheet.transform("<doc a='1' b='2'/>").unwrap(), "2:a2");
}

#[test]
fn explicit_descendant_or_self_node_is_not_a_path_separator() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="doc/descendant-or-self::node()">X</xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(sheet.transform("<doc><a><b/></a></doc>").unwrap(), "XXX");
}

#[test]
fn global_variable_anchors_select_pattern_candidates() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="roots" select="/doc"/><xsl:variable name="selected" select="/doc/foo"/><xsl:template match="/"><xsl:apply-templates/></xsl:template><xsl:template match="*"><xsl:apply-templates/></xsl:template><xsl:template match="$roots//baz">B</xsl:template><xsl:template match="$selected[@kind = 'hit']">H</xsl:template><xsl:template match="text()"/></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(
        sheet
            .transform("<doc><foo kind='hit'><baz/><baz/></foo><foo><baz/></foo></doc>")
            .unwrap(),
        "HB"
    );
}

#[test]
fn intersect_and_except_patterns_constrain_candidate_nodes() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="doc"><xsl:apply-templates/></xsl:template><xsl:template match="foo except /*/*">E</xsl:template><xsl:template match="foo intersect /*/*">I</xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(
        sheet.transform("<doc><foo/><x><foo/></x></doc>").unwrap(),
        "IE"
    );
}

#[test]
fn set_pattern_operands_share_the_same_context_node() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="//leaf"/></xsl:template><xsl:template match="leaf">N</xsl:template><xsl:template match="div[@id='a']//* intersect div[@id='b']//*" priority="2">I</xsl:template><xsl:template match="div[@id='a']//* except div[@id='b']//*" priority="1">E</xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(
        sheet
            .transform("<div id='a'><div id='b'><leaf/></div></div>")
            .unwrap(),
        "E"
    );
}

#[test]
fn root_function_patterns_can_filter_a_parentless_element() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="data" as="element(A)"><A/></xsl:variable><xsl:template name="main"><xsl:apply-templates select="$data"/></xsl:template><xsl:template match="root()[self::A]">R</xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(
        sheet
            .transform_with_options(
                "<source/>",
                &rust_xslt::TransformOptions {
                    initial_template: Some("main".into()),
                    ..Default::default()
                },
            )
            .unwrap(),
        "R"
    );
}

#[test]
fn path_patterns_keep_except_constraints_anchored_to_their_prefix() {
    let sheet = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:mode on-no-match="shallow-skip"/><xsl:variable name="nodes" as="element()*"><x id="1"><a>23</a><x><a>25</a></x></x></xsl:variable><xsl:template name="main"><xsl:apply-templates select="$nodes"/></xsl:template><xsl:template match="x[@id='1']/(descendant::a except child::a)">I</xsl:template><xsl:template match="a">A</xsl:template></xsl:stylesheet>"#,
    ).unwrap();
    assert_eq!(
        sheet
            .transform_with_options(
                "",
                &rust_xslt::TransformOptions {
                    initial_template: Some("main".into()),
                    ..Default::default()
                },
            )
            .unwrap(),
        "AI"
    );
}

#[test]
fn match_pattern_syntax_respects_versioned_extensions() {
    for pattern in ["(a|b)", "and union or", "/[doc]"] {
        let source = format!(
            r#"<xsl:stylesheet version="2.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="{pattern}"/></xsl:stylesheet>"#
        );
        assert_eq!(Stylesheet::compile(&source).unwrap_err().code, "XTSE0340");
    }
    let v3 = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="(a|b)"/></xsl:stylesheet>"#;
    assert!(Stylesheet::compile(v3).is_ok());
}

#[test]
fn pattern_function_arguments_are_literals_or_variables() {
    let stylesheet = |pattern: &str| {
        format!(
            r#"<xsl:stylesheet version="2.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:variable name="v" select="42"/><xsl:key name="k" match="*" use="42"/><xsl:template match="{pattern}"/></xsl:stylesheet>"#
        )
    };
    for pattern in ["key('k', 40+2)//a", "id(concat('a', 'b'))", "root(.)"] {
        assert_eq!(
            Stylesheet::compile(&stylesheet(pattern)).unwrap_err().code,
            "XTSE0340",
            "{pattern}"
        );
    }
    for pattern in ["key('k', $v)//a", "id($v)/a", "a[key('k', 40+2)]", "$v"] {
        assert!(
            Stylesheet::compile(&stylesheet(pattern)).is_ok(),
            "{pattern}"
        );
    }
}

#[test]
fn parenthesized_unions_expand_into_alternatives() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="chap/(p|q)">X</xsl:template><xsl:template match="(sec|doc)/p">Y</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "XYYXX");
    let nested = r#"<xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="(chap|sec)//p">Z</xsl:template>"#;
    assert_eq!(run(nested).unwrap(), "ZZZZ");
}
#[test]
fn key_and_id_patterns_match_selected_nodes() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//chap"/></xsl:template><xsl:template match="key('k', 'c2')">second</xsl:template><xsl:template match="chap">other</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "othersecond");
}
#[test]
fn default_priorities_account_for_path_shape() {
    // The compound pattern (priority 0.5) beats the plain name test (priority 0).
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//p"/></xsl:template><xsl:template match="p">plain </xsl:template><xsl:template match="sec//p">deep </xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "plain deep deep plain ");
    assert!(run(r#"<xsl:template match="a/following::b"/>"#).is_err());
    assert!(run(r#"<xsl:template match="a//"/>"#).is_err());
}

#[test]
fn key_anchored_paths_match_children_descendants_and_attributes() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//p | //chap/@id"/></xsl:template>
      <xsl:template match="key('k', 'c1')/p">C</xsl:template>
      <xsl:template match="key('k', 'c1')//p" priority="0">D</xsl:template>
      <xsl:template match="key('k', 'c2')/@id">A</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "c1CDDA");
}

#[test]
fn selection_anchors_survive_parenthesized_path_unions() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//p | //q"/></xsl:template>
      <xsl:template match="key('k', 'c1')/(p | sec/p)">X</xsl:template>
      <xsl:template match="(key('k', 'c1') | key('k', 'c2'))/q">Q</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "XXXQ");
}

#[test]
fn descendant_paths_accept_parenthesized_pattern_steps() {
    let templates = r#"<xsl:template match="/"><xsl:apply-templates select="//p | //q"/></xsl:template><xsl:template match="key('k', 'c1')//(p | q)">X</xsl:template><xsl:template match="key('k', 'c2')//(p | q)">Y</xsl:template>"#;
    assert_eq!(run(templates).unwrap(), "XXXYY");
    let rooted = r#"<xsl:template match="/"><xsl:apply-templates select="//p | //q"/></xsl:template><xsl:template match="//(p | q)">Z</xsl:template>"#;
    assert_eq!(run(rooted).unwrap(), "ZZZZZ");
    let chained = r#"<xsl:template match="/"><xsl:apply-templates select="//p"/></xsl:template><xsl:template match="key('k', 'c1')//(sec | p)/p">S</xsl:template><xsl:template match="p">N</xsl:template>"#;
    assert_eq!(run(chained).unwrap(), "NSSN");
}

#[test]
fn id_anchored_paths_require_identity_and_the_correct_distance() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
      <xsl:output method="text"/>
      <xsl:template match="/"><xsl:apply-templates select="//p"/></xsl:template>
      <xsl:template match="p">N</xsl:template>
      <xsl:template match="id('anchor')/p">C</xsl:template>
      <xsl:template match="id('anchor')//p" priority="0.25">D</xsl:template>
    </xsl:stylesheet>"#;
    assert_eq!(
        Stylesheet::compile(sheet)
            .unwrap()
            .transform("<r><s xml:id='anchor'><p/><s><p/></s></s><s><p/></s></r>")
            .unwrap(),
        "CDN"
    );
}

#[test]
fn grouping_functions_are_static_errors_in_all_pattern_positions() {
    for (function, code) in [
        ("current-group", "XTSE1060"),
        ("current-grouping-key", "XTSE1070"),
    ] {
        for predicate in [
            format!("exists({function}())"),
            format!("if (true()) then true() else exists({function}())"),
            format!("map {{'key': {function}()}}?key"),
            format!("[{function}()]?1"),
            format!("{function}#0"),
        ] {
            let pattern = format!("p[{predicate}]");
            for instruction in [
                format!(r#"<xsl:template match="{pattern}"/>"#),
                format!(
                    r#"<xsl:template match="/"><xsl:number count="{pattern}"/></xsl:template>"#
                ),
                format!(
                    r#"<xsl:template match="/"><xsl:for-each-group select="doc/*" group-starting-with="{pattern}"/></xsl:template>"#
                ),
            ] {
                assert_eq!(run(&instruction).unwrap_err().code, code, "{instruction}");
            }
        }
    }
}

#[test]
fn grouping_function_text_in_pattern_literals_and_comments_is_harmless() {
    assert!(run(r#"<xsl:template match="p[. = 'current-group()']"/>"#).is_ok());
    assert!(run(r#"<xsl:template match="p[true() (: current-grouping-key() :)]"/>"#).is_ok());
    assert_eq!(run(r#"<xsl:template xmlns:fn="http://www.w3.org/2005/xpath-functions" match="p[exists(fn:current-group())]"/>"#).unwrap_err().code, "XTSE1060");
}

#[test]
fn root_anchored_union_pattern_avoids_repeating_the_same_document_selection() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="enabled" select="false()"/><xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="*"/><xsl:template match="(/doc/a | /doc/b)[$enabled]">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let document = format!("<doc>{}<a/></doc>", "<n/>".repeat(80));
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 100_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&document, &options).unwrap(),
        ""
    );
}

#[test]
fn root_anchored_union_with_pure_step_predicates_stays_within_work_budget() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="enabled" select="false()"/><xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="*"/><xsl:template match="(/doc/a[@kind='x'] | /doc/b[normalize-space(@id)='b'])[$enabled]">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let document = format!("<doc>{}<a kind='x'/></doc>", "<n/>".repeat(80));
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 100_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&document, &options).unwrap(),
        ""
    );
}

#[test]
fn parenthesized_patterns_do_not_rescan_the_document_for_every_candidate() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="*"/><xsl:template match="(pattern | group)[@abstract='true']">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let document = format!(
        "<doc>{}<pattern abstract='true'/></doc>",
        "<pattern/>".repeat(200)
    );
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 200_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&document, &options).unwrap(),
        "X"
    );
}

#[test]
fn parenthesized_root_patterns_with_boolean_variables_fit_the_work_budget() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="enabled" select="false()"/><xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="*"/><xsl:template match="(/doc/a | /doc/b)[$enabled]">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let document = format!("<doc>{}<a/></doc>", "<n/>".repeat(400));
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 300_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&document, &options).unwrap(),
        ""
    );
}

#[test]
fn cached_pattern_with_many_matches_fits_the_work_budget() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="enabled" select="true()"/><xsl:template match="/"><xsl:apply-templates select="/doc/*"/></xsl:template><xsl:template match="*">N</xsl:template><xsl:template match="(/doc/a | /doc/b)[$enabled]">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let document = format!("<doc>{}</doc>", "<a/><b/><n/>".repeat(200));
    let options = rust_xslt::TransformOptions {
        limits: rust_xslt::Limits {
            max_steps: 200_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options(&document, &options).unwrap(),
        "XXN".repeat(200)
    );
}

#[test]
fn parenthesized_pattern_cache_tracks_boolean_variable_values() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:call-template name="group"><xsl:with-param name="enabled" select="true()"/></xsl:call-template><xsl:call-template name="group"><xsl:with-param name="enabled" select="false()"/></xsl:call-template></xsl:template><xsl:template name="group"><xsl:param name="enabled"/><xsl:for-each-group select="/doc/n" group-starting-with="(/doc/n | /doc/x)[$enabled]"><xsl:text>X</xsl:text></xsl:for-each-group><xsl:text>|</xsl:text></xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(
        sheet.transform("<doc><n/><n/><n/></doc>").unwrap(),
        "XXX|X|"
    );
}

#[test]
fn parenthesized_pattern_cache_keeps_different_branches_separate() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:variable name="enabled" select="true()"/><xsl:template match="/"><xsl:apply-templates select="/doc/*"/></xsl:template><xsl:template match="*">N</xsl:template><xsl:template match="(/doc/a | /doc/c)[$enabled]">A</xsl:template><xsl:template match="(/doc/b | /doc/d)[$enabled]">B</xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(
        sheet.transform("<doc><a/><b/><c/><d/></doc>").unwrap(),
        "ABAB"
    );
}

#[test]
fn parenthesized_pattern_keeps_numeric_variable_predicates_positional() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:call-template name="group"><xsl:with-param name="which" select="1"/></xsl:call-template><xsl:call-template name="group"><xsl:with-param name="which" select="2"/></xsl:call-template></xsl:template><xsl:template name="group"><xsl:param name="which"/><xsl:for-each-group select="/doc/n" group-starting-with="(/doc/n | /doc/x)[$which]"><xsl:text>X</xsl:text></xsl:for-each-group><xsl:text>|</xsl:text></xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(sheet.transform("<doc><n/><n/><n/></doc>").unwrap(), "X|XX|");
}

#[test]
fn parenthesized_pattern_cache_keeps_documents_distinct() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:apply-templates select="doc('a')//* | doc('b')//*"/></xsl:template><xsl:template match="*"/><xsl:template match="(pattern | group)[@abstract='true']">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let options = rust_xslt::TransformOptions {
        resolver: Some(rust_xslt::ResourceResolver::new(|uri| match uri {
            "a" => Ok("<doc><pattern abstract='true'/></doc>".into()),
            "b" => Ok("<doc><group abstract='true'/></doc>".into()),
            _ => panic!("unexpected URI: {uri}"),
        })),
        ..Default::default()
    };
    assert_eq!(
        sheet.transform_with_options("<input/>", &options).unwrap(),
        "XX"
    );
}

#[test]
fn parenthesized_pattern_cache_does_not_skip_effectful_predicates() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f" xmlns:xs="http://www.w3.org/2001/XMLSchema"><xsl:output method="text"/><xsl:function name="f:check" as="xs:boolean"><xsl:message>called</xsl:message><xsl:sequence select="true()"/></xsl:function><xsl:template match="/"><xsl:apply-templates select="//*"/></xsl:template><xsl:template match="*"/><xsl:template match="(pattern | group)[f:check()]">X</xsl:template></xsl:stylesheet>"#).unwrap();
    let result = sheet
        .transform_with_documents("<doc><pattern/><group/></doc>", &Default::default())
        .unwrap();
    assert_eq!(result.principal, "XX");
    assert_eq!(result.messages.len(), 8);
}
