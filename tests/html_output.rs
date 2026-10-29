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
