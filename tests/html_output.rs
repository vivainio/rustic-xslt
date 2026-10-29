use rust_xslt::Stylesheet;
fn run(output: &str, body: &str) -> String {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output {output}/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))
        .unwrap()
        .transform("<r/>")
        .unwrap()
}
#[test]
fn html_method_omits_void_end_tags_and_declaration() {
    assert_eq!(
        run(
            r#"method="html""#,
            "<html><body><br/><p>a<br/>b</p><img src='x.png'/><div/></body></html>"
        ),
        "<html><body><br><p>a<br>b</p><img src=\"x.png\"><div></div></body></html>"
    );
}
#[test]
fn html_script_and_style_text_is_not_escaped_and_booleans_minimize() {
    assert_eq!(
        run(
            r#"method="html""#,
            "<html><script>if (a &lt; b &amp;&amp; c) {}</script><style>a &gt; b {}</style><p>x &lt; y</p><input checked='checked' value='a&amp;b\"'/></html>"
        ),
        "<html><script>if (a < b && c) {}</script><style>a > b {}</style><p>x &lt; y</p><input checked value=\"a&amp;b&quot;\"></html>"
    );
}
#[test]
fn html_element_names_match_case_insensitively() {
    assert_eq!(
        run(r#"method="html""#, "<HTML><BR/></HTML>"),
        "<HTML><BR></HTML>"
    );
}
#[test]
fn xhtml_method_uses_self_closing_void_elements() {
    assert_eq!(
        run(
            r#"method="xhtml" omit-xml-declaration="yes""#,
            "<html xmlns='http://www.w3.org/1999/xhtml'><br/><p/><img src='a'/></html>"
        ),
        "<html xmlns=\"http://www.w3.org/1999/xhtml\"><br /><p></p><img src=\"a\" /></html>"
    );
}
#[test]
fn doctype_declarations_precede_the_root_element() {
    assert_eq!(
        run(
            r#"method="html" doctype-public="-//W3C//DTD HTML 4.01//EN" doctype-system="http://www.w3.org/TR/html4/strict.dtd""#,
            "<html/>"
        ),
        "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\" \"http://www.w3.org/TR/html4/strict.dtd\">\n<html></html>"
    );
    assert_eq!(
        run(
            r#"method="xml" omit-xml-declaration="yes" doctype-system="a.dtd""#,
            "<root/>"
        ),
        "<!DOCTYPE root SYSTEM \"a.dtd\">\n<root></root>"
    );
    assert_eq!(
        run(
            r#"method="html" doctype-system="about:legacy-compat""#,
            "<html/>"
        ),
        "<!DOCTYPE html SYSTEM \"about:legacy-compat\">\n<html></html>"
    );
}
#[test]
fn indentation_keeps_html_void_elements_balanced() {
    assert_eq!(
        run(
            r#"method="html" indent="yes""#,
            "<html><body><br/><p>t<b>x</b></p><img src='a'/><div><hr/></div></body></html>"
        ),
        "<html>\n  <body>\n    <br>\n    <p>t<b>x</b></p>\n    <img src=\"a\">\n    <div>\n      <hr>\n    </div>\n  </body>\n</html>"
    );
}

#[test]
fn xml_public_doctype_without_system_is_ignored() {
    assert_eq!(
        run(
            r#"method="xml" omit-xml-declaration="yes" doctype-public="PUBLIC""#,
            "<root/>"
        ),
        "<root></root>"
    );
    assert_eq!(
        run(r#"method="html" doctype-public="PUBLIC""#, "<html/>"),
        "<!DOCTYPE html PUBLIC \"PUBLIC\">\n<html></html>"
    );
}

#[test]
fn doctype_system_identifiers_choose_a_safe_quote_delimiter() {
    assert_eq!(
        run(
            r#"method="xml" omit-xml-declaration="yes" doctype-system="a&quot;b.dtd""#,
            "<root/>"
        ),
        "<!DOCTYPE root SYSTEM 'a\"b.dtd'>\n<root></root>"
    );
}

#[test]
fn uri_attributes_normalize_before_escaping_and_preserve_relative_paths() {
    assert_eq!(
        run(
            r#"method="html""#,
            "<a href='a&#x30a;?x=1&amp;y=2' title='å'>x</a>"
        ),
        "<a href=\"%C3%A5?x=1&amp;y=2\" title=\"å\">x</a>"
    );
}

#[test]
fn namespaced_attributes_do_not_use_html_uri_escaping() {
    let output = run(
        r#"method="html""#,
        "<a xmlns:p='urn:p' href='é' p:href='é'>x</a>",
    );
    assert!(output.contains(" href=\"%C3%A9\""));
    assert!(output.contains(" p:href=\"é\""));
}

#[test]
fn xhtml_uri_attribute_matching_is_case_sensitive() {
    let output = run(
        r#"method="xhtml" omit-xml-declaration="yes""#,
        "<div xmlns='http://www.w3.org/1999/xhtml'><a href='é' HREF='é'/><A href='é'/></div>",
    );
    assert!(output.contains("<a href=\"%C3%A9\" HREF=\"é\""));
    assert!(output.contains("<A href=\"é\""));
}

#[test]
fn xml_output_ignores_uri_escaping_and_xhtml_type_names_are_exact() {
    assert_eq!(
        run(
            r#"method="xml" omit-xml-declaration="yes" escape-uri-attributes="yes""#,
            "<a href='a&#x30a;'/>"
        ),
        "<a href=\"a\u{30a}\"></a>"
    );
    let output = run(
        r#"method="xhtml" omit-xml-declaration="yes""#,
        "<div xmlns='http://www.w3.org/1999/xhtml'><input type='url' value='é'/><input TYPE='url' value='é'/></div>",
    );
    assert!(output.contains("type=\"url\" value=\"%C3%A9\""));
    assert!(output.contains("TYPE=\"url\" value=\"é\""));
}

#[test]
fn content_type_metadata_replaces_only_matching_direct_children() {
    let body = "<html><head><meta http-equiv=' Content-Type ' content='old'/><meta name='keep' content='x'/><title>T</title></head></html>";
    let output = run(r#"method="html" media-type="application/example""#, body);
    assert!(output.contains(
        "<head><meta http-equiv=\"Content-Type\" content=\"application/example; charset=UTF-8\">"
    ));
    assert!(!output.contains("content=\"old\""));
    assert!(output.contains("name=\"keep\""));
    let disabled = run(r#"method="html" include-content-type="no""#, body);
    assert!(disabled.contains("content=\"old\""));
    assert!(!disabled.contains("charset="));
}

#[test]
fn xhtml_metadata_uses_the_head_namespace_and_exact_names() {
    let output = run(
        r#"method="xhtml" omit-xml-declaration="yes""#,
        "<h:html xmlns:h='http://www.w3.org/1999/xhtml'><h:head><h:title>T</h:title></h:head><h:HEAD/></h:html>",
    );
    assert!(output.contains("<h:head><h:meta http-equiv=\"Content-Type\""));
    assert_eq!(output.matches("charset=").count(), 1);
}

#[test]
fn xhtml_metadata_replacement_respects_attribute_case_and_namespace() {
    let output = run(
        r#"method="xhtml" omit-xml-declaration="yes""#,
        "<html xmlns='http://www.w3.org/1999/xhtml' xmlns:p='urn:p'><head><meta HTTP-EQUIV='Content-Type' content='upper'/><meta p:http-equiv='Content-Type' content='namespaced'/><meta http-equiv=' content-type ' content='replace'/></head></html>",
    );
    assert!(output.contains("content=\"upper\""));
    assert!(output.contains("content=\"namespaced\""));
    assert!(!output.contains("content=\"replace\""));
    assert_eq!(output.matches("charset=").count(), 1);
}

#[test]
fn metadata_replacement_survives_try_capture_inside_head() {
    let output = run(
        r#"method="html""#,
        "<html><head><xsl:try><meta http-equiv='Content-Type' content='old'/><title>T</title><xsl:catch><error/></xsl:catch></xsl:try></head></html>",
    );
    assert_eq!(output.matches("charset=").count(), 1);
    assert!(!output.contains("content=\"old\""));
    assert!(output.contains("<title>T</title>"));
}
