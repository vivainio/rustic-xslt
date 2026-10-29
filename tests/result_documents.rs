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
            (
                "out-1.xml",
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><item>a</item>"
            ),
            (
                "out-2.xml",
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><item>b</item>"
            ),
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
}
#[test]
fn duplicate_or_missing_hrefs_are_errors() {
    assert_eq!(
        run(r#"<xsl:result-document href="a"><x/></xsl:result-document><xsl:result-document href="a"><y/></xsl:result-document>"#).unwrap_err().code,
        "XTDE1490"
    );
    assert_eq!(
        run(r#"<xsl:result-document><x/></xsl:result-document>"#)
            .unwrap_err()
            .code,
        "XTDE1490"
    );
    assert!(
        run(r#"<xsl:result-document href="a" method="json"><x/></xsl:result-document>"#).is_err()
    );
    let plain = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output omit-xml-declaration="yes"/><xsl:template match="/"><p/><xsl:result-document href="z"><q/></xsl:result-document></xsl:template></xsl:stylesheet>"#).unwrap();
    assert_eq!(plain.transform("<r/>").unwrap(), "<p></p>");
}
