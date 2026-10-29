use rust_xslt::{StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn sheet(body: &str) -> String {
    format!(
        r##"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:a="urn:a" xmlns:b="urn:a" xmlns:c="urn:c"><xsl:output method="text"/>{body}</xsl:stylesheet>"##
    )
}
#[test]
fn aliases_resolve_for_bindings_calls_and_arguments() {
    let s = Stylesheet::compile(&sheet(r##"
      <xsl:variable name="a:g" select="$b:later"/>
      <xsl:variable name="a:later" select="'global'"/>
      <xsl:template match="/">
        <xsl:variable name="a:v" select="'local'"/>
        <xsl:call-template name="b:t"><xsl:with-param name="b:p" select="$b:v"/></xsl:call-template>
      </xsl:template>
      <xsl:template name="a:t"><xsl:param name="a:p"/><xsl:value-of select="concat($b:g, ':', $b:p)"/></xsl:template>
    "##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "global:local");
}
#[test]
fn rebindings_and_unqualified_names_are_distinct() {
    let s = Stylesheet::compile(&sheet(r##"
      <xsl:variable name="a:v" select="'A'"/><xsl:variable name="c:v" select="'C'"/>
      <xsl:variable name="v" select="'N'"/>
      <xsl:template match="/" xmlns="urn:default"><xsl:value-of select="$b:v"/><xsl:for-each select="*" xmlns:a="urn:c"><xsl:value-of select="$a:v"/><xsl:value-of select="$v"/></xsl:for-each></xsl:template>
    "##)).unwrap();
    assert_eq!(s.transform("<r/>").unwrap(), "ACN");
}
#[test]
fn modes_and_current_mode_use_expanded_names() {
    let s = Stylesheet::compile(&sheet(
        r##"
      <xsl:template match="/" mode="a:m"><xsl:apply-templates mode="#current"/></xsl:template>
      <xsl:template match="r" mode="b:m"><xsl:text>A</xsl:text></xsl:template>
      <xsl:template match="r" mode="c:m"><xsl:text>C</xsl:text></xsl:template>
    "##,
    ))
    .unwrap();
    let mut opts = TransformOptions {
        initial_mode: Some("Q{urn:a}m".into()),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), "A");
    opts.initial_mode = Some("Q{urn:c}m".into());
    assert_eq!(s.transform_with_options("<r/>", &opts).unwrap(), "C");
}
#[test]
fn external_template_and_global_parameter_names() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:param name="a:p" select="'default'"/><xsl:template name="a:t"><xsl:value-of select="$b:p"/></xsl:template>"##)).unwrap();
    let opts = TransformOptions {
        initial_template: Some("Q{urn:a}t".into()),
        parameters: [("Q{urn:a}p".into(), "supplied".into())].into(),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("", &opts).unwrap(), "supplied");
}
#[test]
fn stream_entry_uses_expanded_names() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template name="a:record"><xsl:call-template name="b:t"/></xsl:template><xsl:template name="a:t"><xsl:value-of select="."/></xsl:template>"##)).unwrap();
    let opts = StreamOptions {
        record_path: "/r/n".into(),
        template: "Q{urn:a}record".into(),
        ..Default::default()
    };
    let mut output = vec![];
    s.stream_records(Cursor::new("<r><n>ok</n></r>"), &mut output, &opts)
        .unwrap();
    assert_eq!(output, b"ok");
}
#[test]
fn aliased_duplicates_are_rejected() {
    for body in [
        r##"<xsl:variable name="a:v"/><xsl:variable name="b:v"/>"##,
        r##"<xsl:template name="a:t"/><xsl:template name="b:t"/>"##,
        r##"<xsl:template name="t"><xsl:param name="a:p"/><xsl:param name="b:p"/></xsl:template>"##,
        r##"<xsl:template name="t"><xsl:call-template name="t"><xsl:with-param name="a:p"/><xsl:with-param name="b:p"/></xsl:call-template></xsl:template>"##,
    ] {
        assert!(Stylesheet::compile(&sheet(body)).is_err(), "{body}");
    }
}
#[test]
fn unbound_and_malformed_names_fail_statically() {
    for body in [
        r##"<xsl:variable name="z:v"/>"##,
        r##"<xsl:template name="z:t"/>"##,
        r##"<xsl:template match="/" mode="z:m"/>"##,
        r##"<xsl:template name="t"><xsl:call-template name="z:t"/></xsl:template>"##,
    ] {
        assert_eq!(
            Stylesheet::compile(&sheet(body)).unwrap_err().code,
            "XTSE0280"
        );
    }
    assert_eq!(
        Stylesheet::compile(&sheet(
            r##"<xsl:template name="t"><xsl:value-of select="$z:v"/></xsl:template>"##
        ))
        .unwrap_err()
        .code,
        "XPST0081"
    );
    assert_eq!(
        Stylesheet::compile(&sheet(r##"<xsl:variable name="a:b:c"/>"##))
            .unwrap_err()
            .code,
        "XTSE0020"
    );
}
#[test]
fn unresolved_qualified_variables_are_checked_in_dead_code() {
    assert_eq!(Stylesheet::compile(&sheet(r##"<xsl:template name="t"><xsl:if test="false()"><xsl:value-of select="$a:missing"/></xsl:if></xsl:template>"##)).unwrap_err().code, "XPST0008");
}
#[test]
fn invalid_external_prefix_is_rejected() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template name="a:t"/>"##)).unwrap();
    let opts = TransformOptions {
        initial_template: Some("a:t".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("", &opts).unwrap_err().code,
        "RUST_XSLT_INVALID_NAME"
    );
}

#[test]
fn uriqualified_components_alias_prefixed_names() {
    let s = Stylesheet::compile(&sheet(
        r##"
      <xsl:variable name="Q{urn:a}g" select="'global'"/>
      <xsl:template name=" Q{urn:a}t "><xsl:param name="Q{urn:a}p"/>
        <xsl:value-of select="concat($Q{urn:a}g, ':', $b:p)"/>
      </xsl:template>
      <xsl:template match="/" mode="Q{urn:a}m"><xsl:call-template name="b:t">
        <xsl:with-param name="Q{urn:a}p" select="'param'"/>
      </xsl:call-template></xsl:template>
    "##,
    ))
    .unwrap();
    let opts = TransformOptions {
        initial_mode: Some("Q{urn:a}m".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<r/>", &opts).unwrap(),
        "global:param"
    );
}
#[test]
fn uriqualified_paths_functions_and_types() {
    let s = Stylesheet::compile(&sheet(r##"
      <xsl:template match="/">
        <xsl:variable name="Q{}v" as="Q{http://www.w3.org/2001/XMLSchema}integer" select="Q{http://www.w3.org/2001/XMLSchema}integer('7')"/>
        <xsl:value-of select="Q{http://www.w3.org/2005/xpath-functions}concat($v, ':', Q{urn:doc}r/Q{}n, ':', $v instance of Q{http://www.w3.org/2001/XMLSchema}integer)"/>
      </xsl:template>
    "##)).unwrap();
    assert_eq!(
        s.transform("<r xmlns='urn:doc'><n xmlns=''>ok</n></r>")
            .unwrap(),
        "7:ok:true"
    );
}
#[test]
fn uriqualified_match_and_wildcard_tests() {
    let s = Stylesheet::compile(&sheet(r##"
      <xsl:template match="/"><xsl:value-of select="count(Q{}*/*)"/><xsl:apply-templates select="*/*"/></xsl:template>
      <xsl:template match="Q{urn:doc}*">W</xsl:template>
      <xsl:template match="Q{urn:doc}n">N</xsl:template>
    "##)).unwrap();
    assert_eq!(
        s.transform("<r xmlns:p='urn:doc'><p:n/><p:o/></r>")
            .unwrap(),
        "2NW"
    );
}
#[test]
fn uriqualified_names_reject_malformed_tokens() {
    for expr in ["$Q{urn:a", "$Q{a{b}v", "$Q{urn:a}1v", "$Q{}", "$Q{urn:a}*"] {
        let body =
            format!(r##"<xsl:template name="t"><xsl:value-of select="{expr}"/></xsl:template>"##);
        assert_eq!(
            Stylesheet::compile(&sheet(&body)).unwrap_err().code,
            "XPST0003",
            "{expr}"
        );
    }
}
#[test]
fn explicitly_unnamespaced_function_has_no_default_function_namespace() {
    assert_eq!(
        Stylesheet::compile(&sheet(
            r##"<xsl:template name="t"><xsl:value-of select="Q{}concat('a','b')"/></xsl:template>"##
        ))
        .unwrap_err()
        .code,
        "XPST0017"
    );
}
#[test]
fn uriqualified_alias_duplicates_are_rejected() {
    assert!(
        Stylesheet::compile(&sheet(
            r##"<xsl:variable name="a:v"/><xsl:variable name="Q{urn:a}v"/>"##
        ))
        .is_err()
    );
}

#[test]
fn reserved_namespaces_are_rejected_for_component_declarations() {
    for uri in [
        "http://www.w3.org/1999/XSL/Transform",
        "http://www.w3.org/2005/xpath-functions",
        "http://www.w3.org/2005/xpath-functions/math",
        "http://www.w3.org/2005/xpath-functions/map",
        "http://www.w3.org/2005/xpath-functions/array",
        "http://www.w3.org/XML/1998/namespace",
        "http://www.w3.org/2001/XMLSchema",
        "http://www.w3.org/2001/XMLSchema-instance",
        "http://www.w3.org/2005/xqt-errors",
        "http://www.w3.org/2000/xmlns/",
    ] {
        for body in [
            format!(r##"<xsl:variable name="Q{{{uri}}}v"/>"##),
            format!(r##"<xsl:template name="Q{{{uri}}}t"/>"##),
            format!(r##"<xsl:template match="/" mode="Q{{{uri}}}m"/>"##),
            format!(r##"<xsl:template name="t"><xsl:param name="Q{{{uri}}}p"/></xsl:template>"##),
        ] {
            assert_eq!(
                Stylesheet::compile(&sheet(&body)).unwrap_err().code,
                "XTSE0080",
                "{body}"
            );
        }
    }
    assert_eq!(
        Stylesheet::compile(&sheet(r##"<xsl:variable name="xml:v"/>"##))
            .unwrap_err()
            .code,
        "XTSE0080"
    );
}
#[test]
fn initial_template_name_is_the_reserved_namespace_exception() {
    let s = Stylesheet::compile(&sheet(r##"<xsl:template xmlns:t="http://www.w3.org/1999/XSL/Transform" name="t:initial-template">entry</xsl:template>"##)).unwrap();
    let opts = TransformOptions {
        initial_template: Some("Q{http://www.w3.org/1999/XSL/Transform}initial-template".into()),
        ..Default::default()
    };
    assert_eq!(s.transform_with_options("", &opts).unwrap(), "entry");
}
#[test]
fn reserved_namespaces_remain_available_for_source_name_tests() {
    let s = Stylesheet::compile(&sheet(
        r##"<xsl:template match="/"><xsl:value-of select="r/@xml:lang"/></xsl:template>"##,
    ))
    .unwrap();
    assert_eq!(s.transform("<r xml:lang='fi'/>").unwrap(), "fi");
}

#[test]
fn uriqualified_names_work_inside_avts_and_computed_names() {
    let source = sheet(r##"<xsl:variable name="a:v" select="'value'"/><xsl:template match="/"><out result="{$Q{urn:a}v}" literal="{{Q{{uri}}local}}"><xsl:element name="{Q{http://www.w3.org/2005/xpath-functions}concat('n', 'ode')}"><xsl:value-of select="$a:v"/></xsl:element></out></xsl:template>"##).replace("<xsl:output method=\"text\"/>", "<xsl:output omit-xml-declaration=\"yes\"/>");
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "<out xmlns:a=\"urn:a\" xmlns:b=\"urn:a\" xmlns:c=\"urn:c\" result=\"value\" literal=\"{Q{uri}local}\"><node>value</node></out>"
    );
}
#[test]
fn avt_uri_literals_do_not_treat_quotes_as_string_delimiters() {
    let source = sheet(
        r##"<xsl:variable name="Q{urn:it's}v" select="'ok'"/><xsl:template match="/"><xsl:value-of select="('A','B')" separator="{$Q{urn:it's}v}"/></xsl:template>"##,
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<r/>")
            .unwrap(),
        "AokB"
    );
}
#[test]
fn malformed_avt_braces_report_standard_errors() {
    for value in ["{1", "1}", "{$Q{urn:a}v", "{1{2}}"] {
        let body = format!(r##"<xsl:template match="/"><out value="{value}"/></xsl:template>"##);
        assert_eq!(
            Stylesheet::compile(&sheet(&body)).unwrap_err().code,
            "XTSE0350"
        );
    }
}

#[test]
fn unicode_component_names_and_xpath_tokens() {
    let source = sheet(
        r##"
      <xsl:variable name="a:é" select="'global'"/>
      <xsl:function name="a:名"><xsl:param name="é"/><xsl:sequence select="$é"/></xsl:function>
      <xsl:template match="/">
        <xsl:variable name="Q{urn:a}名" select="Q{urn:a}名($b:é)"/>
        <xsl:value-of select="($Q{urn:a}名, Q{urn:doc}根/Q{urn:doc}é, *:根/*:é, Q{urn:doc}根/@*:名)" separator=":"/>
      </xsl:template>
    "##,
    );
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform("<根 xmlns='urn:doc' 名='attribute'><é>node</é></根>")
            .unwrap(),
        "global:node:node:attribute"
    );
}

#[test]
fn unicode_external_template_names() {
    let source = sheet(r##"<xsl:template name="a:名">ok</xsl:template>"##);
    let options = TransformOptions {
        initial_template: Some("Q{urn:a}名".into()),
        ..Default::default()
    };
    assert_eq!(
        Stylesheet::compile(&source)
            .unwrap()
            .transform_with_options("", &options)
            .unwrap(),
        "ok"
    );
}

#[test]
fn unicode_name_continuations_cannot_start_names() {
    for name in ["Q{urn:a}́bad", "Q{urn:a}·bad", "a:́bad"] {
        let source = sheet(&format!(r##"<xsl:variable name="{name}"/>"##));
        assert!(Stylesheet::compile(&source).is_err(), "{name}");
    }
}
