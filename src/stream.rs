//! Record streaming follows go-xslt's "one grounded record, then discard"
//! approach. This is an explicit record API, not an XSLT streamability claim.
use crate::{
    Diagnostic, Stylesheet, TransformOptions,
    compile::{BindingValue, ComputedName, Content, Instruction},
    runtime,
    xpath::{self, Axis, Budget, Expr, Name, Test},
};
use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, BufRead, Read, Write},
};

/// Limits and entry point for fixed-depth, non-overlapping record streaming.
#[derive(Debug, Clone)]
pub struct StreamOptions {
    /// Absolute child-name path, for example `/catalog/book`.
    pub record_path: String,
    /// Prefix bindings for the record path; unprefixed names have no namespace.
    pub namespaces: BTreeMap<String, String>,
    /// Invoked once per record, with that element as context and position() as
    /// its one-based index. last() is unavailable because input is incremental.
    pub template: String,
    pub parameters: BTreeMap<String, String>,
    pub typed_parameters: BTreeMap<String, crate::ParameterValue>,
    pub initial_parameters: BTreeMap<String, crate::ParameterValue>,
    pub initial_tunnel_parameters: BTreeMap<String, crate::ParameterValue>,
    pub limits: crate::Limits,
    pub max_record_bytes: usize,
    pub max_event_bytes: usize,
    pub max_input_depth: usize,
}
impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            record_path: "/records/record".into(),
            namespaces: BTreeMap::new(),
            template: "record".into(),
            parameters: BTreeMap::new(),
            typed_parameters: BTreeMap::new(),
            initial_parameters: BTreeMap::new(),
            initial_tunnel_parameters: BTreeMap::new(),
            limits: crate::Limits::default(),
            max_record_bytes: 8 * 1024 * 1024,
            max_event_bytes: 1024 * 1024,
            max_input_depth: 128,
        }
    }
}
/// Observable buffering statistics, excluding caller-owned input/output buffers.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StreamStats {
    pub records: usize,
    pub output_bytes: usize,
    pub peak_record_bytes: usize,
    pub peak_event_bytes: usize,
    pub peak_input_depth: usize,
}
fn unsupported(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("RUST_XSLT_STREAM_UNSUPPORTED", message, "compile")
}
fn parse_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("FODC0006", message, "parse")
}
fn io_error(e: io::Error) -> Diagnostic {
    Diagnostic::new("IO", e.to_string(), "run")
}
fn limit(message: &str) -> Diagnostic {
    Diagnostic::new("XPDY0130", message, "run")
}
fn safe_expression(e: &Expr, global: bool) -> Result<(), Diagnostic> {
    let mut pending = vec![e];
    while let Some(e) = pending.pop() {
        match e {
            Expr::Root => return Err(unsupported("absolute paths can escape a streamed record")),
            Expr::Context if global => {
                return Err(unsupported(
                    "streamed globals cannot depend on the source context",
                ));
            }
            Expr::Path(base, steps) => {
                if steps.iter().any(|s| {
                    !matches!(
                        s.axis,
                        Axis::Child
                            | Axis::Attribute
                            | Axis::SelfNode
                            | Axis::Descendant
                            | Axis::DescendantOrSelf
                    )
                }) {
                    return Err(unsupported(
                        "nonlocal axis navigation can escape a streamed record",
                    ));
                }
                pending.push(base);
                for s in steps {
                    pending.extend(&s.predicates);
                }
            }
            Expr::Filter(base, predicates) => {
                pending.push(base);
                pending.extend(predicates);
            }
            Expr::If(c, y, n) => {
                pending.push(c);
                pending.push(y);
                pending.push(n);
            }
            Expr::Unary(_, e) | Expr::TypeOp(_, e, _) | Expr::QNameCast(e, _, _, _) => {
                pending.push(e)
            }
            Expr::For(_, select, body)
            | Expr::Let(_, select, body)
            | Expr::Quantified(_, _, select, body) => {
                pending.push(select);
                pending.push(body);
            }
            Expr::Binary(_, a, b) | Expr::PathMap(a, b) => {
                pending.push(a);
                pending.push(b);
            }
            Expr::Sequence(es) => pending.extend(es),
            Expr::Call(name, args) => {
                if name == "generate-id" {
                    return Err(unsupported(
                        "generate-id() requires node identities across streamed records",
                    ));
                }
                if name == "key" {
                    return Err(unsupported("key() searches beyond the streamed record"));
                }
                if name == "lang" {
                    return Err(unsupported(
                        "lang() can depend on ancestors outside the streamed record",
                    ));
                }
                if name == "root" {
                    return Err(unsupported("root() can escape the streamed record focus"));
                }
                if name == "last" {
                    return Err(unsupported(
                        "last() is not available in the streamed record focus",
                    ));
                }
                if global && (args.is_empty() && !["true", "false"].contains(&name.as_str())) {
                    return Err(unsupported(
                        "streamed global uses an implicit context or focus",
                    ));
                }
                pending.extend(args);
            }
            _ => {}
        }
    }
    Ok(())
}
fn safe_name(n: &ComputedName) -> Result<(), Diagnostic> {
    for e in &n.parts {
        safe_expression(e, false)?;
    }
    if let Some(es) = &n.namespace {
        for e in es {
            safe_expression(e, false)?;
        }
    }
    Ok(())
}
fn safe_content<'a>(c: &'a Content, bodies: &mut Vec<&'a [Instruction]>) -> Result<(), Diagnostic> {
    match c {
        Content::Select(e, sep) => {
            safe_expression(e, false)?;
            for e in sep {
                safe_expression(e, false)?;
            }
        }
        Content::Body(body, sep) => {
            for e in sep {
                safe_expression(e, false)?;
            }
            bodies.push(body);
        }
    };
    Ok(())
}
fn safe_binding<'a>(
    value: &'a BindingValue,
    global: bool,
    bodies: &mut Vec<&'a [Instruction]>,
) -> Result<(), Diagnostic> {
    match value {
        BindingValue::Select(e) => safe_expression(e, global)?,
        BindingValue::Body(body) | BindingValue::Document(body) | BindingValue::Raw(body) => {
            if global {
                safe_global_body(body)?;
            } else {
                bodies.push(body);
            }
        }
    }
    Ok(())
}
fn safe_global_body(body: &[Instruction]) -> Result<(), Diagnostic> {
    fn name(n: &ComputedName) -> Result<(), Diagnostic> {
        for e in &n.parts {
            safe_expression(e, true)?;
        }
        if let Some(es) = &n.namespace {
            for e in es {
                safe_expression(e, true)?;
            }
        }
        Ok(())
    }
    fn content(c: &Content) -> Result<(), Diagnostic> {
        match c {
            Content::Select(e, sep) => {
                safe_expression(e, true)?;
                for e in sep {
                    safe_expression(e, true)?;
                }
            }
            Content::Body(body, sep) => {
                for e in sep {
                    safe_expression(e, true)?;
                }
                safe_global_body(body)?;
            }
        }
        Ok(())
    }
    for i in body {
        match i {
            Instruction::Text(_) => {}
            Instruction::TextValue(_) => {
                return Err(unsupported(
                    "text value templates are not supported when streaming",
                ));
            }
            Instruction::Value(e, sep) => {
                safe_expression(e, true)?;
                if let Some(es) = sep {
                    for e in es {
                        safe_expression(e, true)?;
                    }
                }
            }
            Instruction::Sequence(e) | Instruction::CopyOf(e, _) => safe_expression(e, true)?,
            Instruction::Bind(b) => safe_binding(&b.value, true, &mut vec![])?,
            Instruction::Element {
                attributes, body, ..
            } => {
                for (_, parts) in attributes {
                    for e in parts {
                        safe_expression(e, true)?;
                    }
                }
                safe_global_body(body)?;
            }
            Instruction::AnalyzeString { .. } => {
                return Err(unsupported(
                    "xsl:analyze-string is not supported when streaming",
                ));
            }
            Instruction::Iterate { .. } | Instruction::NextIteration(_) | Instruction::Break(_) => {
                return Err(unsupported("xsl:iterate is not supported when streaming"));
            }
            Instruction::Try { .. } => {
                return Err(unsupported("xsl:try is not supported when streaming"));
            }
            Instruction::Number(_) => {
                return Err(unsupported("xsl:number is not supported when streaming"));
            }
            Instruction::WherePopulated(_) | Instruction::Namespace(..) => {
                return Err(unsupported(
                    "this instruction is not supported when streaming",
                ));
            }
            Instruction::ResultDocument(_) => {
                return Err(unsupported(
                    "xsl:result-document is not supported when streaming",
                ));
            }
            Instruction::OnEmpty(_)
            | Instruction::OnNonEmpty(_)
            | Instruction::Populated { .. } => {
                return Err(unsupported("xsl:on-empty is not supported when streaming"));
            }
            Instruction::UseSets(_) => {
                return Err(unsupported(
                    "attribute sets are not supported when streaming",
                ));
            }
            Instruction::LiteralAttributes(attributes) => {
                for (_, parts) in attributes {
                    for e in parts {
                        safe_expression(e, true)?;
                    }
                }
            }
            Instruction::ComputedElement(n, _, body) => {
                name(n)?;
                safe_global_body(body)?;
            }
            Instruction::Attribute(n, c) => {
                name(n)?;
                content(c)?;
            }
            Instruction::Comment(c)
            | Instruction::Message(c, _)
            | Instruction::AssertFailure(c) => content(c)?,
            Instruction::Pi(es, c) => {
                for e in es {
                    safe_expression(e, true)?;
                }
                content(c)?;
            }
            Instruction::ForEachGroup(..) => {
                return Err(unsupported(
                    "grouping requires buffering the whole sequence",
                ));
            }
            Instruction::If(e, body) | Instruction::ForEach(e, _, body) => {
                safe_expression(e, true)?;
                safe_global_body(body)?;
            }
            Instruction::Choose(bs) => {
                for (e, body) in bs {
                    if let Some(e) = e {
                        safe_expression(e, true)?;
                    }
                    safe_global_body(body)?;
                }
            }
            Instruction::Document(body) => safe_global_body(body)?,
            Instruction::Copy {
                select: Some(e),
                body,
                ..
            } => {
                safe_expression(e, true)?;
                safe_global_body(body)?;
            }
            Instruction::Apply(..)
            | Instruction::Call(..)
            | Instruction::NextMatch(..)
            | Instruction::ApplyImports(..)
            | Instruction::Copy { .. } => {
                return Err(unsupported(
                    "streamed global constructors cannot invoke templates or copy the source context",
                ));
            }
        }
    }
    Ok(())
}
fn validate(sheet: &Stylesheet, options: &StreamOptions) -> Result<Vec<Name>, Diagnostic> {
    let path = xpath::parse(&options.record_path, options.namespaces.clone())?;
    let Expr::Path(base, steps) = path else {
        return Err(unsupported(
            "record path must be an absolute child-name path",
        ));
    };
    if !matches!(*base, Expr::Root) || steps.is_empty() {
        return Err(unsupported("record path must be absolute and nonempty"));
    }
    let mut names = vec![];
    for s in steps {
        if !matches!(s.axis, Axis::Child) || !s.predicates.is_empty() {
            return Err(unsupported(
                "record path must contain only child-name steps",
            ));
        }
        let Test::Name(n) = s.test else {
            return Err(unsupported("record path requires element names"));
        };
        names.push(n);
    }
    let entry_name = crate::qname::external(&options.template)?;
    let template = sheet
        .templates
        .iter()
        .find(|t| t.name.as_ref() == Some(&entry_name))
        .ok_or_else(|| unsupported("unknown record template"))?;
    for b in &sheet.globals {
        safe_binding(&b.value, true, &mut vec![])?;
    }
    let mut pending = vec![template];
    let mut visited = BTreeSet::new();
    while let Some(t) = pending.pop() {
        if !visited.insert(std::ptr::from_ref(t)) {
            continue;
        }
        let mut bodies = vec![t.body.as_slice()];
        for p in &t.params {
            safe_binding(&p.value, false, &mut bodies)?;
        }
        while let Some(body) = bodies.pop() {
            for i in body {
                match i {
                    Instruction::Value(_, Some(parts)) => {
                        for e in parts {
                            safe_expression(e, false)?;
                        }
                    }
                    Instruction::Apply(_, _, keys, _) | Instruction::ForEach(_, keys, _) => {
                        for key in keys {
                            safe_expression(&key.select, false)?;
                        }
                    }
                    _ => {}
                }
                match i {
                    Instruction::ComputedElement(n, _, body) => {
                        safe_name(n)?;
                        bodies.push(body);
                    }
                    Instruction::Attribute(n, c) => {
                        safe_name(n)?;
                        safe_content(c, &mut bodies)?;
                    }
                    Instruction::Comment(c)
                    | Instruction::Message(c, _)
                    | Instruction::AssertFailure(c) => safe_content(c, &mut bodies)?,
                    Instruction::Pi(parts, c) => {
                        for e in parts {
                            safe_expression(e, false)?;
                        }
                        safe_content(c, &mut bodies)?;
                    }
                    Instruction::Text(_) => {}
                    Instruction::TextValue(_) => {
                        return Err(unsupported(
                            "text value templates are not supported when streaming",
                        ));
                    }
                    Instruction::Value(e, _)
                    | Instruction::CopyOf(e, _)
                    | Instruction::Sequence(e) => safe_expression(e, false)?,
                    Instruction::Copy { select, body, .. } => {
                        if let Some(e) = select {
                            safe_expression(e, false)?;
                        }
                        bodies.push(body);
                    }
                    Instruction::Document(body) => bodies.push(body),
                    Instruction::Bind(b) => safe_binding(&b.value, false, &mut bodies)?,
                    Instruction::Apply(_, _, _, params)
                    | Instruction::NextMatch(params)
                    | Instruction::ApplyImports(params) => {
                        for p in params {
                            safe_binding(&p.value, false, &mut bodies)?;
                        }
                        if let Instruction::Apply(e, ..) = i {
                            safe_expression(e, false)?;
                        }
                        for template in &sheet.templates {
                            if let Some(branches) = &template.pattern {
                                if branches.iter().any(|b| {
                                    b.predicate
                                        .as_ref()
                                        .is_some_and(|predicates| !predicates.is_empty())
                                        || b.selected.is_some()
                                        || !b.required.is_empty()
                                        || !b.excluded.is_empty()
                                        || b.path_map.is_some()
                                        || !b.steps.is_empty()
                                            && (b.absolute
                                                || b.steps.len() != 1
                                                || b.selected.is_some()
                                                || b.gaps.iter().any(|g| *g)
                                                || !b.steps[0].predicates.is_empty())
                                }) {
                                    return Err(unsupported(
                                        "streamed dispatch currently requires single-step, predicate-free patterns",
                                    ));
                                }
                                pending.push(template);
                            }
                        }
                    }
                    Instruction::Element {
                        attributes, body, ..
                    } => {
                        for (_, parts) in attributes {
                            for e in parts {
                                safe_expression(e, false)?;
                            }
                        }
                        bodies.push(body);
                    }
                    Instruction::AnalyzeString { .. } => {
                        return Err(unsupported(
                            "xsl:analyze-string is not supported when streaming",
                        ));
                    }
                    Instruction::Iterate { .. }
                    | Instruction::NextIteration(_)
                    | Instruction::Break(_) => {
                        return Err(unsupported("xsl:iterate is not supported when streaming"));
                    }
                    Instruction::Try { .. } => {
                        return Err(unsupported("xsl:try is not supported when streaming"));
                    }
                    Instruction::Number(_) => {
                        return Err(unsupported("xsl:number is not supported when streaming"));
                    }
                    Instruction::WherePopulated(_) | Instruction::Namespace(..) => {
                        return Err(unsupported(
                            "this instruction is not supported when streaming",
                        ));
                    }
                    Instruction::ResultDocument(_) => {
                        return Err(unsupported(
                            "xsl:result-document is not supported when streaming",
                        ));
                    }
                    Instruction::OnEmpty(_)
                    | Instruction::OnNonEmpty(_)
                    | Instruction::Populated { .. } => {
                        return Err(unsupported("xsl:on-empty is not supported when streaming"));
                    }
                    Instruction::UseSets(_) => {
                        return Err(unsupported(
                            "attribute sets are not supported when streaming",
                        ));
                    }
                    Instruction::LiteralAttributes(attributes) => {
                        for (_, parts) in attributes {
                            for e in parts {
                                safe_expression(e, false)?;
                            }
                        }
                    }
                    Instruction::ForEachGroup(..) => {
                        return Err(unsupported(
                            "grouping requires buffering the whole sequence",
                        ));
                    }
                    Instruction::ForEach(e, _, body) | Instruction::If(e, body) => {
                        safe_expression(e, false)?;
                        bodies.push(body);
                    }
                    Instruction::Choose(branches) => {
                        for (e, body) in branches {
                            if let Some(e) = e {
                                safe_expression(e, false)?;
                            }
                            bodies.push(body);
                        }
                    }
                    Instruction::Call(name, params) => {
                        for p in params {
                            safe_binding(&p.value, false, &mut bodies)?;
                        }
                        pending.push(
                            sheet
                                .templates
                                .iter()
                                .find(|t| t.name.as_ref() == Some(name))
                                .ok_or_else(|| unsupported("unknown called template"))?,
                        );
                    }
                }
            }
        }
    }
    Ok(names)
}
/// Bounds allocations inside the XML pull parser before it can buffer a huge token.
struct EventInput<R> {
    inner: R,
    used: usize,
    max: usize,
}
impl<R: BufRead> Read for EventInput<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let bytes = self.fill_buf()?;
        let n = out.len().min(bytes.len());
        out[..n].copy_from_slice(&bytes[..n]);
        self.consume(n);
        Ok(n)
    }
}
impl<R: BufRead> BufRead for EventInput<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.used >= self.max {
            return Err(io::Error::other("XML event byte limit exceeded"));
        }
        let b = self.inner.fill_buf()?;
        Ok(&b[..b.len().min(self.max - self.used)])
    }
    fn consume(&mut self, n: usize) {
        self.used += n;
        self.inner.consume(n);
    }
}
struct RecordBuffer {
    bytes: Vec<u8>,
    max: usize,
}
impl Write for RecordBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.max.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("streamed record byte limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn complete_start(
    start: &BytesStart<'_>,
    parent: &BTreeMap<String, String>,
) -> Result<BytesStart<'static>, Diagnostic> {
    let mut declared = BTreeSet::new();
    for a in start.attributes() {
        let a = a.map_err(|e| parse_error(e.to_string()))?;
        let n = std::str::from_utf8(a.key.as_ref()).map_err(|e| parse_error(e.to_string()))?;
        if n == "xmlns" {
            declared.insert(String::new());
        } else if let Some(p) = n.strip_prefix("xmlns:") {
            declared.insert(p.into());
        }
    }
    let mut complete = start.to_owned();
    for (p, uri) in parent {
        if p != "xml" && !declared.contains(p) {
            let key = if p.is_empty() {
                "xmlns".into()
            } else {
                format!("xmlns:{p}")
            };
            let value = quick_xml::escape::escape(uri);
            complete.push_attribute((key.as_str(), value.as_ref()));
        }
    }
    Ok(complete)
}
struct StartInfo {
    name: Name,
    namespaces: BTreeMap<String, String>,
    complete: BytesStart<'static>,
    preserve: Option<bool>,
}
fn validate_start(
    start: &BytesStart<'_>,
    inherited: &BTreeMap<String, String>,
) -> Result<StartInfo, Diagnostic> {
    let complete = complete_start(start, inherited)?;
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Empty(complete.borrow()))
        .map_err(io_error)?;
    let bytes = writer.into_inner();
    let xml = std::str::from_utf8(&bytes).map_err(|e| parse_error(e.to_string()))?;
    let doc = crate::parse(xml, "parse")?;
    let root = doc.root_element();
    let name = Name {
        uri: root
            .tag_name()
            .namespace()
            .filter(|uri| !uri.is_empty())
            .map(str::to_owned),
        local: root.tag_name().name().into(),
    };
    let namespaces = root
        .namespaces()
        .map(|ns| (ns.name().unwrap_or("").into(), ns.uri().into()))
        .collect();
    let preserve = root
        .attribute(("http://www.w3.org/XML/1998/namespace", "space"))
        .map(|s| s == "preserve");
    Ok(StartInfo {
        name,
        namespaces,
        complete,
        preserve,
    })
}
fn validate_leaf(event: &Event<'_>) -> Result<(), Diagnostic> {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Start(BytesStart::new("_validate")))
        .map_err(io_error)?;
    writer.write_event(event.borrow()).map_err(io_error)?;
    writer
        .write_event(Event::End(quick_xml::events::BytesEnd::new("_validate")))
        .map_err(io_error)?;
    let bytes = writer.into_inner();
    let xml = std::str::from_utf8(&bytes).map_err(|e| parse_error(e.to_string()))?;
    crate::parse(xml, "parse")?;
    Ok(())
}
fn write_output<W: Write>(
    output: &mut W,
    text: &[u8],
    stats: &mut StreamStats,
    max: usize,
) -> Result<(), Diagnostic> {
    if text.len() > max.saturating_sub(stats.output_bytes) {
        return Err(limit("aggregate output byte limit exceeded"));
    }
    output.write_all(text).map_err(io_error)?;
    stats.output_bytes += text.len();
    Ok(())
}
pub(crate) fn transform<R: BufRead, W: Write>(
    sheet: &Stylesheet,
    input: R,
    output: &mut W,
    options: &StreamOptions,
) -> Result<StreamStats, Diagnostic> {
    let path = validate(sheet, options)?;
    let mut budget = Budget::new(options.limits);
    let mut reader = Reader::from_reader(EventInput {
        inner: input,
        used: 0,
        max: options.max_event_bytes,
    });
    reader.config_mut().check_comments = true;
    let mut stack: Vec<(Name, BTreeMap<String, String>, bool)> = vec![];
    let mut event_buffer = vec![];
    let mut record: Option<Writer<RecordBuffer>> = None;
    if sheet.encoding != crate::result::Encoding::Utf8 {
        return Err(Diagnostic::new(
            "RUST_XSLT_UNSUPPORTED",
            "record streaming only writes UTF-8",
            "compile",
        ));
    }
    let mut stats = StreamStats::default();
    let mut record_preserve = false;
    let mut roots = 0;
    let mut declaration = false;
    if sheet.byte_order_mark {
        write_output(
            output,
            "\u{feff}".as_bytes(),
            &mut stats,
            options.limits.max_output_bytes,
        )?;
    }
    if !sheet.text_output
        && sheet.markup != crate::result::Markup::Html
        && let Some(header) = crate::result::xml_declaration(
            sheet.encoding,
            sheet.omit_declaration,
            sheet.standalone,
        )?
    {
        write_output(
            output,
            header.as_bytes(),
            &mut stats,
            options.limits.max_output_bytes,
        )?;
    }
    loop {
        budget.tick()?;
        reader.get_mut().used = 0;
        let event = reader.read_event_into(&mut event_buffer).map_err(|e| {
            if e.to_string().contains("XML event byte limit") {
                limit("XML event byte limit exceeded")
            } else {
                parse_error(e.to_string())
            }
        })?;
        stats.peak_event_bytes = stats.peak_event_bytes.max(reader.get_mut().used);
        match &event {
            Event::Start(start) | Event::Empty(start) => {
                if stack.is_empty() {
                    roots += 1;
                    if roots > 1 {
                        return Err(parse_error("multiple document elements"));
                    }
                }
                if stack.len() >= options.max_input_depth {
                    return Err(limit("input XML depth limit exceeded"));
                }
                let inherited = stack
                    .last()
                    .map(|(_, ns, _)| ns.clone())
                    .unwrap_or_default();
                let inherited_preserve = stack.last().is_some_and(|(_, _, p)| *p);
                let StartInfo {
                    name,
                    namespaces,
                    complete,
                    preserve,
                } = validate_start(start, &inherited)?;
                let preserve = preserve.unwrap_or(inherited_preserve);
                let selected = stack.len() + 1 == path.len()
                    && stack.iter().zip(&path).all(|((n, _, _), p)| n == p)
                    && path.last() == Some(&name);
                if selected {
                    record_preserve = inherited_preserve;
                    record = Some(Writer::new(RecordBuffer {
                        bytes: vec![],
                        max: options.max_record_bytes,
                    }));
                    let e = if matches!(event, Event::Empty(_)) {
                        Event::Empty(complete)
                    } else {
                        Event::Start(complete)
                    };
                    record
                        .as_mut()
                        .unwrap()
                        .write_event(e)
                        .map_err(|_| limit("streamed record byte limit exceeded"))?;
                } else if let Some(record) = &mut record {
                    record
                        .write_event(event.borrow())
                        .map_err(|_| limit("streamed record byte limit exceeded"))?;
                }
                stats.peak_input_depth = stats.peak_input_depth.max(stack.len() + 1);
                if matches!(event, Event::Start(_)) {
                    stack.push((name, namespaces, preserve));
                }
            }
            Event::End(_) => {
                if stack.pop().is_none() {
                    return Err(parse_error("unexpected end element"));
                }
                if let Some(record) = &mut record {
                    record
                        .write_event(event.borrow())
                        .map_err(|_| limit("streamed record byte limit exceeded"))?;
                }
            }
            Event::DocType(_) => return Err(parse_error("DTDs are disabled")),
            Event::Decl(d) => {
                if declaration || roots > 0 || !stack.is_empty() {
                    return Err(parse_error("misplaced XML declaration"));
                }
                declaration = true;
                if d.version()
                    .map_err(|e| parse_error(e.to_string()))?
                    .as_ref()
                    != b"1.0"
                {
                    return Err(unsupported("streaming currently supports XML 1.0 only"));
                }
                if let Some(encoding) = d.encoding()
                    && !encoding
                        .map_err(|e| parse_error(e.to_string()))?
                        .eq_ignore_ascii_case(b"UTF-8")
                {
                    return Err(unsupported("streaming currently supports UTF-8 only"));
                }
            }
            Event::Eof => {
                if !stack.is_empty() || roots != 1 {
                    return Err(parse_error("incomplete XML document"));
                }
                break;
            }
            _ => {
                validate_leaf(&event)?;
                if stack.is_empty() {
                    match &event {
                        Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => {}
                        Event::Comment(_) | Event::PI(_) => {}
                        _ => return Err(parse_error("character data outside document element")),
                    }
                }
                if let Some(record) = &mut record {
                    record
                        .write_event(event.borrow())
                        .map_err(|_| limit("streamed record byte limit exceeded"))?;
                }
            }
        }
        if let Some(buffer) = &record {
            stats.peak_record_bytes = stats.peak_record_bytes.max(buffer.get_ref().bytes.len());
        }
        if record.is_some() && stack.len() < path.len() {
            let bytes = record.take().unwrap().into_inner().bytes;
            let xml = std::str::from_utf8(&bytes).map_err(|e| parse_error(e.to_string()))?;
            let remaining = options
                .limits
                .max_output_bytes
                .saturating_sub(stats.output_bytes);
            let transform = TransformOptions {
                parameters: options.parameters.clone(),
                typed_parameters: options.typed_parameters.clone(),
                initial_parameters: options.initial_parameters.clone(),
                initial_tunnel_parameters: options.initial_tunnel_parameters.clone(),
                initial_template: Some(options.template.clone()),
                initial_mode: None,
                limits: crate::Limits {
                    max_output_bytes: remaining,
                    ..options.limits
                },
                resolver: None,
            };
            let text = runtime::transform_record(
                sheet,
                xml,
                &transform,
                &mut budget,
                stats.records + 1,
                record_preserve,
            )?;
            write_output(
                output,
                text.as_bytes(),
                &mut stats,
                options.limits.max_output_bytes,
            )?;
            stats.records += 1;
        }
        event_buffer.clear();
    }
    Ok(stats)
}
