use rust_xslt::{ResourceResolver, Stylesheet, TransformOptions};

#[test]
fn stylesheet_calls_and_function_items_capture_lexical_base() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:f" xml:base="sub/"><xsl:output method="text"/><xsl:function name="f:base" xml:base="function/"><xsl:sequence select="static-base-uri()"/></xsl:function><xsl:template match="/"><xsl:variable name="get" select="static-base-uri#0"/><xsl:variable name="resolve" select="resolve-uri#1"/><xsl:for-each select="."><xsl:sequence xml:base="nested/" select="static-base-uri(), resolve-uri('item'), $get(), $resolve('item'), f:base()"/></xsl:for-each></xsl:template></xsl:stylesheet>"#;
    let compiled =
        Stylesheet::compile_with_base_uri(sheet, "https://example.test/main.xsl").unwrap();
    assert_eq!(
        compiled.transform("<r/>").unwrap(),
        "https://example.test/sub/nested/ https://example.test/sub/nested/item https://example.test/sub/ https://example.test/sub/item https://example.test/sub/function/"
    );
}

#[test]
fn stylesheet_base_resolves_literal_documents_and_resources() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="doc('doc.xml')/r, unparsed-text('text.txt')"/></xsl:template></xsl:stylesheet>"#;
    let compiled =
        Stylesheet::compile_with_base_uri(sheet, "https://example.test/dir/main.xsl").unwrap();
    let options = TransformOptions {
        resolver: Some(ResourceResolver::new(|uri| match uri {
            "https://example.test/dir/doc.xml" => Ok("<r>document</r>".into()),
            "https://example.test/dir/text.txt" => Ok("text".into()),
            _ => panic!("unexpected URI: {uri}"),
        })),
        ..Default::default()
    };
    assert_eq!(
        compiled
            .transform_with_options("<input/>", &options)
            .unwrap(),
        "documenttext"
    );
}

#[test]
fn document_pattern_and_lookup_keep_their_static_base() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:variable name="f" select="function-lookup(QName('http://www.w3.org/2005/xpath-functions', 'doc'), 1)"/><xsl:apply-templates select="$f('doc.xml')/r"/></xsl:template><xsl:template match="doc('doc.xml')/r">matched</xsl:template></xsl:stylesheet>"#;
    let compiled =
        Stylesheet::compile_with_base_uri(sheet, "https://example.test/dir/main.xsl").unwrap();
    let options = TransformOptions {
        resolver: Some(ResourceResolver::new(|uri| {
            assert_eq!(uri, "https://example.test/dir/doc.xml");
            Ok("<r/>".into())
        })),
        ..Default::default()
    };
    assert_eq!(
        compiled
            .transform_with_options("<input/>", &options)
            .unwrap(),
        "matched"
    );
}

#[test]
fn document_resolves_using_static_or_node_base_and_removes_duplicates() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:variable name="f" select="document#1"/><xsl:sequence select="document('asset.xml') is doc('https://example.test/styles/asset.xml'), document('asset.xml', /r) is doc('https://example.test/data/asset.xml'), document(/r/@href) is doc('https://example.test/data/asset.xml'), count(document(('asset.xml', 'asset.xml'))), $f('asset.xml') is doc('https://example.test/styles/asset.xml')"/></xsl:template></xsl:stylesheet>"#;
    let compiled =
        Stylesheet::compile_with_base_uri(sheet, "https://example.test/styles/main.xsl").unwrap();
    let options = TransformOptions {
        source_uri: Some("https://example.test/data/source.xml".into()),
        resolver: Some(ResourceResolver::new(|uri| match uri {
            "https://example.test/styles/asset.xml" | "https://example.test/data/asset.xml" => {
                Ok("<r/>".into())
            }
            _ => Err(rust_xslt::Diagnostic {
                code: "FODC0002".into(),
                message: format!("missing: {uri}"),
                phase: "run",
                line: 0,
                column: 0,
            }),
        })),
        ..Default::default()
    };
    assert_eq!(
        compiled
            .transform_with_options(r#"<r href="asset.xml"/>"#, &options)
            .unwrap(),
        "true true true 1 true"
    );
}

#[test]
fn document_checks_arguments_and_unsupported_fragments() {
    for (expr, code) in [
        ("document(42)", "XPTY0004"),
        ("document('x', ())", "XPTY0004"),
        ("document('x', 1)", "XPTY0004"),
    ] {
        let sheet = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#
        );
        assert_eq!(
            Stylesheet::compile_with_base_uri(&sheet, "https://example.test/main.xsl")
                .unwrap()
                .transform("<r/>")
                .unwrap_err()
                .code,
            code,
            "{expr}"
        );
    }
}

#[test]
fn document_selects_xml_shorthand_and_element_fragments() {
    let sheet = r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="name(document('data.xml#alpha')), name(document('data.xml#element(/1/2)')), name(document('data.xml#element(alpha/2)')), name(document('data.xml#unknown(a(b))element(/1/1)')), name(document('data.xml#%C3%A9')), count(document(('data.xml#alpha','data.xml#element(/1/1)')))"/></xsl:template></xsl:stylesheet>"#;
    let compiled =
        Stylesheet::compile_with_base_uri(sheet, "https://example.test/main.xsl").unwrap();
    let options = TransformOptions {
        resolver: Some(ResourceResolver::new(|uri| {
            assert_eq!(uri, "https://example.test/data.xml");
            Ok(
                "<root><first xml:id='alpha'><a/><b/></first><!--x--><second xml:id='é'/></root>"
                    .into(),
            )
        })),
        ..Default::default()
    };
    assert_eq!(
        compiled
            .transform_with_options("<input/>", &options)
            .unwrap(),
        "first second b first second 1"
    );
    for fragment in [
        "missing",
        "element(/1/0)",
        "element(/2)",
        "unknown(foo)",
        "element(/1",
        "%FF",
        "",
    ] {
        let sheet = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:sequence select="document('data.xml#{fragment}')"/></xsl:template></xsl:stylesheet>"#
        );
        let compiled =
            Stylesheet::compile_with_base_uri(&sheet, "https://example.test/main.xsl").unwrap();
        assert_eq!(
            compiled
                .transform_with_options("<input/>", &options)
                .unwrap_err()
                .code,
            "XTDE1160",
            "{fragment}"
        );
    }
}

#[test]
fn document_reports_a_missing_base_for_relative_references() {
    for expr in [
        "document('relative.xml')",
        "document('relative.xml', /r)",
        "document(/r/@href)",
    ] {
        let sheet = format!(
            r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#
        );
        assert_eq!(
            Stylesheet::compile(&sheet)
                .unwrap()
                .transform(r#"<r href="relative.xml"/>"#)
                .unwrap_err()
                .code,
            "XTDE1162",
            "{expr}"
        );
    }
}
