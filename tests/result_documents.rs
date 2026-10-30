use rust_xslt::{Stylesheet, TransformOptions};
fn run(body: &str) -> Result<rust_xslt::TransformResult, rust_xslt::Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/">{body}</xsl:template></xsl:stylesheet>"#))?
        .transform_with_documents("<r><i>a</i><i>b</i></r>", &TransformOptions::default())
}
#[test]
fn secondary_results_are_returned_with_their_hrefs() {
    let result = run(r#"<main/><xsl:for-each select="r/i"><xsl:result-document href="out-{position()}.xml"><item><xsl:value-of select="."/></item></xsl:result-document></xsl:for-each>"#).unwrap();
    assert_eq!(result.principal, "<main></main>");
    let documents: Vec<(&str, &str)> = result
        .documents
        .iter()
        .map(|d| (d.href.as_str(), d.content.as_str()))
        .collect();
    assert_eq!(
        documents,
        vec![
            ("out-1.xml", "<item>a</item>"),
            ("out-2.xml", "<item>b</item>"),
        ]
    );
}
#[test]
fn result_document_methods_and_indentation_apply_per_document() {
    let result = run(r#"<xsl:result-document href="t.txt" method="text">hello</xsl:result-document><xsl:result-document href="p.html" method="html"><html><br/></html></xsl:result-document><xsl:result-document href="x.xml" indent="yes" omit-xml-declaration="yes"><a><b/></a></xsl:result-document>"#).unwrap();
    assert_eq!(result.principal, "");
    assert_eq!(result.documents[0].content, "hello");
    assert_eq!(result.documents[1].content, "<html><br></html>");
    assert_eq!(result.documents[2].content, "<a>\n  <b></b>\n</a>");
    let doctype = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><xsl:result-document href="d.xml" doctype-public="PUB" doctype-system="SYS"><root/></xsl:result-document></xsl:template></xsl:stylesheet>"#,
    )
    .unwrap()
    .transform_with_documents("<r/>", &TransformOptions::default())
    .unwrap();
    assert_eq!(
        doctype.documents[0].content,
        "<!DOCTYPE root PUBLIC \"PUB\" \"SYS\">\n<root></root>"
    );
    let principal_doctype = Stylesheet::compile(
        r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><xsl:result-document doctype-system="principal.dtd"><root/></xsl:result-document></xsl:template></xsl:stylesheet>"#,
    )
    .unwrap()
    .transform("<r/>")
    .unwrap();
    assert_eq!(
        principal_doctype,
        "<!DOCTYPE root SYSTEM \"principal.dtd\">\n<root></root>"
    );
}
#[test]
fn duplicate_hrefs_are_errors_and_missing_href_writes_to_the_principal_result() {
    assert_eq!(
        run(r#"<xsl:result-document href="a"><x/></xsl:result-document><xsl:result-document href="a"><y/></xsl:result-document>"#).unwrap_err().code,
        "XTDE1490"
    );
    let principal = run(r#"<xsl:result-document><x/></xsl:result-document>"#).unwrap();
    assert_eq!(principal.principal, "<x></x>");
    assert!(principal.documents.is_empty());
    let empty_href = run(r#"<xsl:result-document href=""><y/></xsl:result-document>"#).unwrap();
    assert_eq!(empty_href.principal, "<y></y>");
    assert!(
        run(r#"<xsl:result-document href="a" method="json"><x/></xsl:result-document>"#).is_err()
    );
    assert_eq!(
        run(r#"<x/><xsl:result-document><y/></xsl:result-document>"#)
            .unwrap_err()
            .code,
        "XTDE1490"
    );
    assert_eq!(
        run(r#"<xsl:result-document><x/></xsl:result-document><y/>"#)
            .unwrap_err()
            .code,
        "XTDE1490"
    );
    let plain = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><p/><xsl:result-document href="z"><q/></xsl:result-document></xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(plain.transform("<r/>").unwrap(), "<p></p>");
}

#[test]
fn result_document_encoding_is_evaluated_in_the_caller_context() {
    let result = run(r#"<xsl:variable name="encoding" select="'US-ASCII'"/><xsl:result-document encoding="{$encoding}"><out>é</out></xsl:result-document>"#).unwrap();
    assert_eq!(result.principal, "<out>&#xE9;</out>");
}

#[test]
fn result_document_method_is_evaluated_in_the_caller_context() {
    let result = run(r#"<xsl:variable name="method" select="'text'"/><xsl:result-document method="{$method}"><out>a &amp; b</out></xsl:result-document>"#).unwrap();
    assert_eq!(result.principal, "a & b");
}

#[test]
fn nested_result_documents_cannot_reuse_an_open_destination() {
    let error = run(r#"<xsl:result-document href="same.xml"><outer/><xsl:result-document href="same.xml"><inner/></xsl:result-document></xsl:result-document>"#).unwrap_err();
    assert_eq!(error.code, "XTDE1490");
}

#[test]
fn html_result_documents_retain_public_only_doctypes() {
    let result = run(r#"<xsl:result-document method="html" doctype-public="PUBLIC"><html/></xsl:result-document>"#).unwrap();
    assert_eq!(
        result.principal,
        "<!DOCTYPE html PUBLIC \"PUBLIC\">\n<html></html>"
    );
}

#[test]
fn dynamic_cdata_qname_errors_are_runtime_diagnostics() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:result-document cdata-section-elements="{r/@name}"><out/></xsl:result-document></xsl:template></xsl:stylesheet>"#).unwrap();
    for source in [
        r#"<r name="missing:item"/>"#,
        r#"<r name="bad:name:again"/>"#,
        "<r name=\"a\u{a0}b\"/>",
    ] {
        let error = sheet.transform(source).unwrap_err();
        assert_eq!(error.code, "XTDE0030");
        assert_eq!(error.phase, "run");
    }
}

#[test]
fn explicit_principal_serialization_overrides_implicit_declaration_conflicts() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output standalone="yes" omit-xml-declaration="yes"/><xsl:template match="/"><xsl:result-document standalone="omit"><out/></xsl:result-document></xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(sheet.transform("<r/>").unwrap(), "<out></out>");
}

#[test]
fn restricted_encoding_references_are_outside_cdata() {
    let result = run(r#"<xsl:result-document encoding="US-ASCII" cdata-section-elements="out"><out>aéz</out></xsl:result-document>"#).unwrap();
    assert_eq!(
        result.principal,
        "<out><![CDATA[a]]>&#xE9;<![CDATA[z]]></out>"
    );
}

#[test]
fn invalid_doctype_identifiers_report_serialization_parameter_errors() {
    for body in [
        r#"<xsl:result-document doctype-system="a&quot;b&apos;c"><out/></xsl:result-document>"#,
        r#"<xsl:result-document doctype-public="bad&lt;public" doctype-system="out.dtd"><out/></xsl:result-document>"#,
    ] {
        let error = run(body).unwrap_err();
        assert_eq!(error.code, "SEPM0016");
        assert_eq!(error.phase, "run");
    }
}

#[test]
fn uri_escaping_overrides_named_output_and_survives_try_capture() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:formats" exclude-result-prefixes="f"><xsl:output name="f:html" method="html" escape-uri-attributes="no"/><xsl:template match="/"><xsl:variable name="enabled" select="'yes'"/><xsl:result-document href="escaped.html" format="f:html" escape-uri-attributes="{$enabled}"><xsl:try><a href="é">x</a><xsl:catch><error/></xsl:catch></xsl:try></xsl:result-document><xsl:result-document href="literal.html" format="f:html"><xsl:try><a href="é">x</a><xsl:catch><error/></xsl:catch></xsl:try></xsl:result-document></xsl:template></xsl:stylesheet>"#).unwrap();
    let result = sheet
        .transform_with_documents("<r/>", &TransformOptions::default())
        .unwrap();
    assert_eq!(result.documents[0].content, "<a href=\"%C3%A9\">x</a>");
    assert_eq!(result.documents[1].content, "<a href=\"é\">x</a>");
}

#[test]
fn uri_escaping_avts_have_static_scope_and_dynamic_value_errors() {
    let error = run(r#"<xsl:variable name="flag" select="'maybe'"/><xsl:result-document method="html" escape-uri-attributes="{$flag}"><a/></xsl:result-document>"#).unwrap_err();
    assert_eq!(error.code, "XTDE0030");
    assert_eq!(error.phase, "run");
    assert!(error.message.contains("escape-uri-attributes"));
    assert!(error.message.contains("maybe"));
    let error = run(r#"<xsl:if test="false()"><xsl:result-document escape-uri-attributes="{$missing}"/></xsl:if>"#).unwrap_err();
    assert_eq!(error.code, "XPST0008");
    assert_eq!(error.phase, "compile");
}

#[test]
fn explicit_charset_parameters_in_media_types_are_rejected() {
    let error = run(r#"<xsl:variable name="media" select="'text/html; CHARSET = UTF-8'"/><xsl:result-document method="html" media-type="{$media}"><head/></xsl:result-document>"#).unwrap_err();
    assert_eq!(error.code, "SEPM0016");
    let valid = run(r#"<xsl:result-document method="html" media-type='text/html; note="x;charset=y"'><head/></xsl:result-document>"#).unwrap();
    assert!(valid.principal.contains("charset=UTF-8"));
}

#[test]
fn explicit_principal_media_type_overrides_unused_implicit_settings() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="html" media-type="text/html; charset=wrong"/><xsl:template match="/"><xsl:result-document media-type="text/html"><html><head/></html></xsl:result-document></xsl:template></xsl:stylesheet>"#).unwrap();
    let output = sheet.transform("<r/>").unwrap();
    assert!(output.contains("content=\"text/html; charset=UTF-8\""));
    assert!(!output.contains("wrong"));
}

#[test]
fn media_types_reject_unterminated_quoted_parameters() {
    let error = run(r#"<xsl:result-document method="html" media-type='text/html; note=&quot;open'><head/></xsl:result-document>"#).unwrap_err();
    assert_eq!(error.code, "SEPM0016");
    assert_eq!(error.phase, "run");
    assert!(error.message.contains("unterminated"));
}

#[test]
fn generated_metadata_uses_each_result_documents_dynamic_settings() {
    for href in ["", "href='secondary.html'"] {
        let body = r#"<xsl:variable name="media" select="'application/example'"/><xsl:variable name="encoding" select="'US-ASCII'"/><xsl:variable name="include" select="'yes'"/><xsl:result-document $HREF method="html" encoding="{$encoding}" media-type="{$media}" include-content-type="{$include}"><xsl:try><html><head><title>é</title></head></html><xsl:catch><error/></xsl:catch></xsl:try></xsl:result-document>"#.replace("$HREF", href);
        let result = run(&body).unwrap();
        let content = if href.is_empty() {
            &result.principal
        } else {
            &result.documents[0].content
        };
        assert!(content.contains("content=\"application/example; charset=US-ASCII\""));
        assert!(content.contains("<title>&#xE9;</title>"));
        assert_eq!(content.matches("charset=").count(), 1);
    }
}

#[test]
fn prefix_undeclaration_flags_follow_the_output_method() {
    let error = run(r#"<xsl:variable name="flag" select="'yes'"/><xsl:result-document method="xml" undeclare-prefixes="{$flag}"><out/></xsl:result-document>"#).unwrap_err();
    assert_eq!(error.code, "SEPM0010");
    assert_eq!(run(r#"<xsl:result-document method="text" undeclare-prefixes="yes"><out>text</out></xsl:result-document>"#).unwrap().principal, "text");
}

#[test]
fn explicit_principal_results_can_disable_inherited_prefix_undeclarations() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="xml" omit-xml-declaration="yes" undeclare-prefixes="yes"/><xsl:template match="/"><xsl:result-document undeclare-prefixes="no"><out/></xsl:result-document></xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(sheet.transform("<r/>").unwrap(), "<out></out>");
    let inherited = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="xml" undeclare-prefixes="yes"/><xsl:template match="/"><out/></xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(inherited.transform("<r/>").unwrap_err().code, "SEPM0010");
}
