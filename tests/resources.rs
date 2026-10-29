use rust_xslt::{Diagnostic, ResourceResolver, Stylesheet, TransformOptions};
fn options() -> TransformOptions {
    TransformOptions {
        resolver: Some(ResourceResolver::new(|href| match href {
            "a.txt" => Ok("line one\nline two\n".to_owned()),
            "data.json" => Ok(r#"{"k": [1, 2, 3]}"#.to_owned()),
            _ => Err(Diagnostic {
                code: "XTRE0000".into(),
                message: format!("not found: {href}"),
                phase: "run",
                line: 0,
                column: 0,
            }),
        })),
        ..TransformOptions::default()
    }
}
fn run(expr: &str, options: &TransformOptions) -> Result<String, Diagnostic> {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output method="text"/><xsl:template match="/"><xsl:sequence select="{expr}"/></xsl:template></xsl:stylesheet>"#))?
        .transform_with_options("<r/>", options)
}
#[test]
fn unparsed_text_and_json_doc_use_the_resolver() {
    let o = options();
    assert_eq!(
        run("unparsed-text('a.txt')", &o).unwrap(),
        "line one\nline two\n"
    );
    assert_eq!(
        run("unparsed-text-lines('a.txt')", &o).unwrap(),
        "line one line two"
    );
    assert_eq!(run("count(unparsed-text-lines('a.txt'))", &o).unwrap(), "2");
    assert_eq!(run("unparsed-text-available('a.txt')", &o).unwrap(), "true");
    assert_eq!(
        run("unparsed-text-available('missing.txt')", &o).unwrap(),
        "false"
    );
    assert_eq!(run("json-doc('data.json')?k?2", &o).unwrap(), "2");
    assert_eq!(run("empty(unparsed-text(()))", &o).unwrap(), "true");
}
#[test]
fn missing_resources_and_resolvers_are_errors() {
    assert_eq!(
        run("unparsed-text('missing.txt')", &options())
            .unwrap_err()
            .code,
        "FOUT1170"
    );
    assert_eq!(
        run("unparsed-text('a.txt')", &TransformOptions::default())
            .unwrap_err()
            .code,
        "FOUT1170"
    );
    assert_eq!(
        run(
            "unparsed-text-available('a.txt')",
            &TransformOptions::default()
        )
        .unwrap(),
        "false"
    );
}
#[test]
fn doc_loads_documents_named_by_literal_uris() {
    let o = TransformOptions {
        resolver: Some(ResourceResolver::new(|href| match href {
            "lib.xml" => Ok(
                "<?xml version=\"1.0\"?><lib><b id=\"1\">one</b><b id=\"2\">two</b></lib>"
                    .to_owned(),
            ),
            _ => Err(Diagnostic {
                code: "XTRE0000".into(),
                message: "missing".into(),
                phase: "run",
                line: 0,
                column: 0,
            }),
        })),
        ..TransformOptions::default()
    };
    assert_eq!(run("doc('lib.xml')/lib/b[2]", &o).unwrap(), "two");
    assert_eq!(run("count(doc('lib.xml')//b)", &o).unwrap(), "2");
    assert_eq!(
        run("string-join(document('lib.xml')//b/@id, ',')", &o).unwrap(),
        "1,2"
    );
    assert_eq!(run("doc-available('lib.xml')", &o).unwrap(), "true");
    assert_eq!(run("doc-available('nope.xml')", &o).unwrap(), "false");
    assert_eq!(run("doc('nope.xml')", &o).unwrap_err().code, "FODC0002");
    assert_eq!(run("doc('lib.xml') is doc('lib.xml')", &o).unwrap(), "true");
}

#[test]
fn global_bindings_load_documents_before_evaluation() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    let options = TransformOptions {
        resolver: Some(ResourceResolver::new(move |_| {
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("<lib><b>one</b><b>two</b></lib>".into())
        })),
        ..TransformOptions::default()
    };
    let sheet = Stylesheet::compile(r#"
        <xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:f="urn:test">
          <xsl:output method="text"/>
          <xsl:variable name="count" select="count($doc/lib/b)"/>
          <xsl:variable name="doc" select="doc('lib.xml')"/>
          <xsl:variable name="available" select="doc-available('lib.xml')"/>
          <xsl:function name="f:loaded"><xsl:sequence select="doc('lib.xml')"/></xsl:function>
          <xsl:template match="/">
            <xsl:sequence select="$count, $available, $doc is doc('lib.xml'), $doc is f:loaded()"/>
          </xsl:template>
        </xsl:stylesheet>"#).unwrap();
    assert_eq!(
        sheet.transform_with_options("<r/>", &options).unwrap(),
        "2 true true true"
    );
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(sheet.transform("<r/>").unwrap_err().code, "FODC0002");
}

#[test]
fn loaded_documents_use_xml_document_parsing_and_limits() {
    let options = TransformOptions {
        resolver: Some(ResourceResolver::new(|uri| {
            Ok(match uri {
                "valid.xml" => "\u{feff}<?xml version='1.0'?>\n<!--before--><r/>\n<?after data?>\n",
                "fragment.xml" => "<a/><b/>",
                "text.xml" => "text",
                _ => "<broken>",
            }
            .into())
        })),
        ..TransformOptions::default()
    };
    assert_eq!(
        run("count(doc('valid.xml')/node())", &options).unwrap(),
        "3"
    );
    assert_eq!(
        run("count(doc('valid.xml')/text())", &options).unwrap(),
        "0"
    );
    assert_eq!(
        run(
            "doc('valid.xml')/r/parent::node() is doc('valid.xml')",
            &options
        )
        .unwrap(),
        "true"
    );
    for uri in ["fragment.xml", "text.xml", "broken.xml"] {
        assert_eq!(
            run(&format!("doc-available('{uri}')"), &options).unwrap(),
            "false"
        );
        assert_eq!(
            run(&format!("doc('{uri}')"), &options).unwrap_err().code,
            "FODC0002"
        );
    }
    let small = TransformOptions {
        limits: rust_xslt::Limits {
            max_temporary_bytes: 4,
            ..Default::default()
        },
        ..options
    };
    assert_eq!(
        run("doc('valid.xml')", &small).unwrap_err().code,
        "XPDY0130"
    );
}
