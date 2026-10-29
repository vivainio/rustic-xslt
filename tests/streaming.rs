use rust_xslt::{StreamOptions, Stylesheet, TransformOptions};
use std::io::{self, BufRead, Cursor, Read, Write};
fn sheet(body: &str) -> Stylesheet {
    Stylesheet::compile(&format!(r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:output method="text"/>{body}</xsl:stylesheet>"#)).unwrap()
}
#[test]
fn stream_matches_grounded_for_each_and_preserves_focus() {
    let s = sheet(
        r#"<xsl:template match="/"><xsl:for-each select="records/record"><xsl:call-template name="record"/></xsl:for-each></xsl:template><xsl:template name="record"><xsl:value-of select="concat(position(), ':', @id, ':', normalize-space(title))"/><xsl:text>;</xsl:text></xsl:template>"#,
    );
    let input = "<records><skip/><record id='a'><title> One </title></record><record id='b'><title>Two</title></record><record id='c'/></records>";
    let mut output = vec![];
    let stats = s
        .stream_records(Cursor::new(input), &mut output, &StreamOptions::default())
        .unwrap();
    assert_eq!(stats.records, 3);
    assert_eq!(output, s.transform(input).unwrap().as_bytes());
    assert_eq!(output, b"1:a:One;2:b:Two;3:c:;");
    assert_eq!(stats.output_bytes, output.len());
}
#[test]
fn inherited_namespaces_entities_and_cdata_survive_record_detachment() {
    let s = sheet(
        r#"<xsl:template name="record" xmlns:p="urn:p"><xsl:value-of select="concat(@p:id, ':', p:title, ':', .)"/><xsl:text>;</xsl:text></xsl:template>"#,
    );
    let options = StreamOptions {
        record_path: "/p:records/p:record".into(),
        namespaces: [("p".into(), "urn:p".into())].into(),
        ..Default::default()
    };
    let input = r#"<p:records xmlns:p="urn:p"><p:record p:id="a"><p:title>A&amp;B</p:title><![CDATA[<text>]]></p:record><p:record xmlns:p="urn:other" p:id="skip"/><p:record p:id="b"/></p:records>"#;
    let mut output = vec![];
    let stats = s
        .stream_records(Cursor::new(input), &mut output, &options)
        .unwrap();
    assert_eq!(stats.records, 2);
    assert_eq!(output, b"a:A&B:A&B<text>;b::;");
}
#[test]
fn fixed_depth_prevents_nested_record_double_counting() {
    let s = sheet(r#"<xsl:template name="record"><xsl:value-of select="@id"/></xsl:template>"#);
    let mut output = vec![];
    let stats = s
        .stream_records(
            Cursor::new("<records><record id='outer'><record id='inner'/></record></records>"),
            &mut output,
            &StreamOptions::default(),
        )
        .unwrap();
    assert_eq!(stats.records, 1);
    assert_eq!(output, b"outer");
}
#[test]
fn static_stream_rejection_happens_before_any_input_or_output() {
    struct Unreadable;
    impl Read for Unreadable {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("input consumed before static rejection")
        }
    }
    impl BufRead for Unreadable {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            panic!("input consumed before static rejection")
        }
        fn consume(&mut self, _: usize) {
            panic!()
        }
    }
    for expression in [
        "../secret",
        "/records/record",
        "last()",
        "lang('en')",
        "generate-id()",
    ] {
        let s = sheet(&format!(
            r#"<xsl:template name="record"><xsl:value-of select="{expression}"/></xsl:template>"#
        ));
        let mut output = vec![];
        let e = s
            .stream_records(Unreadable, &mut output, &StreamOptions::default())
            .unwrap_err();
        assert_eq!(e.code, "RUST_XSLT_STREAM_UNSUPPORTED");
        assert!(output.is_empty());
    }
    let s = sheet(r#"<xsl:template match=".[true()]"/>"#);
    let mut output = vec![];
    assert_eq!(
        s.stream_records(Unreadable, &mut output, &StreamOptions::default())
            .unwrap_err()
            .code,
        "RUST_XSLT_STREAM_UNSUPPORTED"
    );
    assert!(output.is_empty());
}
#[test]
fn unsafe_globals_and_indirect_calls_are_rejected() {
    for body in [
        r#"<xsl:variable name="secret" select="/records/@secret"/><xsl:template name="record"><xsl:value-of select="$secret"/></xsl:template>"#,
        r#"<xsl:template name="record"><xsl:call-template name="escape"/></xsl:template><xsl:template name="escape"><xsl:value-of select=".."/></xsl:template>"#,
    ] {
        let s = sheet(body);
        assert_eq!(
            s.stream_records(
                Cursor::new("<records/>"),
                &mut vec![],
                &StreamOptions::default()
            )
            .unwrap_err()
            .code,
            "RUST_XSLT_STREAM_UNSUPPORTED"
        );
    }
}
#[test]
fn budgets_cover_all_records_and_aggregate_output() {
    let s = sheet(r#"<xsl:template name="record"><xsl:text>abc</xsl:text></xsl:template>"#);
    let input = "<records><record/><record/></records>";
    let mut output = vec![];
    let options = StreamOptions {
        limits: rust_xslt::Limits {
            max_output_bytes: 5,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        s.stream_records(Cursor::new(input), &mut output, &options)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
    assert_eq!(output, b"abc");
    let options = StreamOptions {
        limits: rust_xslt::Limits {
            max_steps: 25,
            ..Default::default()
        },
        ..Default::default()
    };
    let input = format!("<records>{}</records>", "<record/>".repeat(20));
    assert_eq!(
        s.stream_records(Cursor::new(input), &mut vec![], &options)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
}
#[test]
fn oversized_tokens_records_and_input_depth_fail_with_limits() {
    let s = sheet(r#"<xsl:template name="record"><xsl:value-of select="."/></xsl:template>"#);
    let cases = [
        (
            format!("<records><record>{}</record></records>", "x".repeat(200)),
            StreamOptions {
                max_event_bytes: 64,
                ..Default::default()
            },
        ),
        (
            format!("<records><record>{}</record></records>", "<n/>".repeat(100)),
            StreamOptions {
                max_record_bytes: 100,
                ..Default::default()
            },
        ),
        (
            "<records><record><n/></record></records>".into(),
            StreamOptions {
                max_input_depth: 2,
                ..Default::default()
            },
        ),
    ];
    for (input, options) in cases {
        assert_eq!(
            s.stream_records(Cursor::new(input), &mut vec![], &options)
                .unwrap_err()
                .code,
            "XPDY0130"
        );
    }
}
#[test]
fn validates_ignored_content_and_document_well_formedness() {
    let s = sheet(r#"<xsl:template name="record"/>"#);
    for input in [
        "<records><ignored a='1' a='2'/></records>",
        "<records><ignored>&unknown;</ignored></records>",
        "<records><ignored></different></records>",
        "<records/><other/>",
        "<records><ignored xmlns:p='urn:p' p:x='1' p:x='2'/></records>",
        "<records><p:ignored/></records>",
        "<records><ignored>\u{1}</ignored></records>",
        "<records>",
        "<records/>junk",
        "<!DOCTYPE records><records/>",
        "<records><!--bad--comment--></records>",
    ] {
        assert!(
            s.stream_records(Cursor::new(input), &mut vec![], &StreamOptions::default())
                .is_err(),
            "{input:?}"
        );
    }
}
#[test]
fn writer_failures_propagate_and_are_not_retried() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("broken sink"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let s = sheet(r#"<xsl:template name="record"><xsl:text>x</xsl:text></xsl:template>"#);
    assert_eq!(
        s.stream_records(
            Cursor::new("<records><record/></records>"),
            &mut Broken,
            &StreamOptions::default()
        )
        .unwrap_err()
        .code,
        "IO"
    );
}
#[test]
fn string_parameters_work_without_source_global_navigation() {
    let s = sheet(
        r#"<xsl:param name="prefix" select="'default'"/><xsl:template name="record"><xsl:value-of select="concat($prefix, @id)"/></xsl:template>"#,
    );
    let options = StreamOptions {
        parameters: [("prefix".into(), "#".into())].into(),
        ..Default::default()
    };
    let mut output = vec![];
    s.stream_records(
        Cursor::new("<records><record id='a'/><record id='b'/></records>"),
        &mut output,
        &options,
    )
    .unwrap();
    assert_eq!(output, b"#a#b");
    // The same named template is usable through the ordinary API.
    let options = TransformOptions {
        initial_template: Some("record".into()),
        ..Default::default()
    };
    assert_eq!(
        s.transform_with_options("<record/>", &options).unwrap(),
        "default"
    );
}

#[test]
fn generated_input_is_incremental_and_record_buffers_do_not_grow_with_count() {
    use std::{cell::Cell, rc::Rc};
    struct Generated {
        count: usize,
        consumed: Rc<Cell<usize>>,
        phase: u8,
        offset: usize,
    }
    impl Generated {
        fn chunk(&self) -> &'static [u8] {
            match self.phase {
                0 => b"<records>",
                1 => b"<record id='x'><title>hello</title></record>",
                2 => b"</records>",
                _ => b"",
            }
        }
    }
    impl Read for Generated {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            let current = self.fill_buf()?;
            let n = b.len().min(current.len());
            b[..n].copy_from_slice(&current[..n]);
            self.consume(n);
            Ok(n)
        }
    }
    impl BufRead for Generated {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Ok(&self.chunk()[self.offset..])
        }
        fn consume(&mut self, n: usize) {
            self.offset += n;
            if self.offset == self.chunk().len() {
                self.offset = 0;
                match self.phase {
                    0 => self.phase = 1,
                    1 => {
                        self.consumed.set(self.consumed.get() + 1);
                        if self.consumed.get() == self.count {
                            self.phase = 2;
                        }
                    }
                    2 => self.phase = 3,
                    _ => {}
                }
            }
        }
    }
    struct Sink {
        consumed: Rc<Cell<usize>>,
        total: usize,
        first: bool,
    }
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if !self.first {
                assert!(
                    self.consumed.get() < self.total,
                    "output began only after all input was consumed"
                );
                self.first = true;
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let s = sheet(r#"<xsl:template name="record"><xsl:value-of select="title"/></xsl:template>"#);
    let run = |count| {
        let consumed = Rc::new(Cell::new(0));
        let input = Generated {
            count,
            consumed: consumed.clone(),
            phase: 0,
            offset: 0,
        };
        let mut sink = Sink {
            consumed,
            total: count,
            first: false,
        };
        s.stream_records(input, &mut sink, &StreamOptions::default())
            .unwrap()
    };
    let small = run(100);
    let large = run(10_000);
    assert_eq!(large.records, 10_000);
    assert_eq!(large.output_bytes, 50_000);
    assert_eq!(small.peak_record_bytes, large.peak_record_bytes);
    assert_eq!(small.peak_event_bytes, large.peak_event_bytes);
    assert_eq!(small.peak_input_depth, large.peak_input_depth);
}

#[test]
fn aggregate_record_limits_include_xml_escaping() {
    let sheet = Stylesheet::compile(r#"<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:output encoding="UTF-8" omit-xml-declaration="yes"/><xsl:template name="record"><a>&amp;</a></xsl:template></xsl:stylesheet>"#).unwrap();
    let input = "<records><record/><record/></records>";
    let mut output = Vec::new();
    let options = StreamOptions {
        limits: rust_xslt::Limits {
            max_output_bytes: 22,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet
            .stream_records(Cursor::new(input), &mut output, &options)
            .unwrap_err()
            .code,
        "XPDY0130"
    );
    assert_eq!(output, b"<a>&amp;</a>");
    output.clear();
    let stats = sheet
        .stream_records(Cursor::new(input), &mut output, &StreamOptions::default())
        .unwrap();
    assert_eq!(output, b"<a>&amp;</a><a>&amp;</a>");
    assert_eq!(stats.output_bytes, output.len());
}
