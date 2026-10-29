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
