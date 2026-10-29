use rust_xslt::{Limits, StreamOptions, Stylesheet, TransformOptions};
use std::io::Cursor;
fn sheet(output: &str, body: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output {output}/><xsl:template match="/" name="record">{body}</xsl:template></xsl:stylesheet>"#)).unwrap()
}

#[test]
fn xml_declaration_flags_accept_all_boolean_lexical_forms() {
    for value in ["yes", "true", "1", " true "] {
        assert_eq!(
            sheet(&format!("omit-xml-declaration='{value}'"), "<r/>")
                .transform("<s/>")
                .unwrap(),
            "<r></r>"
        );
    }
    for value in ["no", "false", "0"] {
        assert!(
            sheet(&format!("omit-xml-declaration='{value}'"), "<r/>")
                .transform("<s/>")
                .unwrap()
                .starts_with("<?xml")
        );
    }
}

#[test]
fn standalone_is_emitted_or_omitted_and_conflicting_options_fail() {
    for (value, expected) in [
        ("yes", "yes"),
        ("true", "yes"),
        ("1", "yes"),
        ("no", "no"),
        ("false", "no"),
        ("0", "no"),
    ] {
        assert_eq!(
            sheet(&format!("standalone='{value}'"), "<r/>")
                .transform("<s/>")
                .unwrap(),
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"{expected}\"?><r></r>")
        );
        assert_eq!(
            sheet(
                &format!("standalone='{value}' omit-xml-declaration='yes'"),
                "<r/>"
            )
            .transform("<s/>")
            .unwrap_err()
            .code,
            "SEPM0009"
        );
    }
    assert_eq!(
        sheet("standalone='omit' omit-xml-declaration='yes'", "<r/>")
            .transform("<s/>")
            .unwrap(),
        "<r></r>"
    );
}

#[test]
fn html_and_text_methods_ignore_xml_declaration_settings() {
    assert_eq!(
        sheet(
            "method='html' standalone='yes' omit-xml-declaration='no'",
            "<br/>"
        )
        .transform("<s/>")
        .unwrap(),
        "<br>"
    );
    assert_eq!(
        sheet(
            "method='text' standalone='yes' omit-xml-declaration='yes'",
            "text"
        )
        .transform("<s/>")
        .unwrap(),
        "text"
    );
}

#[test]
fn utf8_bom_precedes_declarations_and_is_counted_in_output_limits() {
    let s = sheet("byte-order-mark='yes' standalone='yes'", "<r/>");
    assert!(s.transform("<s/>").unwrap().starts_with("\u{feff}<?xml"));
    assert_eq!(
        sheet("method='text' byte-order-mark='true'", "x")
            .transform("<s/>")
            .unwrap(),
        "\u{feff}x"
    );
    assert_eq!(
        sheet(
            "method='text' byte-order-mark='yes' encoding='ISO-8859-1'",
            "x"
        )
        .transform("<s/>")
        .unwrap(),
        "x"
    );
    let options = TransformOptions {
        limits: Limits {
            max_output_bytes: 3,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet("method='text' byte-order-mark='yes'", "x")
            .transform_with_options("<s/>", &options)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}

#[test]
fn record_streaming_emits_one_preamble_for_all_records() {
    let s = sheet("byte-order-mark='yes' standalone='no'", "<r/>");
    let options = StreamOptions {
        record_path: "/s/n".into(),
        template: "record".into(),
        ..Default::default()
    };
    let mut output = vec![];
    let stats = s
        .stream_records(Cursor::new("<s><n/><n/></s>"), &mut output, &options)
        .unwrap();
    let expected =
        "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?><r></r><r></r>";
    assert_eq!(output, expected.as_bytes());
    assert_eq!(stats.output_bytes, expected.len());
}

#[test]
fn secondary_results_have_independent_declaration_options() {
    let s = sheet(
        "standalone='yes' byte-order-mark='yes'",
        r#"<xsl:result-document href="a" standalone="omit" byte-order-mark="no" omit-xml-declaration="yes"><r/></xsl:result-document><xsl:result-document href="b"><r/></xsl:result-document>"#,
    );
    let result = s
        .transform_with_documents("<s/>", &TransformOptions::default())
        .unwrap();
    assert_eq!(result.documents[0].content, "<r></r>");
    assert_eq!(
        result.documents[1].content,
        "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><r></r>"
    );
}

#[test]
fn invalid_serialization_booleans_are_static_errors() {
    for name in [
        "omit-xml-declaration",
        "standalone",
        "byte-order-mark",
        "include-content-type",
        "escape-uri-attributes",
        "undeclare-prefixes",
    ] {
        let source = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output {name}="maybe"/></xsl:stylesheet>"#
        );
        assert_eq!(
            Stylesheet::compile(&source).unwrap_err().code,
            "XTSE0020",
            "{name}"
        );
    }
}
