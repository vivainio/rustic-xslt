use crate::whitespace::xml_space;
use crate::{
    Diagnostic, Stylesheet,
    xpath::{self, Expr},
};
use roxmltree::Node;
use std::collections::{BTreeMap, BTreeSet};
pub(crate) const XSL: &str = "http://www.w3.org/1999/XSL/Transform";

#[derive(Debug)]
pub(crate) enum Instruction {
    Text(String),
    /// Text with `{expression}` value templates (expand-text).
    TextValue(Vec<Expr>),
    Value(Expr, Option<Vec<Expr>>),
    Apply(Expr, String, Vec<Sort>, Vec<Binding>),
    Copy {
        select: Option<Expr>,
        namespaces: bool,
        inherit: bool,
        body: Vec<Instruction>,
    },
    CopyOf(Expr, bool),
    Document(Vec<Instruction>),
    Sequence(Expr),
    ComputedElement(ComputedName, bool, Vec<Instruction>),
    Attribute(ComputedName, Content),
    Comment(Content),
    Message(Content, bool),
    /// The failing branch of `xsl:assert`: raises XTMM9001 with the message text.
    AssertFailure(Content),
    Pi(Vec<Expr>, Content),
    Element {
        element: crate::result::Element,
        attributes: Vec<(crate::result::Name, Vec<Expr>)>,
        body: Vec<Instruction>,
    },
    ForEach(Expr, Vec<Sort>, Vec<Instruction>),
    ForEachGroup(Expr, Grouping, Vec<Sort>, Vec<Instruction>),
    If(Expr, Vec<Instruction>),
    Choose(Vec<(Option<Expr>, Vec<Instruction>)>),
    Bind(Binding),
    Call(String, Vec<Binding>),
    NextMatch(Vec<Binding>),
    ApplyImports(Vec<Binding>),
    UseSets(Vec<String>),
    Number(Box<NumberSpec>),
    Iterate {
        select: Expr,
        params: Vec<Binding>,
        on_completion: Vec<Instruction>,
        body: Vec<Instruction>,
    },
    NextIteration(Vec<Binding>),
    Try {
        body: Vec<Instruction>,
        catches: Vec<Catch>,
    },
    WherePopulated(Vec<Instruction>),
    /// `xsl:on-empty` / `xsl:on-non-empty` markers, consumed when the body is assembled.
    OnEmpty(Vec<Instruction>),
    OnNonEmpty(Vec<Instruction>),
    /// A sequence constructor with conditional fallbacks: the main part runs first.
    Populated {
        main: Vec<Instruction>,
        on_empty: Vec<Instruction>,
        on_non_empty: Vec<Instruction>,
    },
    ResultDocument(Box<ResultDocumentSpec>),
    Namespace(Vec<Expr>, Content),
    Break(Vec<Instruction>),
    AnalyzeString {
        select: Expr,
        regex: Vec<Expr>,
        flags: Vec<Expr>,
        matching: Vec<Instruction>,
        non_matching: Vec<Instruction>,
    },
    LiteralAttributes(Vec<(crate::result::Name, Vec<Expr>)>),
}
/// An `xsl:catch` clause; `errors` holds (namespace, local) name tests where `None` is a wildcard.
#[derive(Debug)]
pub(crate) struct Catch {
    pub errors: Vec<(Option<String>, Option<String>)>,
    pub body: Vec<Instruction>,
}
/// A secondary result created by `xsl:result-document`.
#[derive(Debug)]
pub(crate) struct ResultDocumentSpec {
    pub href: Vec<Expr>,
    pub method: Option<Vec<Expr>>,
    pub format: Option<(Vec<Expr>, BTreeMap<String, String>)>,
    pub omit_declaration: Option<Vec<Expr>>,
    pub standalone: Option<Vec<Expr>>,
    pub byte_order_mark: Option<Vec<Expr>>,
    pub indent: Option<Vec<Expr>>,
    pub escape_uri_attributes: Option<Vec<Expr>>,
    pub include_content_type: Option<Vec<Expr>>,
    pub media_type: Option<Vec<Expr>>,
    pub encoding: Option<Vec<Expr>>,
    pub output_version: Option<Vec<Expr>>,
    pub cdata: Option<(Vec<Expr>, BTreeMap<String, String>)>,
    pub character_maps: Vec<String>,
    pub doctype_public: Option<Vec<Expr>>,
    pub doctype_system: Option<Vec<Expr>>,
    pub body: Vec<Instruction>,
}
pub(crate) const ERROR_NAMESPACE: &str = "http://www.w3.org/2005/xqt-errors";
pub(crate) const ERROR_VARIABLES: [&str; 7] = [
    "code",
    "description",
    "value",
    "module",
    "line-number",
    "column-number",
    "additional",
];
#[derive(Debug, Default)]
pub(crate) struct AttributeSet {
    pub uses: Vec<String>,
    pub body: Vec<Instruction>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum NumberLevel {
    Single,
    Multiple,
    Any,
}
#[derive(Debug)]
pub(crate) struct NumberSpec {
    pub value: Option<Expr>,
    pub select: Option<Expr>,
    pub level: NumberLevel,
    pub count: Option<Vec<crate::pattern::Branch>>,
    pub from: Option<Vec<crate::pattern::Branch>>,
    pub format: Vec<Expr>,
    pub ordinal: Vec<Expr>,
    pub grouping_separator: Vec<Expr>,
    pub grouping_size: Vec<Expr>,
}
#[derive(Debug)]
pub(crate) enum Grouping {
    By(Expr),
    Adjacent(Expr),
    Starting(Vec<crate::pattern::Branch>),
    Ending(Vec<crate::pattern::Branch>),
}
/// Sort settings that are attribute value templates, resolved when the sort runs.
#[derive(Debug, Clone, Default)]
pub(crate) struct DynamicSort {
    pub data_type: Option<Vec<Expr>>,
    pub order: Option<Vec<Expr>>,
    pub case_order: Option<Vec<Expr>>,
    pub collation: Option<Vec<Expr>>,
}
#[derive(Debug, Clone)]
pub(crate) struct Sort {
    pub dynamic: DynamicSort,
    pub collation: Option<crate::collation::Collation>,
    pub select: Expr,
    pub numeric: bool,
    pub descending: bool,
    pub case_order: String,
}
#[derive(Debug)]
pub(crate) struct ComputedName {
    pub parts: Vec<Expr>,
    pub namespace: Option<Vec<Expr>>,
    pub bindings: BTreeMap<String, String>,
}
#[derive(Debug)]
pub(crate) enum Content {
    Select(Expr, Vec<Expr>),
    Body(Vec<Instruction>, Vec<Expr>),
}
#[derive(Debug)]
pub(crate) enum BindingValue {
    Select(Expr),
    Body(Vec<Instruction>),
    Raw(Vec<Instruction>),
    Document(Vec<Instruction>),
}
#[derive(Debug)]
pub(crate) struct Binding {
    pub name: String,
    pub value: BindingValue,
    pub parameter: bool,
    pub required: bool,
    pub implicit_required: bool,
    pub tunnel: bool,
    pub ty: Option<crate::seqtype::SequenceType>,
}
#[derive(Debug)]
pub(crate) struct Template {
    pub pattern: Option<Vec<crate::pattern::Branch>>,
    pub modes: Vec<String>,
    pub priority: Option<f64>,
    pub name: Option<String>,
    pub params: Vec<Binding>,
    pub result_type: Option<crate::seqtype::SequenceType>,
    pub body: Vec<Instruction>,
    /// Import precedence: declarations of importing modules outrank imported ones.
    pub precedence: u32,
}
fn error(n: Node<'_, '_>, message: impl Into<String>) -> Diagnostic {
    let pos = n.document().text_pos_at(n.range().start);
    Diagnostic {
        code: "RUST_XSLT_UNSUPPORTED".into(),
        message: message.into(),
        phase: "compile",
        line: pos.row,
        column: pos.col,
    }
}
fn xslt_version(n: Node<'_, '_>) -> u8 {
    match n.document().root_element().attribute("version") {
        Some("1.0") => 1,
        Some("2.0") => 2,
        _ => 3,
    }
}
fn static_error(n: Node<'_, '_>, code: &str, message: impl Into<String>) -> Diagnostic {
    let mut e = error(n, message);
    e.code = code.into();
    e
}
fn component_name(n: Node<'_, '_>, text: &str) -> Result<String, Diagnostic> {
    // Unlike XPath's built-in function/type prefixes, component prefixes must
    // actually be declared in the stylesheet.
    let mut bindings: BTreeMap<String, String> = n
        .namespaces()
        .filter_map(|ns| ns.name().map(|p| (p.into(), ns.uri().into())))
        .collect();
    bindings.insert("xml".into(), "http://www.w3.org/XML/1998/namespace".into());
    crate::qname::resolve(text.trim(), &bindings, "XTSE0020").map_err(|mut e| {
        let pos = n.document().text_pos_at(n.range().start);
        e.line = pos.row;
        e.column = pos.col;
        e
    })
}
fn check_component_namespace(
    n: Node<'_, '_>,
    name: &str,
    initial_template: bool,
) -> Result<(), Diagnostic> {
    let (uri, local) = crate::qname::parts(name);
    if initial_template && uri == Some(XSL) && local == "initial-template" {
        return Ok(());
    }
    if matches!(
        uri,
        Some(
            XSL | "http://www.w3.org/2005/xpath-functions"
                | "http://www.w3.org/2005/xpath-functions/math"
                | "http://www.w3.org/2005/xpath-functions/map"
                | "http://www.w3.org/2005/xpath-functions/array"
                | "http://www.w3.org/XML/1998/namespace"
                | "http://www.w3.org/2001/XMLSchema"
                | "http://www.w3.org/2001/XMLSchema-instance"
                | "http://www.w3.org/2005/xqt-errors"
                | "http://www.w3.org/2000/xmlns/"
        )
    ) {
        return Err(static_error(
            n,
            "XTSE0080",
            format!("reserved component namespace: {name}"),
        ));
    }
    Ok(())
}
fn required<'a>(n: Node<'a, '_>, a: &str) -> Result<&'a str, Diagnostic> {
    n.attribute(a)
        .ok_or_else(|| error(n, format!("missing required attribute: {a}")))
}
fn namespaces(n: Node<'_, '_>) -> BTreeMap<String, String> {
    let mut namespaces: BTreeMap<_, _> = n
        .namespaces()
        .filter_map(|ns| ns.name().map(|prefix| (prefix.into(), ns.uri().into())))
        .collect();
    if let Some(uri) = n.ancestors().find_map(|ancestor| {
        if ancestor.tag_name().namespace() == Some(XSL) {
            ancestor.attribute("xpath-default-namespace")
        } else {
            ancestor.attribute((XSL, "xpath-default-namespace"))
        }
    }) {
        namespaces.insert(
            crate::qname::XPATH_DEFAULT_NAMESPACE.into(),
            crate::qname::normalize_uri(uri),
        );
    }
    namespaces.insert("xml".into(), "http://www.w3.org/XML/1998/namespace".into());
    namespaces
        .entry("xs".into())
        .or_insert_with(|| "http://www.w3.org/2001/XMLSchema".into());
    namespaces
        .entry("fn".into())
        .or_insert_with(|| "http://www.w3.org/2005/xpath-functions".into());
    namespaces
}
fn expr(n: Node<'_, '_>, s: &str) -> Result<Expr, Diagnostic> {
    expression(n, s, false)
}
fn expression(n: Node<'_, '_>, s: &str, optional: bool) -> Result<Expr, Diagnostic> {
    let namespaces = namespaces(n);
    let parsed = if optional {
        xpath::parse_optional(s, namespaces)
    } else {
        xpath::parse(s, namespaces)
    };
    parsed.map_err(|mut e| {
        let pos = n.document().text_pos_at(n.range().start);
        e.line = pos.row;
        e.column = pos.col;
        e
    })
}
fn attrs(n: Node<'_, '_>, allowed: &[&str]) -> Result<(), Diagnostic> {
    excluded_uris(n)?;
    for a in n.attributes() {
        if a.namespace() == Some(XSL) && a.name() == "xpath-default-namespace" {
            return Err(static_error(
                n,
                "XTSE0090",
                "xpath-default-namespace must be unprefixed on XSLT elements",
            ));
        }
        if a.namespace() == Some("http://www.w3.org/XML/1998/namespace") && a.name() == "space" {
            continue;
        }
        if a.namespace().is_none() && a.name() == "default-mode" {
            default_mode(n)?;
            continue;
        }
        if a.namespace().is_none()
            && matches!(
                a.name(),
                "exclude-result-prefixes" | "xpath-default-namespace"
            )
        {
            continue;
        }
        if a.namespace().is_none() && a.name() == "use-when" {
            continue;
        }
        if a.namespace().is_none() && a.name() == "expand-text" {
            boolean_attribute(n, "expand-text", false)?;
            continue;
        }
        if a.namespace().is_some() || !allowed.contains(&a.name()) {
            return Err(error(n, format!("unsupported attribute: {}", a.name())));
        }
    }
    Ok(())
}
fn empty(n: Node<'_, '_>) -> Result<(), Diagnostic> {
    if n.children()
        .any(|c| c.is_element() || c.is_text() && !xml_space(c.text().unwrap_or_default()))
    {
        return Err(error(n, "instruction content is unsupported"));
    }
    Ok(())
}
fn boolean_value(n: Node<'_, '_>, value: Option<&str>, default: bool) -> Result<bool, Diagnostic> {
    match value.map(str::trim) {
        None => Ok(default),
        Some("yes" | "true" | "1") => Ok(true),
        Some("no" | "false" | "0") => Ok(false),
        _ => {
            let mut e = error(n, "invalid boolean stylesheet attribute");
            e.code = "XTSE0020".into();
            Err(e)
        }
    }
}
fn boolean_attribute(n: Node<'_, '_>, name: &str, default: bool) -> Result<bool, Diagnostic> {
    boolean_value(n, n.attribute(name), default)
}
fn standalone(n: Node<'_, '_>) -> Result<Option<bool>, Diagnostic> {
    match n.attribute("standalone").map(str::trim) {
        None | Some("omit") => Ok(None),
        value => boolean_value(n, value, false).map(Some),
    }
}
fn binding(n: Node<'_, '_>, parameter: bool, depth: usize) -> Result<Binding, Diagnostic> {
    let declaration = n.tag_name().name() == "param";
    if !declaration && n.attribute("required").is_some() {
        return Err(static_error(
            n,
            "XTSE0090",
            "required is only allowed on xsl:param",
        ));
    }
    attrs(
        n,
        if declaration {
            &["name", "select", "as", "required", "tunnel", "static"]
        } else if parameter {
            &["name", "select", "as", "tunnel"]
        } else {
            &["name", "select", "as", "static"]
        },
    )?;
    let tunnel = parameter && boolean_attribute(n, "tunnel", false)?;
    if declaration
        && tunnel
        && n.parent_element().is_none_or(|p| {
            p.tag_name().namespace() != Some(XSL) || p.tag_name().name() != "template"
        })
    {
        return Err(static_error(
            n,
            "XTSE0020",
            "tunnel parameters must belong to a template",
        ));
    }
    let mandatory = declaration && boolean_attribute(n, "required", false)?;
    if mandatory
        && (n.attribute("select").is_some()
            || n.children()
                .any(|c| c.is_element() || c.is_text() && !xml_space(c.text().unwrap_or_default())))
    {
        return Err(static_error(
            n,
            "XTSE0010",
            "required parameters cannot have a default value",
        ));
    }
    let ty = n
        .attribute("as")
        .map(|s| crate::seqtype::SequenceType::parse(s, &namespaces(n)))
        .transpose()?;
    let id = component_name(
        n,
        n.attribute("name")
            .ok_or_else(|| static_error(n, "XTSE0010", "binding requires a name"))?,
    )?;
    check_component_namespace(n, &id, false)?;
    let value = if let Some(s) = n.attribute("select") {
        empty(n)?;
        BindingValue::Select(expr(n, s)?)
    } else {
        let body = body(n, depth + 1, false)?;
        if mandatory && !body.is_empty() {
            return Err(static_error(
                n,
                "XTSE0010",
                "required parameters cannot have a default value",
            ));
        }
        if body.is_empty() {
            BindingValue::Select(if ty.is_some() {
                Expr::Sequence(vec![])
            } else {
                Expr::String(String::new())
            })
        } else if ty.is_some() && body.len() == 1 && matches!(body[0], Instruction::Document(_)) {
            let Instruction::Document(body) = body.into_iter().next().unwrap() else {
                unreachable!()
            };
            BindingValue::Document(body)
        } else if ty.is_some() && body.len() == 1 && matches!(body[0], Instruction::Sequence(_)) {
            let Instruction::Sequence(e) = body.into_iter().next().unwrap() else {
                unreachable!()
            };
            BindingValue::Select(e)
        } else if ty.is_some()
            && raw_result_body(&body)
            && (!element_result_body(&body)
                || top_sequence(&body)
                || body.iter().all(|i| matches!(i, Instruction::Bind(_))))
        {
            BindingValue::Raw(body)
        } else {
            if ty.as_ref().is_some_and(|ty| {
                matches!(
                    ty.kind,
                    crate::seqtype::Kind::Attribute | crate::seqtype::Kind::NamedAttribute(_)
                )
            }) {
                return Err(error(
                    n,
                    "standalone attribute constructor sequences are not implemented yet",
                ));
            }
            if ty.is_some() && top_sequence(&body) {
                return Err(error(
                    n,
                    "mixed typed constructor sequences are not implemented yet",
                ));
            }
            BindingValue::Body(body)
        }
    };
    let implicit_required = declaration
        && n.attribute("select").is_none()
        && ty.as_ref().is_some_and(|ty| ty.min > 0)
        && matches!(&value, BindingValue::Select(Expr::Sequence(es)) if es.is_empty());
    Ok(Binding {
        name: id,
        value,
        parameter,
        required: mandatory,
        implicit_required,
        tunnel,
        ty,
    })
}
// Typed mixed top-level atomic/node sequences need a raw XDM collector. Do not
// silently turn those atomics into text while the collector is still pending.
fn top_sequence(body: &[Instruction]) -> bool {
    body.iter().any(|i| match i {
        Instruction::Sequence(_)
        | Instruction::Call(_, _)
        | Instruction::NextMatch(_)
        | Instruction::ApplyImports(_)
        | Instruction::CopyOf(..)
        | Instruction::Copy { .. }
        | Instruction::Document(_) => true,
        Instruction::ForEach(_, _, b)
        | Instruction::ForEachGroup(_, _, _, b)
        | Instruction::If(_, b) => top_sequence(b),
        Instruction::Choose(bs) => bs.iter().any(|(_, b)| top_sequence(b)),
        _ => false,
    })
}
pub(crate) fn raw_result_body(body: &[Instruction]) -> bool {
    body.iter().all(|i| match i {
        Instruction::Sequence(_)
        | Instruction::CopyOf(..)
        | Instruction::Copy { .. }
        | Instruction::Apply(..)
        | Instruction::Call(..)
        | Instruction::NextMatch(_)
        | Instruction::ApplyImports(_)
        | Instruction::Bind(_)
        | Instruction::Element { .. }
        | Instruction::ComputedElement(..)
        | Instruction::Document(_)
        | Instruction::Text(_)
        | Instruction::TextValue(_)
        | Instruction::Value(..)
        | Instruction::Comment(_)
        | Instruction::Message(..)
        | Instruction::AssertFailure(_)
        | Instruction::UseSets(_)
        | Instruction::Iterate { .. }
        | Instruction::NextIteration(_)
        | Instruction::Break(_)
        | Instruction::Number(_)
        | Instruction::WherePopulated(_)
        | Instruction::OnEmpty(_)
        | Instruction::OnNonEmpty(_)
        | Instruction::Populated { .. }
        | Instruction::Namespace(..)
        | Instruction::ResultDocument(_)
        | Instruction::Try { .. }
        | Instruction::AnalyzeString { .. }
        | Instruction::LiteralAttributes(_)
        | Instruction::Pi(..)
        | Instruction::Attribute(..) => true,
        Instruction::ForEach(_, _, body)
        | Instruction::ForEachGroup(_, _, _, body)
        | Instruction::If(_, body) => raw_result_body(body),
        Instruction::Choose(branches) => branches.iter().all(|(_, body)| raw_result_body(body)),
    })
}
fn element_result_body(body: &[Instruction]) -> bool {
    body.iter().all(|i| match i {
        Instruction::Element { .. } | Instruction::ComputedElement(..) | Instruction::Bind(_) => {
            true
        }
        Instruction::ForEach(_, _, body)
        | Instruction::ForEachGroup(_, _, _, body)
        | Instruction::If(_, body) => element_result_body(body),
        Instruction::Choose(branches) => branches.iter().all(|(_, body)| element_result_body(body)),
        _ => false,
    })
}
/// Whether `expand-text` is in force for a node: the nearest declaration wins.
fn expand_text(n: Node<'_, '_>) -> bool {
    for ancestor in n.ancestors().filter(|a| a.is_element()) {
        let value = if ancestor.tag_name().namespace() == Some(XSL) {
            ancestor.attribute("expand-text")
        } else {
            ancestor.attribute((XSL, "expand-text"))
        };
        if let Some(value) = value {
            return matches!(value.trim(), "yes" | "true" | "1");
        }
    }
    false
}
/// A text node, or a text value template when `expand-text` applies.
fn text_instruction(n: Node<'_, '_>, text: &str) -> Result<Instruction, Diagnostic> {
    if expand_text(n) && text.contains(['{', '}']) {
        Ok(Instruction::TextValue(avt(n, text)?))
    } else {
        Ok(Instruction::Text(text.into()))
    }
}
thread_local! {
    static STATIC_VARIABLES: std::cell::RefCell<xpath::Variables<'static>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
}
/// Evaluate a static (compile-time) expression such as a `use-when` condition.
fn static_value(n: Node<'_, '_>, source: &str) -> Result<xpath::Value<'static>, Diagnostic> {
    let parsed = expr(n, source)?;
    let variables = STATIC_VARIABLES.with(|v| v.borrow().clone());
    let mut budget = xpath::Budget::new(crate::Limits::default());
    let focus = xpath::Focus {
        item: xpath::Item::Absent,
        position: 0,
        size: 0,
    };
    xpath::eval(&parsed, &focus, &variables, &mut budget, 0, false).map_err(|mut e| {
        let pos = n.document().text_pos_at(n.range().start);
        e.phase = "compile";
        e.line = pos.row;
        e.column = pos.col;
        e
    })
}
/// Whether a `use-when` condition on this element excludes it.
fn use_when_excludes(n: Node<'_, '_>) -> Result<bool, Diagnostic> {
    let condition = if n.tag_name().namespace() == Some(XSL) {
        n.attribute("use-when")
    } else {
        n.attribute((XSL, "use-when"))
    };
    let Some(condition) = condition else {
        return Ok(false);
    };
    Ok(!xpath::boolean(&static_value(n, condition)?)?)
}
fn avt(n: Node<'_, '_>, value: &str) -> Result<Vec<Expr>, Diagnostic> {
    let mut parts = vec![];
    let mut text = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                text.push('{');
                continue;
            }
            if !text.is_empty() {
                parts.push(Expr::String(std::mem::take(&mut text)));
            }
            let mut expression = String::new();
            let mut quote = None;
            let mut uri = false;
            let mut comments = 0usize;
            let mut closed = false;
            while let Some(c) = chars.next() {
                if comments > 0 {
                    expression.push(c);
                    if c == '(' && chars.peek() == Some(&':') {
                        expression.push(chars.next().unwrap());
                        comments += 1;
                        if comments > 128 {
                            return Err(static_error(
                                n,
                                "XPST0003",
                                "XPath comment nesting limit exceeded",
                            ));
                        }
                    } else if c == ':' && chars.peek() == Some(&')') {
                        expression.push(chars.next().unwrap());
                        comments -= 1;
                    }
                } else if uri {
                    expression.push(c);
                    if c == '}' {
                        uri = false;
                    }
                } else if let Some(q) = quote {
                    expression.push(c);
                    if c == q {
                        quote = None;
                    }
                } else if c == '(' && chars.peek() == Some(&':') {
                    expression.push(c);
                    expression.push(chars.next().unwrap());
                    comments = 1;
                } else if c == '\'' || c == '"' {
                    quote = Some(c);
                    expression.push(c);
                } else if c == '}' {
                    closed = true;
                    break;
                } else if c == '{' && expression.ends_with('Q') {
                    uri = true;
                    expression.push(c);
                } else if c == '{' {
                    return Err(static_error(n, "XTSE0350", "unexpected opening AVT brace"));
                } else {
                    expression.push(c);
                }
            }
            if comments > 0 {
                return Err(static_error(n, "XPST0003", "unterminated XPath comment"));
            }
            if !closed {
                return Err(static_error(
                    n,
                    "XTSE0350",
                    "unclosed attribute value template",
                ));
            }
            let optional = n.document().root_element().attribute("version") == Some("3.0");
            parts.push(self::expression(n, &expression, optional)?);
        } else if c == '}' {
            if chars.next() != Some('}') {
                return Err(static_error(n, "XTSE0350", "unescaped closing AVT brace"));
            }
            text.push('}');
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        parts.push(Expr::String(text));
    }
    Ok(parts)
}
fn computed_name(n: Node<'_, '_>) -> Result<ComputedName, Diagnostic> {
    let mut bindings = namespaces(n);
    if let Some(uri) = n.lookup_namespace_uri(None) {
        bindings.insert(String::new(), uri.into());
    }
    Ok(ComputedName {
        parts: avt(n, required(n, "name")?)?,
        namespace: n.attribute("namespace").map(|s| avt(n, s)).transpose()?,
        bindings,
    })
}
fn content(n: Node<'_, '_>, depth: usize) -> Result<Content, Diagnostic> {
    let separator = avt(n, n.attribute("separator").unwrap_or(" "))?;
    if let Some(s) = n.attribute("select") {
        empty(n)?;
        Ok(Content::Select(expr(n, s)?, separator))
    } else {
        let separator = avt(
            n,
            n.attribute("separator")
                .unwrap_or(if n.tag_name().name() == "attribute" {
                    ""
                } else {
                    " "
                }),
        )?;
        Ok(Content::Body(body(n, depth, false)?, separator))
    }
}
fn with_params(
    n: Node<'_, '_>,
    depth: usize,
    allow_sort: bool,
) -> Result<Vec<Binding>, Diagnostic> {
    let mut params = vec![];
    for c in n.children() {
        if c.is_text() && !xml_space(c.text().unwrap_or_default()) {
            return Err(error(c, "unexpected template invocation text"));
        }
        if !c.is_element() {
            continue;
        }
        if allow_sort && c.tag_name().namespace() == Some(XSL) && c.tag_name().name() == "sort" {
            continue;
        }
        if c.tag_name().namespace() != Some(XSL) || c.tag_name().name() != "with-param" {
            return Err(error(c, "expected with-param"));
        }
        let b = binding(c, true, depth)?;
        if params.iter().any(|p: &Binding| p.name == b.name) {
            let mut e = error(c, "duplicate with-param");
            e.code = "XTSE0670".into();
            return Err(e);
        }
        params.push(b);
    }
    Ok(params)
}
fn sorts(n: Node<'_, '_>) -> Result<Vec<Sort>, Diagnostic> {
    let mut keys = vec![];
    let mut content = false;
    for c in n.children() {
        if c.is_element() && c.tag_name().namespace() == Some(XSL) && c.tag_name().name() == "sort"
        {
            if content {
                return Err(error(c, "sort keys must precede the instruction body"));
            }
            attrs(
                c,
                &[
                    "select",
                    "data-type",
                    "order",
                    "case-order",
                    "lang",
                    "collation",
                    "stable",
                ],
            )?;
            empty(c)?;
            let dynamic_attr = |name: &str| -> Result<Option<Vec<Expr>>, Diagnostic> {
                match c.attribute(name) {
                    Some(v) if v.contains(['{', '}']) => avt(c, v).map(Some),
                    _ => Ok(None),
                }
            };
            let mut dynamic = DynamicSort {
                data_type: dynamic_attr("data-type")?,
                order: dynamic_attr("order")?,
                case_order: dynamic_attr("case-order")?,
                collation: dynamic_attr("collation")?,
            };
            let numeric = match c.attribute("data-type").unwrap_or("text") {
                _ if dynamic.data_type.is_some() => false,
                "text" => false,
                "number" => true,
                _ => return Err(error(c, "unsupported sort data-type")),
            };
            let descending = match c.attribute("order").unwrap_or("ascending") {
                _ if dynamic.order.is_some() => false,
                "ascending" => false,
                "descending" => true,
                _ => return Err(error(c, "unsupported sort order")),
            };
            let case_order = if dynamic.case_order.is_some() {
                ""
            } else {
                c.attribute("case-order").unwrap_or("")
            };
            if !["", "upper-first", "lower-first"].contains(&case_order) {
                return Err(error(c, "unsupported case-order"));
            }
            let collation = match c.attribute("collation") {
                Some(uri) if dynamic.collation.is_none() => Some(
                    crate::collation::Collation::parse(uri.trim()).map_err(|mut e| {
                        e.code = "XTDE1035".into();
                        e.phase = "compile";
                        e
                    })?,
                ),
                _ => None,
            };
            let _ = &mut dynamic;
            keys.push(Sort {
                select: expr(c, c.attribute("select").unwrap_or("."))?,
                numeric,
                descending,
                case_order: case_order.into(),
                collation,
                dynamic,
            });
        } else if c.is_element() || c.is_text() && !xml_space(c.text().unwrap_or_default()) {
            if n.tag_name().name() == "apply-templates"
                && !(c.is_element()
                    && c.tag_name().namespace() == Some(XSL)
                    && c.tag_name().name() == "with-param")
            {
                return Err(error(
                    c,
                    "apply-templates requires sort or with-param children",
                ));
            }
            content = true;
        }
    }
    Ok(keys)
}
fn excluded_uris(n: Node<'_, '_>) -> Result<BTreeSet<String>, Diagnostic> {
    let mut uris = BTreeSet::new();
    for ancestor in n.ancestors().filter(|n| n.is_element()) {
        let value = if ancestor.tag_name().namespace() == Some(XSL) {
            ancestor.attribute("exclude-result-prefixes")
        } else {
            ancestor.attribute((XSL, "exclude-result-prefixes"))
        };
        for token in value.unwrap_or("").split_whitespace() {
            if token == "#all" {
                uris.extend(ancestor.namespaces().map(|ns| ns.uri().to_owned()));
            } else {
                let prefix = if token == "#default" {
                    None
                } else {
                    Some(token)
                };
                let uri = ancestor.lookup_namespace_uri(prefix).ok_or_else(|| {
                    let mut e = error(ancestor, "undeclared exclude-result-prefixes prefix");
                    e.code = "XTSE0808".into();
                    e
                })?;
                uris.insert(uri.into());
            }
        }
    }
    Ok(uris)
}
fn space_rules(n: Node<'_, '_>, strip: bool) -> Result<Vec<crate::whitespace::Rule>, Diagnostic> {
    attrs(n, &["elements"])?;
    empty(n)?;
    let mut rules = vec![];
    for token in required(n, "elements")?.split_whitespace() {
        let (uri, local, specificity) = if token == "*" {
            (None, None, 0)
        } else if let Some(prefix) = token.strip_suffix(":*") {
            let uri = n
                .lookup_namespace_uri(Some(prefix))
                .ok_or_else(|| error(n, "unbound whitespace-rule prefix"))?;
            (Some(uri.into()), None, 1)
        } else {
            let e = expr(n, token)?;
            let Expr::Path(base, steps) = e else {
                return Err(error(n, "invalid whitespace-rule name"));
            };
            if steps.len() != 1
                || !matches!(*base, Expr::Context)
                || !matches!(steps[0].axis, xpath::Axis::Child)
                || !steps[0].predicates.is_empty()
            {
                return Err(error(n, "invalid whitespace-rule name"));
            }
            let xpath::Test::Name(name) = &steps[0].test else {
                return Err(error(n, "invalid whitespace-rule name"));
            };
            (
                Some(name.uri.clone().unwrap_or_default()),
                Some(name.local.clone()),
                2,
            )
        };
        rules.push(crate::whitespace::Rule {
            uri,
            local,
            specificity,
            strip,
        });
    }
    if rules.is_empty() {
        return Err(error(n, "empty whitespace-rule list"));
    }
    Ok(rules)
}
fn body(
    parent: Node<'_, '_>,
    depth: usize,
    skip_params: bool,
) -> Result<Vec<Instruction>, Diagnostic> {
    if depth > 128 {
        return Err(error(parent, "stylesheet nesting limit exceeded"));
    }
    let mut out = vec![];
    for n in parent.children() {
        if n.is_text() {
            let t = n.text().unwrap_or_default();
            if !xml_space(t)
                || parent
                    .ancestors()
                    .filter_map(|n| n.attribute(("http://www.w3.org/XML/1998/namespace", "space")))
                    .next()
                    == Some("preserve")
            {
                out.push(text_instruction(n, t)?);
            }
        } else if n.is_element() {
            if use_when_excludes(n)? {
                continue;
            }
            if n.tag_name().namespace() == Some(XSL) {
                match n.tag_name().name() {
                    "param" if skip_params => continue,
                    "text" => {
                        attrs(n, &[])?;
                        if n.children().any(|c| c.is_element()) {
                            return Err(error(n, "xsl:text cannot contain elements"));
                        }
                        let text: String = n
                            .children()
                            .filter(|c| c.is_text())
                            .filter_map(|c| c.text())
                            .collect();
                        out.push(text_instruction(n, &text)?);
                    }
                    "value-of" | "apply-templates" => {
                        attrs(
                            n,
                            if n.tag_name().name() == "apply-templates" {
                                &["select", "mode"]
                            } else {
                                &["select", "separator"]
                            },
                        )?;
                        let sorts = if n.tag_name().name() == "apply-templates" {
                            sorts(n)?
                        } else {
                            empty(n)?;
                            vec![]
                        };
                        let value = if let Some(s) = n.attribute("select") {
                            expr(n, s)?
                        } else if n.tag_name().name() == "apply-templates" {
                            expr(n, "node()")?
                        } else {
                            return Err(error(n, "value-of requires select"));
                        };
                        out.push(if n.tag_name().name() == "value-of" {
                            Instruction::Value(
                                value,
                                n.attribute("separator").map(|s| avt(n, s)).transpose()?,
                            )
                        } else {
                            Instruction::Apply(
                                value,
                                mode(n, n.attribute("mode").unwrap_or("#default"), true)?,
                                sorts,
                                with_params(n, depth, true)?,
                            )
                        });
                    }
                    "param" | "on-completion"
                        if parent.tag_name().namespace() == Some(XSL)
                            && parent.tag_name().name() == "iterate" =>
                    {
                        continue;
                    }
                    "catch"
                        if parent.tag_name().namespace() == Some(XSL)
                            && parent.tag_name().name() == "try" =>
                    {
                        continue;
                    }
                    "sort"
                        if parent.tag_name().namespace() == Some(XSL)
                            && matches!(
                                parent.tag_name().name(),
                                "for-each" | "for-each-group" | "perform-sort"
                            ) =>
                    {
                        continue;
                    }
                    "element" => {
                        attrs(
                            n,
                            &[
                                "name",
                                "namespace",
                                "inherit-namespaces",
                                "use-attribute-sets",
                            ],
                        )?;
                        let mut inner = body(n, depth + 1, false)?;
                        if let Some(sets) = n.attribute("use-attribute-sets") {
                            inner.insert(0, Instruction::UseSets(set_names(n, sets)?));
                        }
                        out.push(Instruction::ComputedElement(
                            computed_name(n)?,
                            boolean_attribute(n, "inherit-namespaces", true)?,
                            inner,
                        ));
                    }
                    "attribute" => {
                        attrs(n, &["name", "namespace", "select", "separator"])?;
                        out.push(Instruction::Attribute(
                            computed_name(n)?,
                            content(n, depth + 1)?,
                        ));
                    }
                    "comment" => {
                        attrs(n, &["select"])?;
                        out.push(Instruction::Comment(content(n, depth + 1)?));
                    }
                    "map" => {
                        attrs(n, &["on-duplicates", "use-attribute-sets"])?;
                        if n.attribute("on-duplicates").is_some() {
                            return Err(error(n, "map on-duplicates is not implemented"));
                        }
                        let parts = map_parts(n, depth)?;
                        out.push(Instruction::Sequence(Expr::Call(
                            "map:merge".into(),
                            vec![
                                Expr::Sequence(parts),
                                Expr::MapConstructor(vec![(
                                    Expr::String("duplicates".into()),
                                    Expr::String("reject".into()),
                                )]),
                            ],
                        )));
                    }
                    "map-entry" => {
                        return Err(static_error(
                            n,
                            "XTSE0010",
                            "xsl:map-entry is only allowed inside xsl:map",
                        ));
                    }
                    "evaluate" => {
                        attrs(
                            n,
                            &[
                                "xpath",
                                "as",
                                "context-item",
                                "namespace-context",
                                "schema-aware",
                                "base-uri",
                                "with-params",
                                "static",
                            ],
                        )?;
                        if n.attribute("namespace-context").is_some()
                            || n.attribute("base-uri").is_some()
                            || n.attribute("with-params").is_some()
                        {
                            return Err(error(
                                n,
                                "xsl:evaluate namespace-context, base-uri and with-params are not implemented",
                            ));
                        }
                        let mut params = vec![];
                        for c in n.children() {
                            if c.is_element()
                                && c.tag_name().namespace() == Some(XSL)
                                && c.tag_name().name() == "with-param"
                            {
                                let b = binding(c, true, depth)?;
                                let BindingValue::Select(e) = b.value else {
                                    return Err(error(
                                        c,
                                        "xsl:evaluate parameters need a select attribute",
                                    ));
                                };
                                params.push((b.name, e));
                            } else if c.is_element()
                                || c.is_text() && !xml_space(c.text().unwrap_or_default())
                            {
                                return Err(static_error(
                                    c,
                                    "XTSE0010",
                                    "unexpected content in xsl:evaluate",
                                ));
                            }
                        }
                        out.push(Instruction::Sequence(Expr::Evaluate {
                            xpath: Box::new(expr(n, required(n, "xpath")?)?),
                            context: n
                                .attribute("context-item")
                                .map(|c| expr(n, c).map(Box::new))
                                .transpose()?,
                            params,
                            namespaces: namespaces(n),
                        }));
                    }
                    "perform-sort" => {
                        attrs(n, &["select"])?;
                        let keys = sorts(n)?;
                        if keys.is_empty() {
                            return Err(static_error(
                                n,
                                "XTSE0010",
                                "xsl:perform-sort needs at least one xsl:sort",
                            ));
                        }
                        let source = match n.attribute("select") {
                            Some(s) => {
                                if n.children().any(|c| {
                                    c.is_element()
                                        && !(c.tag_name().namespace() == Some(XSL)
                                            && c.tag_name().name() == "sort")
                                        || c.is_text() && !xml_space(c.text().unwrap_or_default())
                                }) {
                                    return Err(static_error(
                                        n,
                                        "XTSE1040",
                                        "xsl:perform-sort with select cannot have a body",
                                    ));
                                }
                                expr(n, s)?
                            }
                            None => {
                                let content = body(n, depth + 1, false)?;
                                match <[Instruction; 1]>::try_from(content) {
                                    Ok([Instruction::Sequence(e)]) => e,
                                    Ok(_) | Err(_) => {
                                        return Err(error(
                                            n,
                                            "xsl:perform-sort bodies are limited to one xsl:sequence",
                                        ));
                                    }
                                }
                            }
                        };
                        out.push(Instruction::Sequence(Expr::Sorted(Box::new(source), keys)));
                    }
                    "number" => {
                        attrs(
                            n,
                            &[
                                "value",
                                "select",
                                "level",
                                "count",
                                "from",
                                "format",
                                "lang",
                                "letter-value",
                                "ordinal",
                                "grouping-separator",
                                "grouping-size",
                                "start-at",
                            ],
                        )?;
                        empty(n)?;
                        if n.attribute("start-at").is_some() {
                            return Err(error(n, "xsl:number start-at is not implemented"));
                        }
                        if n.attribute("value").is_some()
                            && ["select", "level", "count", "from"]
                                .iter()
                                .any(|a| n.attribute(*a).is_some())
                        {
                            return Err(static_error(
                                n,
                                "XTSE0975",
                                "value excludes select, level, count and from",
                            ));
                        }
                        let level = match n.attribute("level").unwrap_or("single") {
                            "single" => NumberLevel::Single,
                            "multiple" => NumberLevel::Multiple,
                            "any" => NumberLevel::Any,
                            _ => return Err(static_error(n, "XTSE0020", "invalid number level")),
                        };
                        let pattern = |name: &str| {
                            n.attribute(name)
                                .map(|p| crate::pattern::parse(p, namespaces(n), xslt_version(n)))
                                .transpose()
                        };
                        let optional_avt = |name: &str| -> Result<Vec<Expr>, Diagnostic> {
                            n.attribute(name)
                                .map(|v| avt(n, v))
                                .transpose()
                                .map(Option::unwrap_or_default)
                        };
                        out.push(Instruction::Number(Box::new(NumberSpec {
                            value: n.attribute("value").map(|v| expr(n, v)).transpose()?,
                            select: n.attribute("select").map(|v| expr(n, v)).transpose()?,
                            level,
                            count: pattern("count")?,
                            from: pattern("from")?,
                            format: match n.attribute("format") {
                                Some(f) => avt(n, f)?,
                                None => vec![Expr::String("1".into())],
                            },
                            ordinal: optional_avt("ordinal")?,
                            grouping_separator: optional_avt("grouping-separator")?,
                            grouping_size: optional_avt("grouping-size")?,
                        })));
                    }
                    "result-document" => {
                        attrs(
                            n,
                            &[
                                "href",
                                "format",
                                "method",
                                "omit-xml-declaration",
                                "standalone",
                                "byte-order-mark",
                                "indent",
                                "escape-uri-attributes",
                                "include-content-type",
                                "media-type",
                                "encoding",
                                "output-version",
                                "doctype-public",
                                "doctype-system",
                                "cdata-section-elements",
                                "use-character-maps",
                                "validation",
                                "type",
                            ],
                        )?;
                        let encoding = n
                            .attribute("encoding")
                            .map(|value| {
                                if !value.contains('{')
                                    && !value.contains('}')
                                    && crate::result::Encoding::parse(value).is_none()
                                {
                                    return Err(error(
                                        n,
                                        "only UTF-8, ISO-8859-1 and US-ASCII results are supported",
                                    ));
                                }
                                avt(n, value)
                            })
                            .transpose()?;
                        let method = n.attribute("method");
                        if method.is_some_and(|method| {
                            !method.contains('{')
                                && !method.contains('}')
                                && !matches!(method, "xml" | "html" | "xhtml" | "text")
                        }) {
                            return Err(error(n, "unsupported result document method"));
                        }
                        let flag = |name: &str| -> Result<Option<Vec<Expr>>, Diagnostic> {
                            n.attribute(name)
                                .map(|value| {
                                    if !value.contains('{') && !value.contains('}') {
                                        boolean_attribute(n, name, false)?;
                                    }
                                    avt(n, value)
                                })
                                .transpose()
                        };
                        out.push(Instruction::ResultDocument(Box::new(ResultDocumentSpec {
                            href: n
                                .attribute("href")
                                .map(|h| avt(n, h))
                                .transpose()?
                                .unwrap_or_default(),
                            method: method.map(|value| avt(n, value)).transpose()?,
                            format: n
                                .attribute("format")
                                .map(|value| avt(n, value).map(|parts| (parts, namespaces(n))))
                                .transpose()?,
                            omit_declaration: flag("omit-xml-declaration")?,
                            standalone: n
                                .attribute("standalone")
                                .map(|value| {
                                    if !value.contains('{') && !value.contains('}') {
                                        standalone(n)?;
                                    }
                                    avt(n, value)
                                })
                                .transpose()?,
                            byte_order_mark: flag("byte-order-mark")?,
                            indent: flag("indent")?,
                            escape_uri_attributes: flag("escape-uri-attributes")?,
                            include_content_type: flag("include-content-type")?,
                            media_type: n
                                .attribute("media-type")
                                .map(|value| avt(n, value))
                                .transpose()?,
                            encoding,
                            output_version: n
                                .attribute("output-version")
                                .map(|value| avt(n, value))
                                .transpose()?,
                            cdata: n
                                .attribute("cdata-section-elements")
                                .map(|value| {
                                    let mut bindings = namespaces(n);
                                    bindings.remove(crate::qname::XPATH_DEFAULT_NAMESPACE);
                                    if let Some(uri) = n.lookup_namespace_uri(None) {
                                        bindings.insert(
                                            crate::qname::XPATH_DEFAULT_NAMESPACE.into(),
                                            uri.into(),
                                        );
                                    }
                                    avt(n, value).map(|parts| (parts, bindings))
                                })
                                .transpose()?,
                            character_maps: n
                                .attribute("use-character-maps")
                                .unwrap_or("")
                                .split_whitespace()
                                .map(|name| component_name(n, name))
                                .collect::<Result<Vec<_>, _>>()?,
                            doctype_public: n
                                .attribute("doctype-public")
                                .map(|value| avt(n, value))
                                .transpose()?,
                            doctype_system: n
                                .attribute("doctype-system")
                                .map(|value| avt(n, value))
                                .transpose()?,
                            body: body(n, depth + 1, false)?,
                        })));
                    }
                    "on-empty" | "on-non-empty" => {
                        attrs(n, &["select"])?;
                        let content = match n.attribute("select") {
                            Some(s) => {
                                empty(n)?;
                                vec![Instruction::Sequence(expr(n, s)?)]
                            }
                            None => body(n, depth + 1, false)?,
                        };
                        out.push(if n.tag_name().name() == "on-empty" {
                            Instruction::OnEmpty(content)
                        } else {
                            Instruction::OnNonEmpty(content)
                        });
                    }
                    "where-populated" => {
                        attrs(n, &[])?;
                        out.push(Instruction::WherePopulated(body(n, depth + 1, false)?));
                    }
                    "namespace" => {
                        attrs(n, &["name", "select"])?;
                        out.push(Instruction::Namespace(
                            avt(n, required(n, "name")?)?,
                            content(n, depth + 1)?,
                        ));
                    }
                    "try" => {
                        attrs(n, &["select", "rollback-output"])?;
                        boolean_attribute(n, "rollback-output", true)?;
                        let mut catches = vec![];
                        let mut seen_catch = false;
                        for c in n.children() {
                            if c.is_text() && !xml_space(c.text().unwrap_or_default()) {
                                if seen_catch {
                                    return Err(static_error(
                                        c,
                                        "XTSE0010",
                                        "text after xsl:catch in xsl:try",
                                    ));
                                }
                                continue;
                            }
                            if !c.is_element() {
                                continue;
                            }
                            let is_catch = c.tag_name().namespace() == Some(XSL)
                                && c.tag_name().name() == "catch";
                            if is_catch {
                                attrs(c, &["errors", "select"])?;
                                seen_catch = true;
                                let mut errors = vec![];
                                for token in c.attribute("errors").unwrap_or("*").split_whitespace()
                                {
                                    errors.push(error_test(c, token)?);
                                }
                                let content = match c.attribute("select") {
                                    Some(s) => {
                                        empty(c)?;
                                        vec![Instruction::Sequence(expr(c, s)?)]
                                    }
                                    None => body(c, depth + 1, false)?,
                                };
                                catches.push(Catch {
                                    errors,
                                    body: content,
                                });
                            } else if seen_catch {
                                return Err(static_error(
                                    c,
                                    "XTSE0010",
                                    "xsl:catch must follow the body of xsl:try",
                                ));
                            }
                        }
                        if catches.is_empty() {
                            return Err(static_error(n, "XTSE0010", "xsl:try needs a xsl:catch"));
                        }
                        let content = match n.attribute("select") {
                            Some(s) => {
                                if n.children().any(|c| {
                                    c.is_element()
                                        && !(c.tag_name().namespace() == Some(XSL)
                                            && c.tag_name().name() == "catch")
                                }) {
                                    return Err(static_error(
                                        n,
                                        "XTSE3140",
                                        "xsl:try with select cannot have a body",
                                    ));
                                }
                                vec![Instruction::Sequence(expr(n, s)?)]
                            }
                            None => body(n, depth + 1, false)?,
                        };
                        out.push(Instruction::Try {
                            body: content,
                            catches,
                        });
                    }
                    "iterate" => {
                        attrs(n, &["select"])?;
                        let mut params = vec![];
                        let mut on_completion = vec![];
                        for c in n.children() {
                            if !c.is_element() || c.tag_name().namespace() != Some(XSL) {
                                continue;
                            }
                            match c.tag_name().name() {
                                "param" => {
                                    let b = binding(c, true, depth)?;
                                    if b.required || b.tunnel {
                                        return Err(static_error(
                                            c,
                                            "XTSE0010",
                                            "iterate parameters cannot be required or tunnel",
                                        ));
                                    }
                                    if !matches!(b.value, BindingValue::Select(_)) {
                                        return Err(error(
                                            c,
                                            "iterate parameters need a select attribute",
                                        ));
                                    }
                                    if params.iter().any(|p: &Binding| p.name == b.name) {
                                        return Err(static_error(
                                            c,
                                            "XTSE0580",
                                            "duplicate iterate parameter",
                                        ));
                                    }
                                    params.push(b);
                                }
                                "on-completion" => {
                                    attrs(c, &["select"])?;
                                    on_completion = match c.attribute("select") {
                                        Some(s) => {
                                            empty(c)?;
                                            vec![Instruction::Sequence(expr(c, s)?)]
                                        }
                                        None => body(c, depth + 1, false)?,
                                    };
                                }
                                _ => {}
                            }
                        }
                        out.push(Instruction::Iterate {
                            select: expr(n, required(n, "select")?)?,
                            params,
                            on_completion,
                            body: body(n, depth + 1, false)?,
                        });
                    }
                    "next-iteration" => {
                        attrs(n, &[])?;
                        let params = with_params(n, depth, false)?;
                        if params
                            .iter()
                            .any(|p| !matches!(p.value, BindingValue::Select(_)))
                        {
                            return Err(error(
                                n,
                                "next-iteration parameters need a select attribute",
                            ));
                        }
                        out.push(Instruction::NextIteration(params));
                    }
                    "break" => {
                        attrs(n, &["select"])?;
                        out.push(Instruction::Break(match n.attribute("select") {
                            Some(s) => {
                                empty(n)?;
                                vec![Instruction::Sequence(expr(n, s)?)]
                            }
                            None => body(n, depth + 1, false)?,
                        }));
                    }
                    "analyze-string" => {
                        attrs(n, &["select", "regex", "flags"])?;
                        let mut matching = None;
                        let mut non_matching = None;
                        for c in n.children() {
                            if c.is_text() && !xml_space(c.text().unwrap_or_default()) {
                                return Err(static_error(
                                    c,
                                    "XTSE0010",
                                    "unexpected text in xsl:analyze-string",
                                ));
                            }
                            if !c.is_element() {
                                continue;
                            }
                            if c.tag_name().namespace() != Some(XSL) {
                                return Err(static_error(
                                    c,
                                    "XTSE0010",
                                    "unexpected element in xsl:analyze-string",
                                ));
                            }
                            match c.tag_name().name() {
                                "matching-substring"
                                    if matching.is_none() && non_matching.is_none() =>
                                {
                                    attrs(c, &[])?;
                                    matching = Some(body(c, depth + 1, false)?);
                                }
                                "non-matching-substring" if non_matching.is_none() => {
                                    attrs(c, &[])?;
                                    non_matching = Some(body(c, depth + 1, false)?);
                                }
                                "fallback" => {}
                                _ => {
                                    return Err(static_error(
                                        c,
                                        "XTSE0010",
                                        "invalid child of xsl:analyze-string",
                                    ));
                                }
                            }
                        }
                        if matching.is_none() && non_matching.is_none() {
                            return Err(static_error(
                                n,
                                "XTSE1130",
                                "xsl:analyze-string needs a matching or non-matching child",
                            ));
                        }
                        out.push(Instruction::AnalyzeString {
                            select: expr(n, required(n, "select")?)?,
                            regex: avt(n, required(n, "regex")?)?,
                            flags: n
                                .attribute("flags")
                                .map(|f| avt(n, f))
                                .transpose()?
                                .unwrap_or_default(),
                            matching: matching.unwrap_or_default(),
                            non_matching: non_matching.unwrap_or_default(),
                        });
                    }
                    "assert" => {
                        attrs(n, &["test", "select", "error-code"])?;
                        out.push(Instruction::If(
                            Expr::Call("not".into(), vec![expr(n, required(n, "test")?)?]),
                            vec![Instruction::AssertFailure(content(n, depth + 1)?)],
                        ));
                    }
                    "message" => {
                        attrs(n, &["select", "terminate", "error-code"])?;
                        out.push(Instruction::Message(
                            content(n, depth + 1)?,
                            boolean_attribute(n, "terminate", false)?,
                        ));
                    }
                    "processing-instruction" => {
                        attrs(n, &["name", "select"])?;
                        out.push(Instruction::Pi(
                            avt(n, required(n, "name")?)?,
                            content(n, depth + 1)?,
                        ));
                    }
                    "sequence" => {
                        attrs(n, &["select"])?;
                        empty(n)?;
                        out.push(Instruction::Sequence(expr(n, required(n, "select")?)?));
                    }
                    "copy" => {
                        attrs(n, &["select", "copy-namespaces", "inherit-namespaces"])?;
                        out.push(Instruction::Copy {
                            select: n.attribute("select").map(|s| expr(n, s)).transpose()?,
                            namespaces: boolean_attribute(n, "copy-namespaces", true)?,
                            inherit: boolean_attribute(n, "inherit-namespaces", true)?,
                            body: body(n, depth + 1, false)?,
                        });
                    }
                    "copy-of" => {
                        attrs(n, &["select", "copy-namespaces"])?;
                        empty(n)?;
                        out.push(Instruction::CopyOf(
                            expr(n, required(n, "select")?)?,
                            boolean_attribute(n, "copy-namespaces", true)?,
                        ));
                    }
                    "document" => {
                        attrs(n, &[])?;
                        out.push(Instruction::Document(body(n, depth + 1, false)?));
                    }
                    "variable" => out.push(Instruction::Bind(binding(n, false, depth)?)),
                    "for-each" => {
                        attrs(n, &["select"])?;
                        out.push(Instruction::ForEach(
                            expr(n, required(n, "select")?)?,
                            sorts(n)?,
                            body(n, depth + 1, false)?,
                        ));
                    }
                    "for-each-group" => {
                        attrs(
                            n,
                            &[
                                "select",
                                "group-by",
                                "group-adjacent",
                                "group-starting-with",
                                "group-ending-with",
                                "composite",
                                "collation",
                            ],
                        )?;
                        if n.attribute("collation").is_some()
                            || boolean_attribute(n, "composite", false)?
                        {
                            return Err(error(
                                n,
                                "grouping collations and composite keys are not implemented",
                            ));
                        }
                        let given = [
                            "group-by",
                            "group-adjacent",
                            "group-starting-with",
                            "group-ending-with",
                        ]
                        .into_iter()
                        .filter(|a| n.attribute(*a).is_some())
                        .collect::<Vec<_>>();
                        let [attribute] = given.as_slice() else {
                            return Err(static_error(
                                n,
                                "XTSE1080",
                                "exactly one grouping attribute is required",
                            ));
                        };
                        let value = n.attribute(*attribute).unwrap();
                        let grouping = match *attribute {
                            "group-by" => Grouping::By(expr(n, value)?),
                            "group-adjacent" => Grouping::Adjacent(expr(n, value)?),
                            "group-starting-with" => Grouping::Starting(crate::pattern::parse(
                                value,
                                namespaces(n),
                                xslt_version(n),
                            )?),
                            _ => Grouping::Ending(crate::pattern::parse(
                                value,
                                namespaces(n),
                                xslt_version(n),
                            )?),
                        };
                        out.push(Instruction::ForEachGroup(
                            expr(n, required(n, "select")?)?,
                            grouping,
                            sorts(n)?,
                            body(n, depth + 1, false)?,
                        ));
                    }
                    "if" => {
                        attrs(n, &["test"])?;
                        out.push(Instruction::If(
                            expr(n, required(n, "test")?)?,
                            body(n, depth + 1, false)?,
                        ));
                    }
                    "choose" => {
                        attrs(n, &[])?;
                        let mut branches = vec![];
                        let mut otherwise = false;
                        for c in n.children() {
                            if c.is_text() && !xml_space(c.text().unwrap_or_default()) {
                                return Err(error(c, "unexpected text in choose"));
                            }
                            if !c.is_element() {
                                continue;
                            }
                            if otherwise || c.tag_name().namespace() != Some(XSL) {
                                return Err(error(c, "invalid choose branch"));
                            }
                            let test = match c.tag_name().name() {
                                "when" => {
                                    attrs(c, &["test"])?;
                                    Some(expr(c, required(c, "test")?)?)
                                }
                                "otherwise" if !branches.is_empty() => {
                                    attrs(c, &[])?;
                                    otherwise = true;
                                    None
                                }
                                _ => {
                                    return Err(error(
                                        c,
                                        "choose requires when branches followed by optional otherwise",
                                    ));
                                }
                            };
                            branches.push((test, body(c, depth + 1, false)?));
                        }
                        if branches.is_empty() {
                            return Err(error(n, "choose requires at least one when"));
                        }
                        out.push(Instruction::Choose(branches));
                    }
                    "apply-imports" => {
                        attrs(n, &[])?;
                        out.push(Instruction::ApplyImports(with_params(n, depth, false)?));
                    }
                    "next-match" => {
                        attrs(n, &[])?;
                        out.push(Instruction::NextMatch(with_params(n, depth, false)?));
                    }
                    "call-template" => {
                        attrs(n, &["name"])?;
                        let id = component_name(n, required(n, "name")?)?;
                        let params = with_params(n, depth, false)?;
                        out.push(Instruction::Call(id, params));
                    }
                    other => {
                        return Err(error(n, format!("unsupported XSLT instruction: {other}")));
                    }
                }
            } else {
                if n.attribute((XSL, "default-mode")).is_some() {
                    default_mode(n)?;
                }
                let mut attributes = vec![];
                for a in n.attributes() {
                    if a.namespace() == Some(XSL) {
                        if [
                            "exclude-result-prefixes",
                            "xpath-default-namespace",
                            "inherit-namespaces",
                            "default-mode",
                            "expand-text",
                            "use-when",
                            "use-attribute-sets",
                        ]
                        .contains(&a.name())
                        {
                            continue;
                        }
                        return Err(error(n, "unsupported XSLT attribute on literal result"));
                    }
                    attributes.push((
                        crate::result::attribute_name(n, a.name(), a.namespace()),
                        avt(n, a.value())?,
                    ));
                }
                let mut element = crate::result::source_element(n, false);
                element.inherit_namespaces =
                    boolean_value(n, n.attribute((XSL, "inherit-namespaces")), true)?;
                let excluded = excluded_uris(n)?;
                element
                    .namespaces
                    .retain(|_, uri| uri != XSL && !excluded.contains(uri));
                let mut inner = body(n, depth + 1, false)?;
                if let Some(sets) = n.attribute((XSL, "use-attribute-sets")) {
                    // Literal attributes follow the sets so that they take precedence.
                    inner.insert(0, Instruction::LiteralAttributes(attributes));
                    inner.insert(0, Instruction::UseSets(set_names(n, sets)?));
                    attributes = vec![];
                }
                out.push(Instruction::Element {
                    element,
                    attributes,
                    body: inner,
                });
            }
        }
    }
    if out
        .iter()
        .any(|i| matches!(i, Instruction::OnEmpty(_) | Instruction::OnNonEmpty(_)))
    {
        let (mut main, mut on_empty, mut on_non_empty) = (vec![], vec![], vec![]);
        for instruction in out {
            match instruction {
                Instruction::OnEmpty(body) => on_empty.extend(body),
                Instruction::OnNonEmpty(body) => on_non_empty.extend(body),
                other => main.push(other),
            }
        }
        return Ok(vec![Instruction::Populated {
            main,
            on_empty,
            on_non_empty,
        }]);
    }
    Ok(out)
}
/// Translate the children of `xsl:map` into XPath expressions that yield maps.
fn map_parts(parent: Node<'_, '_>, depth: usize) -> Result<Vec<Expr>, Diagnostic> {
    let children: Vec<Node<'_, '_>> = parent
        .children()
        .filter(|c| c.is_element() || c.is_text() && !xml_space(c.text().unwrap_or_default()))
        .collect();
    map_nodes(&children, depth)
}
fn map_nodes(children: &[Node<'_, '_>], depth: usize) -> Result<Vec<Expr>, Diagnostic> {
    if depth > 128 {
        return Err(Diagnostic::new(
            "RUST_XSLT_UNSUPPORTED",
            "stylesheet nesting limit exceeded",
            "compile",
        ));
    }
    let mut parts: Vec<Expr> = vec![];
    for (index, c) in children.iter().enumerate() {
        if c.is_text() || c.tag_name().namespace() != Some(XSL) {
            return Err(static_error(
                *c,
                "XTSE0010",
                "xsl:map may only contain instructions that produce maps",
            ));
        }
        match c.tag_name().name() {
            "map-entry" => {
                attrs(*c, &["key", "select"])?;
                let key = expr(*c, required(*c, "key")?)?;
                let value = match c.attribute("select") {
                    Some(s) => {
                        empty(*c)?;
                        expr(*c, s)?
                    }
                    None => {
                        let content = body(*c, depth + 1, false)?;
                        match <[Instruction; 1]>::try_from(content) {
                            Ok([Instruction::Sequence(e)]) => e,
                            Err(content) if content.is_empty() => Expr::Sequence(vec![]),
                            _ => {
                                return Err(error(
                                    *c,
                                    "xsl:map-entry bodies are limited to one xsl:sequence",
                                ));
                            }
                        }
                    }
                };
                parts.push(Expr::Call("map:entry".into(), vec![key, value]));
            }
            "sequence" => {
                attrs(*c, &["select"])?;
                empty(*c)?;
                parts.push(expr(*c, required(*c, "select")?)?);
            }
            "if" => {
                attrs(*c, &["test"])?;
                parts.push(Expr::If(
                    Box::new(expr(*c, required(*c, "test")?)?),
                    Box::new(Expr::Sequence(map_parts(*c, depth + 1)?)),
                    Box::new(Expr::Sequence(vec![])),
                ));
            }
            "choose" => {
                attrs(*c, &[])?;
                let mut chain = Expr::Sequence(vec![]);
                let branches: Vec<Node<'_, '_>> = c.children().filter(|b| b.is_element()).collect();
                for branch in branches.iter().rev() {
                    match branch.tag_name().name() {
                        "when" => {
                            attrs(*branch, &["test"])?;
                            chain = Expr::If(
                                Box::new(expr(*branch, required(*branch, "test")?)?),
                                Box::new(Expr::Sequence(map_parts(*branch, depth + 1)?)),
                                Box::new(chain),
                            );
                        }
                        "otherwise" => {
                            attrs(*branch, &[])?;
                            chain = Expr::Sequence(map_parts(*branch, depth + 1)?);
                        }
                        _ => return Err(error(*branch, "invalid choose branch")),
                    }
                }
                parts.push(chain);
            }
            "for-each" => {
                attrs(*c, &["select"])?;
                if !sorts(*c)?.is_empty() {
                    return Err(error(*c, "sorting inside xsl:map is not implemented"));
                }
                parts.push(Expr::PathMap(
                    Box::new(expr(*c, required(*c, "select")?)?),
                    Box::new(Expr::Sequence(map_parts(*c, depth + 1)?)),
                ));
            }
            "variable" => {
                attrs(*c, &["name", "select", "as"])?;
                let name = component_name(*c, required(*c, "name")?)?;
                let value = expr(*c, required(*c, "select")?)?;
                // The variable scopes over the remaining siblings.
                let rest = map_nodes(&children[index + 1..], depth + 1)?;
                parts.push(Expr::Let(
                    name,
                    Box::new(value),
                    Box::new(Expr::Sequence(rest)),
                ));
                return Ok(parts);
            }
            other => {
                return Err(error(
                    *c,
                    format!("xsl:{other} is not supported inside xsl:map"),
                ));
            }
        }
    }
    Ok(parts)
}
fn error_test(
    n: Node<'_, '_>,
    token: &str,
) -> Result<(Option<String>, Option<String>), Diagnostic> {
    if token == "*" {
        return Ok((None, None));
    }
    if let Some(local) = token.strip_prefix("*:") {
        return Ok((None, Some(local.to_owned())));
    }
    if let Some(rest) = token.strip_prefix("Q{") {
        let (uri, local) = rest
            .split_once('}')
            .ok_or_else(|| static_error(n, "XTSE0020", "invalid error name test"))?;
        return Ok((
            Some(uri.to_owned()),
            (local != "*").then(|| local.to_owned()),
        ));
    }
    if let Some((prefix, local)) = token.split_once(':') {
        let uri = namespaces(n)
            .get(prefix)
            .cloned()
            .ok_or_else(|| static_error(n, "XTSE0280", format!("unbound prefix: {prefix}")))?;
        return Ok((Some(uri), (local != "*").then(|| local.to_owned())));
    }
    Ok((Some(String::new()), Some(token.to_owned())))
}
/// Flatten the character maps named by `use-character-maps`, following nested uses.
fn resolve_character_maps(
    maps: &BTreeMap<String, CharacterMap>,
    used: &[String],
) -> Result<BTreeMap<char, String>, Diagnostic> {
    fn visit(
        name: &str,
        maps: &BTreeMap<String, CharacterMap>,
        stack: &mut Vec<String>,
        out: &mut BTreeMap<char, String>,
    ) -> Result<(), Diagnostic> {
        let Some((uses, entries)) = maps.get(name) else {
            return Err(Diagnostic::new(
                "XTSE1590",
                format!("unknown character map: {name}"),
                "compile",
            ));
        };
        if stack.iter().any(|s| s == name) {
            return Err(Diagnostic::new(
                "XTSE1600",
                format!("character map uses itself: {name}"),
                "compile",
            ));
        }
        stack.push(name.to_owned());
        for used in uses {
            visit(used, maps, stack, out)?;
        }
        stack.pop();
        // Characters of the using map override those of the maps it uses.
        for (c, s) in entries {
            out.insert(*c, s.clone());
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    for name in used {
        visit(name, maps, &mut vec![], &mut out)?;
    }
    Ok(out)
}
fn set_names(n: Node<'_, '_>, value: &str) -> Result<Vec<String>, Diagnostic> {
    value
        .split([' ', '\t', '\r', '\n'])
        .filter(|s| !s.is_empty())
        .map(|s| component_name(n, s))
        .collect()
}
fn default_mode(n: Node<'_, '_>) -> Result<String, Diagnostic> {
    for ancestor in n.ancestors().filter(|n| n.is_element()) {
        let value = if ancestor.tag_name().namespace() == Some(XSL) {
            ancestor.attribute("default-mode")
        } else {
            ancestor.attribute((XSL, "default-mode"))
        };
        if let Some(value) = value {
            if value.trim() == "#unnamed" {
                return Ok(String::new());
            }
            let name = component_name(ancestor, value)?;
            check_component_namespace(ancestor, &name, false)?;
            return Ok(name);
        }
    }
    Ok(String::new())
}
fn mode(n: Node<'_, '_>, value: &str, current: bool) -> Result<String, Diagnostic> {
    let value = value.trim();
    if value == "#default" {
        return default_mode(n);
    }
    if value == "#unnamed" {
        return Ok(String::new());
    }
    if current && value == "#current" {
        return Ok(value.into());
    }
    let name = component_name(n, value)?;
    check_component_namespace(n, &name, false)?;
    Ok(name)
}
struct OutputProperty {
    precedence: u32,
    value: String,
    conflict: Option<Diagnostic>,
}

#[derive(Default)]
struct OutputGroup {
    properties: BTreeMap<String, OutputProperty>,
    cdata_elements: Vec<String>,
    character_maps: Vec<String>,
}

fn apply_output_properties(
    sheet: &mut crate::result::Serialization,
    properties: &BTreeMap<String, OutputProperty>,
) -> Result<(), Diagnostic> {
    for property in properties.values() {
        if let Some(error) = &property.conflict {
            return Err(error.clone());
        }
    }
    let get = |name: &str| properties.get(name).map(|property| property.value.as_str());
    let method = get("method").unwrap_or("xml");
    sheet.text_output = method == "text";
    sheet.markup = match method {
        "xml" | "text" => crate::result::Markup::Xml,
        "html" => crate::result::Markup::Html,
        "xhtml" => crate::result::Markup::Xhtml,
        _ => {
            return Err(Diagnostic::new(
                "RUST_XSLT_UNSUPPORTED",
                "unsupported output method",
                "compile",
            ));
        }
    };
    if method == "xml" && get("version").is_some_and(|version| version != "1.0") {
        return Err(Diagnostic::new(
            "RUST_XSLT_UNSUPPORTED",
            "only XML 1.0 output is currently supported",
            "compile",
        ));
    }
    sheet.encoding = get("encoding")
        .and_then(crate::result::Encoding::parse)
        .unwrap_or_default();
    sheet.indent = get("indent") == Some("yes");
    sheet.escape_uri_attributes = get("escape-uri-attributes") != Some("no");
    sheet.include_content_type = get("include-content-type") != Some("no");
    sheet.media_type = get("media-type").map(str::to_owned);
    sheet.byte_order_mark = get("byte-order-mark") == Some("yes");
    sheet.omit_specified = properties.contains_key("omit-xml-declaration");
    sheet.omit_declaration =
        get("omit-xml-declaration").map_or(method == "html", |value| value == "yes");
    sheet.standalone = match get("standalone") {
        None | Some("omit") => None,
        value => Some(value == Some("yes")),
    };
    if get("doctype-public").is_some() || get("doctype-system").is_some() {
        sheet.doctype = Some(crate::result::Doctype {
            public: get("doctype-public").map(str::to_owned),
            system: get("doctype-system").map(str::to_owned),
        });
    }
    Ok(())
}

struct Declarations<'a> {
    sheet: &'a mut Stylesheet,
    output_properties: &'a mut BTreeMap<String, OutputGroup>,
    functions: &'a mut BTreeMap<(String, usize), xpath::UserFunction>,
    keys: &'a mut BTreeMap<String, Vec<xpath::KeyDefinition>>,
    attribute_sets: &'a mut BTreeMap<String, AttributeSet>,
    character_maps: &'a mut BTreeMap<String, CharacterMap>,
    decimal_formats: &'a mut BTreeMap<String, crate::format_number::Format>,
    counter: &'a mut u32,
    global_precedence: &'a mut BTreeMap<String, u32>,
    function_precedence: &'a mut BTreeMap<(String, usize), u32>,
}
type CharacterMap = (Vec<String>, Vec<(char, String)>);
type Resolver<'a> = &'a dyn Fn(&str) -> Result<String, Diagnostic>;
fn declarations(
    root: Node<'_, '_>,
    state: &mut Declarations<'_>,
    resolver: Option<Resolver<'_>>,
    depth: usize,
    inherited: Option<u32>,
) -> Result<(), Diagnostic> {
    let Declarations {
        sheet,
        output_properties,
        functions,
        keys,
        attribute_sets,
        character_maps,
        decimal_formats,
        counter,
        global_precedence,
        function_precedence,
    } = state;
    // Imported modules rank below this module and earlier imports rank below later ones.
    for n in root.children() {
        if !(n.is_element()
            && n.tag_name().namespace() == Some(XSL)
            && n.tag_name().name() == "import")
        {
            continue;
        }
        if inherited.is_some() {
            return Err(error(
                n,
                "xsl:import inside an included module is not supported",
            ));
        }
        attrs(n, &["href"])?;
        empty(n)?;
        let href = required(n, "href")?;
        let Some(resolver) = resolver else {
            return Err(error(n, "xsl:import needs a resource resolver"));
        };
        if depth >= 32 {
            return Err(static_error(n, "XTSE0210", "stylesheet import is too deep"));
        }
        let imported = resolver(href)?;
        let doc = crate::parse(&imported, "compile")?;
        let imported_root = doc.root_element();
        if imported_root.tag_name().namespace() != Some(XSL)
            || !matches!(imported_root.tag_name().name(), "stylesheet" | "transform")
        {
            return Err(error(
                imported_root,
                "imported module must be xsl:stylesheet or xsl:transform",
            ));
        }
        attrs(imported_root, &["version"])?;
        declarations(
            imported_root,
            &mut Declarations {
                sheet,
                output_properties,
                functions,
                keys,
                attribute_sets,
                character_maps,
                decimal_formats,
                counter,
                global_precedence,
                function_precedence,
            },
            Some(resolver),
            depth + 1,
            None,
        )?;
    }
    let precedence = inherited.unwrap_or_else(|| {
        let own = **counter;
        **counter += 1;
        own
    });
    let mut other_seen = false;
    for n in root.children() {
        if n.is_text() && !xml_space(n.text().unwrap_or_default()) {
            return Err(error(n, "unexpected top-level text"));
        }
        if !n.is_element() {
            continue;
        }
        if n.tag_name().namespace() != Some(XSL) {
            return Err(error(n, "unsupported top-level element"));
        }
        if use_when_excludes(n)? {
            continue;
        }
        if n.tag_name().name() == "import" {
            if other_seen {
                return Err(static_error(
                    n,
                    "XTSE0200",
                    "xsl:import must precede all other declarations",
                ));
            }
            continue;
        }
        other_seen = true;
        match n.tag_name().name() {
            "mode" => {
                use crate::mode::OnNoMatch;
                attrs(
                    n,
                    &[
                        "name",
                        "on-no-match",
                        "on-multiple-match",
                        "warning-on-multiple-match",
                        "streamable",
                        "use-accumulators",
                        "visibility",
                    ],
                )?;
                empty(n)?;
                if boolean_attribute(n, "streamable", false)? {
                    return Err(error(n, "XSLT streamable modes are not implemented"));
                }
                if n.attribute("use-accumulators")
                    .is_some_and(|s| !s.trim().is_empty())
                {
                    return Err(error(n, "mode accumulators are not implemented"));
                }
                let name = n
                    .attribute("name")
                    .map(|s| component_name(n, s))
                    .transpose()?
                    .unwrap_or_default();
                check_component_namespace(n, &name, false)?;
                let policy = n
                    .attribute("on-no-match")
                    .map(|s| {
                        Ok(match s.trim() {
                            "text-only-copy" => OnNoMatch::TextOnlyCopy,
                            "shallow-copy" => OnNoMatch::ShallowCopy,
                            "deep-copy" => OnNoMatch::DeepCopy,
                            "shallow-skip" => OnNoMatch::ShallowSkip,
                            "deep-skip" => OnNoMatch::DeepSkip,
                            "fail" => OnNoMatch::Fail,
                            _ => {
                                return Err(static_error(
                                    n,
                                    "XTSE0020",
                                    "invalid on-no-match policy",
                                ));
                            }
                        })
                    })
                    .transpose()?;
                let multiple = n
                    .attribute("on-multiple-match")
                    .map(|s| match s.trim() {
                        "fail" => Ok(true),
                        "use-last" => Ok(false),
                        _ => Err(static_error(
                            n,
                            "XTSE0020",
                            "invalid on-multiple-match policy",
                        )),
                    })
                    .transpose()?;
                let _warning_on_multiple_match =
                    boolean_attribute(n, "warning-on-multiple-match", true)?;
                let visibility = n
                    .attribute("visibility")
                    .map(|s| match s.trim() {
                        "public" => Ok(crate::mode::Visibility::Public),
                        "private" => Ok(crate::mode::Visibility::Private),
                        "final" => Ok(crate::mode::Visibility::Final),
                        "abstract" => Err(error(n, "abstract modes require package support")),
                        _ => Err(static_error(n, "XTSE0020", "invalid mode visibility")),
                    })
                    .transpose()?;
                if name.is_empty()
                    && visibility.is_some_and(|v| v != crate::mode::Visibility::Private)
                {
                    return Err(static_error(
                        n,
                        "XTSE0020",
                        "the unnamed mode must be private",
                    ));
                }
                let entry = sheet.modes.entry(name).or_default();
                if policy.zip(entry.on_no_match).is_some_and(|(a, b)| a != b)
                    || multiple
                        .zip(entry.fail_on_multiple)
                        .is_some_and(|(a, b)| a != b)
                    || visibility
                        .zip(entry.visibility)
                        .is_some_and(|(a, b)| a != b)
                {
                    return Err(static_error(n, "XTSE0545", "conflicting mode declarations"));
                }
                entry.on_no_match = policy.or(entry.on_no_match);
                entry.fail_on_multiple = multiple.or(entry.fail_on_multiple);
                entry.visibility = visibility.or(entry.visibility);
            }
            "strip-space" | "preserve-space" => {
                let rules = space_rules(n, n.tag_name().name() == "strip-space")?;
                for rule in &rules {
                    if sheet.space_rules.iter().any(|r| {
                        r.specificity == rule.specificity
                            && r.strip != rule.strip
                            && r.uri == rule.uri
                            && r.local == rule.local
                    }) {
                        let mut e = error(n, "conflicting whitespace declarations");
                        e.code = "XTSE0270".into();
                        return Err(e);
                    }
                }
                sheet.space_rules.extend(rules);
            }
            "param" | "variable" => {
                let b = binding(n, n.tag_name().name() == "param", 0)?;
                if n.attribute("static")
                    .is_some_and(|s| matches!(s.trim(), "yes" | "true" | "1"))
                {
                    // Static variables are evaluated now, and also remain ordinary globals.
                    let value = match n.attribute("select") {
                        Some(select) => static_value(n, select)?,
                        None => vec![xpath::Item::String(String::new())],
                    };
                    STATIC_VARIABLES.with(|v| v.borrow_mut().insert(b.name.clone(), value));
                }
                match global_precedence.get(&b.name) {
                    Some(existing) if *existing == precedence => {
                        return Err(static_error(n, "XTSE0630", "duplicate global binding"));
                    }
                    Some(_) => sheet.globals.retain(|g| g.name != b.name),
                    None => {}
                }
                global_precedence.insert(b.name.clone(), precedence);
                sheet.globals.push(b);
            }
            "template" => {
                attrs(n, &["match", "name", "mode", "priority", "as"])?;
                let p = n
                    .attribute("match")
                    .map(|p| crate::pattern::parse(p, namespaces(n), xslt_version(n)))
                    .transpose()?;
                let id = n
                    .attribute("name")
                    .map(|s| component_name(n, s))
                    .transpose()?;
                if let Some(id) = &id {
                    check_component_namespace(n, id, true)?;
                }
                if p.is_none() && id.is_none() {
                    return Err(error(n, "template requires match or name"));
                }
                if let Some(id) = &id {
                    for existing in sheet.templates.iter_mut() {
                        if existing.name.as_ref() == Some(id) {
                            if existing.precedence == precedence {
                                return Err(static_error(
                                    n,
                                    "XTSE0660",
                                    "duplicate named template",
                                ));
                            }
                            // The importing module's template replaces the imported one.
                            existing.name = None;
                        }
                    }
                }
                let mut params = vec![];
                let mut content = false;
                for c in n.children() {
                    if c.is_element()
                        && c.tag_name().namespace() == Some(XSL)
                        && c.tag_name().name() == "param"
                    {
                        if content {
                            return Err(error(c, "template parameters must precede the body"));
                        }
                        let b = binding(c, true, 0)?;
                        if params.iter().any(|p: &Binding| p.name == b.name) {
                            return Err(static_error(
                                c,
                                "XTSE0580",
                                "duplicate template parameter",
                            ));
                        }
                        params.push(b);
                    } else if c.is_element()
                        || c.is_text() && !xml_space(c.text().unwrap_or_default())
                    {
                        content = true;
                    }
                }
                let modes = if n.attribute("mode").map(str::trim) == Some("#all") {
                    vec!["#all".into()]
                } else {
                    n.attribute("mode")
                        .unwrap_or("#default")
                        .split_whitespace()
                        .map(|m| {
                            mode(n, m, false).map_err(|mut e| {
                                if e.code == "XTSE0020" {
                                    e.code = "XTSE0550".into();
                                }
                                e
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?
                };
                if modes.is_empty() || modes.iter().collect::<BTreeSet<_>>().len() != modes.len() {
                    return Err(static_error(n, "XTSE0550", "empty or duplicate mode list"));
                }
                let priority = n
                    .attribute("priority")
                    .map(|s| {
                        s.parse::<f64>()
                            .map_err(|_| error(n, "invalid template priority"))
                    })
                    .transpose()?;
                if priority.is_some_and(|p| !p.is_finite()) {
                    return Err(error(n, "template priority must be finite"));
                }
                if p.is_none() && (n.attribute("mode").is_some() || priority.is_some()) {
                    return Err(error(n, "mode/priority require a match pattern"));
                }
                let result_type = n
                    .attribute("as")
                    .map(|s| crate::seqtype::SequenceType::parse(s, &namespaces(n)))
                    .transpose()?;
                let body = body(n, 0, true)?;
                if let Some(ty) = &result_type {
                    let unconstrained = matches!(ty.kind, crate::seqtype::Kind::Item)
                        && ty.min == 0
                        && ty.max == usize::MAX;
                    let direct = matches!(
                        body.as_slice(),
                        [Instruction::Sequence(_)] | [Instruction::Document(_)]
                    );
                    if !unconstrained
                        && !direct
                        && !element_result_body(&body)
                        && !raw_result_body(&body)
                    {
                        return Err(error(
                            n,
                            "this template result constructor requires raw XDM sequence collection",
                        ));
                    }
                }
                sheet.templates.push(Template {
                    pattern: p,
                    modes,
                    priority,
                    name: id,
                    params,
                    result_type,
                    body,
                    precedence,
                });
            }
            "output" => {
                attrs(
                    n,
                    &[
                        "name",
                        "method",
                        "omit-xml-declaration",
                        "encoding",
                        "indent",
                        "version",
                        "doctype-public",
                        "doctype-system",
                        "html-version",
                        "media-type",
                        "include-content-type",
                        "escape-uri-attributes",
                        "undeclare-prefixes",
                        "byte-order-mark",
                        "standalone",
                        "cdata-section-elements",
                        "use-character-maps",
                    ],
                )?;
                let id = n
                    .attribute("name")
                    .map(|name| component_name(n, name))
                    .transpose()?
                    .unwrap_or_default();
                let group = output_properties.entry(id).or_default();
                if let Some(names) = n.attribute("use-character-maps") {
                    group.character_maps.extend(
                        names
                            .split_whitespace()
                            .map(|name| component_name(n, name))
                            .collect::<Result<Vec<_>, _>>()?,
                    );
                }
                if let Some(names) = n.attribute("cdata-section-elements") {
                    let all = &mut group.cdata_elements;
                    for name in names.split_whitespace() {
                        let mut expanded = component_name(n, name)?;
                        if !name.contains(':')
                            && !name.starts_with("Q{")
                            && let Some(uri) = n.lookup_namespace_uri(None)
                        {
                            expanded = format!("Q{{{uri}}}{name}");
                        }
                        if !all.contains(&expanded) {
                            all.push(expanded);
                        }
                    }
                }
                empty(n)?;
                for attribute in n
                    .attributes()
                    .filter(|attribute| attribute.namespace().is_none())
                {
                    let name = attribute.name();
                    if [
                        "name",
                        "use-character-maps",
                        "cdata-section-elements",
                        "use-when",
                        "expand-text",
                        "default-mode",
                        "exclude-result-prefixes",
                        "xpath-default-namespace",
                    ]
                    .contains(&name)
                    {
                        continue;
                    }
                    let value = match name {
                        "indent"
                        | "omit-xml-declaration"
                        | "byte-order-mark"
                        | "include-content-type"
                        | "escape-uri-attributes"
                        | "undeclare-prefixes" => {
                            if boolean_attribute(n, name, false)? {
                                "yes".into()
                            } else {
                                "no".into()
                            }
                        }
                        "standalone" => match standalone(n)? {
                            Some(true) => "yes",
                            Some(false) => "no",
                            None => "omit",
                        }
                        .into(),
                        "encoding" => crate::result::Encoding::parse(attribute.value().trim())
                            .ok_or_else(|| {
                                error(
                                    n,
                                    "only UTF-8, ISO-8859-1 and US-ASCII output are supported",
                                )
                            })?
                            .name()
                            .into(),
                        _ => attribute.value().trim().to_owned(),
                    };
                    match group.properties.get_mut(name) {
                        Some(previous) if previous.precedence > precedence => {}
                        Some(previous) if previous.precedence == precedence => {
                            if previous.value != value {
                                previous.conflict = Some(static_error(
                                    n,
                                    "XTSE1560",
                                    format!("conflicting output property: {name}"),
                                ));
                            }
                        }
                        _ => {
                            group.properties.insert(
                                name.to_owned(),
                                OutputProperty {
                                    precedence,
                                    value,
                                    conflict: None,
                                },
                            );
                        }
                    }
                }
            }
            "include" => {
                attrs(n, &["href"])?;
                empty(n)?;
                let href = required(n, "href")?;
                let Some(resolver) = resolver else {
                    return Err(error(n, "xsl:include needs a resource resolver"));
                };
                if depth >= 32 {
                    return Err(static_error(
                        n,
                        "XTSE0180",
                        "stylesheet inclusion is too deep",
                    ));
                }
                let included = resolver(href)?;
                let doc = crate::parse(&included, "compile")?;
                let included_root = doc.root_element();
                if included_root.tag_name().namespace() != Some(XSL)
                    || !matches!(included_root.tag_name().name(), "stylesheet" | "transform")
                {
                    return Err(error(
                        included_root,
                        "included module must be xsl:stylesheet or xsl:transform",
                    ));
                }
                attrs(included_root, &["version"])?;
                declarations(
                    included_root,
                    &mut Declarations {
                        sheet,
                        output_properties,
                        functions,
                        keys,
                        attribute_sets,
                        character_maps,
                        decimal_formats,
                        counter,
                        global_precedence,
                        function_precedence,
                    },
                    Some(resolver),
                    depth + 1,
                    Some(precedence),
                )?;
            }
            "decimal-format" => {
                attrs(
                    n,
                    &[
                        "name",
                        "decimal-separator",
                        "grouping-separator",
                        "infinity",
                        "minus-sign",
                        "NaN",
                        "percent",
                        "per-mille",
                        "zero-digit",
                        "digit",
                        "pattern-separator",
                        "exponent-separator",
                    ],
                )?;
                empty(n)?;
                let id = match n.attribute("name") {
                    Some(name) => component_name(n, name)?,
                    None => String::new(),
                };
                let mut format = crate::format_number::Format::default();
                let single = |name: &str, target: &mut char| -> Result<(), Diagnostic> {
                    if let Some(value) = n.attribute(name) {
                        let mut chars = value.chars();
                        match (chars.next(), chars.next()) {
                            (Some(c), None) => *target = c,
                            _ => {
                                return Err(static_error(
                                    n,
                                    "XTSE0020",
                                    format!("{name} must be a single character"),
                                ));
                            }
                        }
                    }
                    Ok(())
                };
                single("decimal-separator", &mut format.decimal)?;
                single("grouping-separator", &mut format.grouping)?;
                single("minus-sign", &mut format.minus)?;
                single("percent", &mut format.percent)?;
                single("per-mille", &mut format.permille)?;
                single("zero-digit", &mut format.zero)?;
                single("digit", &mut format.digit)?;
                single("pattern-separator", &mut format.pattern_separator)?;
                single("exponent-separator", &mut format.exponent)?;
                if let Some(v) = n.attribute("infinity") {
                    format.infinity = v.into();
                }
                if let Some(v) = n.attribute("NaN") {
                    format.nan = v.into();
                }
                if !(0..10)
                    .all(|d| char::from_u32(format.zero as u32 + d).is_some_and(|c| c.is_numeric()))
                {
                    return Err(static_error(
                        n,
                        "XTSE1295",
                        "zero-digit must start a run of ten digits",
                    ));
                }
                let mut roles = format.property_chars().to_vec();
                roles.push(format.zero);
                roles.sort_unstable();
                if roles.windows(2).any(|w| w[0] == w[1]) {
                    return Err(static_error(
                        n,
                        "XTSE1300",
                        "decimal-format properties must use distinct characters",
                    ));
                }
                if let Some(existing) = decimal_formats.get(&id)
                    && *existing != format
                {
                    return Err(static_error(
                        n,
                        "XTSE1290",
                        "conflicting decimal-format declarations",
                    ));
                }
                decimal_formats.insert(id, format);
            }
            "character-map" => {
                attrs(n, &["name", "use-character-maps"])?;
                let id = component_name(
                    n,
                    n.attribute("name").ok_or_else(|| {
                        static_error(n, "XTSE0010", "character-map requires a name")
                    })?,
                )?;
                if character_maps.contains_key(&id) {
                    return Err(static_error(n, "XTSE1580", "duplicate character map"));
                }
                let uses = n
                    .attribute("use-character-maps")
                    .map(|s| set_names(n, s))
                    .transpose()?
                    .unwrap_or_default();
                let mut entries = vec![];
                for c in n.children() {
                    if c.is_text() && !xml_space(c.text().unwrap_or_default()) {
                        return Err(static_error(
                            c,
                            "XTSE0010",
                            "unexpected text in xsl:character-map",
                        ));
                    }
                    if !c.is_element() {
                        continue;
                    }
                    if c.tag_name().namespace() != Some(XSL)
                        || c.tag_name().name() != "output-character"
                    {
                        return Err(static_error(c, "XTSE0010", "expected xsl:output-character"));
                    }
                    attrs(c, &["character", "string"])?;
                    let mut chars = required(c, "character")?.chars();
                    let (Some(ch), None) = (chars.next(), chars.next()) else {
                        return Err(static_error(
                            c,
                            "XTSE0020",
                            "character must be a single character",
                        ));
                    };
                    entries.push((ch, required(c, "string")?.to_owned()));
                }
                character_maps.insert(id, (uses, entries));
            }
            "attribute-set" => {
                attrs(
                    n,
                    &["name", "use-attribute-sets", "visibility", "streamable"],
                )?;
                let id = component_name(
                    n,
                    n.attribute("name").ok_or_else(|| {
                        static_error(n, "XTSE0010", "attribute-set requires a name")
                    })?,
                )?;
                let uses = n
                    .attribute("use-attribute-sets")
                    .map(|s| set_names(n, s))
                    .transpose()?
                    .unwrap_or_default();
                let content = body(n, 1, false)?;
                if content
                    .iter()
                    .any(|i| !matches!(i, Instruction::Attribute(..)))
                {
                    return Err(static_error(
                        n,
                        "XTSE0010",
                        "attribute sets may only contain xsl:attribute",
                    ));
                }
                let entry = attribute_sets.entry(id).or_default();
                entry.uses.extend(uses);
                entry.body.extend(content);
            }
            "key" => {
                attrs(
                    n,
                    &["name", "match", "use", "collation", "composite", "version"],
                )?;
                let id = component_name(
                    n,
                    n.attribute("name")
                        .ok_or_else(|| static_error(n, "XTSE0010", "key requires a name"))?,
                )?;
                let composite = boolean_attribute(n, "composite", false)?;
                let collation_uri = n
                    .attribute("collation")
                    .unwrap_or("http://www.w3.org/2005/xpath-functions/collation/codepoint")
                    .trim()
                    .to_owned();
                let collation = crate::collation::Collation::parse(&collation_uri)
                    .map_err(|_| static_error(n, "XTSE1210", "unsupported key collation"))?;
                if let Some(previous) = keys.get(&id).and_then(|definitions| definitions.first()) {
                    if previous.composite != composite {
                        return Err(static_error(
                            n,
                            "XTSE1222",
                            "key declarations disagree on composite",
                        ));
                    }
                    if previous.collation_uri != collation_uri {
                        return Err(static_error(
                            n,
                            "XTSE1220",
                            "key declarations disagree on collation",
                        ));
                    }
                }
                let pattern = n
                    .attribute("match")
                    .ok_or_else(|| static_error(n, "XTSE0010", "key requires a match pattern"))?;
                let branches = crate::pattern::parse(pattern, namespaces(n), xslt_version(n))?;
                let Some(select) = n.attribute("use") else {
                    return Err(error(n, "xsl:key requires the use attribute here"));
                };
                empty(n)?;
                keys.entry(id).or_default().push(xpath::KeyDefinition {
                    branches,
                    value: expr(n, select)?,
                    composite,
                    collation,
                    collation_uri,
                    backwards: n
                        .ancestors()
                        .find_map(|ancestor| ancestor.attribute("version"))
                        .and_then(|version| version.parse::<f64>().ok())
                        .is_some_and(|version| version < 2.0),
                });
            }
            "function" => {
                attrs(
                    n,
                    &[
                        "name",
                        "as",
                        "visibility",
                        "override",
                        "override-extension-function",
                        "new-each-time",
                        "cache",
                    ],
                )?;
                let id = component_name(
                    n,
                    n.attribute("name")
                        .ok_or_else(|| static_error(n, "XTSE0010", "function requires a name"))?,
                )?;
                if crate::qname::parts(&id).0.is_none_or(str::is_empty) {
                    return Err(static_error(
                        n,
                        "XTSE0080",
                        "function names need a namespace",
                    ));
                }
                check_component_namespace(n, &id, false)?;
                let result = n
                    .attribute("as")
                    .map(|s| crate::seqtype::SequenceType::parse(s, &namespaces(n)))
                    .transpose()?;
                let mut params: Vec<(String, Option<crate::seqtype::SequenceType>)> = vec![];
                let mut content = false;
                for c in n.children() {
                    if c.is_element()
                        && c.tag_name().namespace() == Some(XSL)
                        && c.tag_name().name() == "param"
                    {
                        if content {
                            return Err(error(c, "function parameters must precede the body"));
                        }
                        attrs(c, &["name", "as"])?;
                        let name = component_name(
                            c,
                            c.attribute("name").ok_or_else(|| {
                                static_error(c, "XTSE0010", "param requires a name")
                            })?,
                        )?;
                        if params.iter().any(|(p, _)| *p == name) {
                            return Err(static_error(
                                c,
                                "XTSE0580",
                                "duplicate function parameter",
                            ));
                        }
                        let ty = c
                            .attribute("as")
                            .map(|s| crate::seqtype::SequenceType::parse(s, &namespaces(c)))
                            .transpose()?;
                        empty(c)?;
                        params.push((name, ty));
                    } else if c.is_element()
                        || c.is_text() && !xml_space(c.text().unwrap_or_default())
                    {
                        content = true;
                    }
                }
                let mut bindings = vec![];
                let mut result_expr = None;
                for instruction in body(n, 1, true)? {
                    match instruction {
                        Instruction::Bind(Binding {
                            name,
                            value: BindingValue::Select(select),
                            ..
                        }) if result_expr.is_none() => bindings.push((name, select)),
                        Instruction::Sequence(e) if result_expr.is_none() => result_expr = Some(e),
                        _ => {
                            return Err(error(
                                n,
                                "function bodies are limited to variables followed by xsl:sequence",
                            ));
                        }
                    }
                }
                let mut function_body = result_expr.unwrap_or(Expr::Sequence(vec![]));
                for (name, select) in bindings.into_iter().rev() {
                    function_body = Expr::Let(name, Box::new(select), Box::new(function_body));
                }
                let key = (id, params.len());
                if function_precedence.get(&key) == Some(&precedence) {
                    return Err(static_error(
                        n,
                        "XTSE0770",
                        "duplicate function declaration",
                    ));
                }
                function_precedence.insert(key.clone(), precedence);
                functions.insert(
                    key,
                    xpath::UserFunction {
                        params,
                        body: function_body,
                        result,
                    },
                );
            }
            other => return Err(error(n, format!("unsupported declaration: {other}"))),
        }
    }
    Ok(())
}
pub(crate) fn compile(
    source: &str,
    resolver: Option<Resolver<'_>>,
) -> Result<Stylesheet, Diagnostic> {
    xpath::take_document_uris();
    STATIC_VARIABLES.with(|v| v.borrow_mut().clear());
    let doc = crate::parse(source, "compile")?;
    let root = doc.root_element();
    if root.tag_name().namespace() != Some(XSL)
        || !matches!(root.tag_name().name(), "stylesheet" | "transform")
    {
        return Err(error(root, "expected xsl:stylesheet or xsl:transform"));
    }
    attrs(root, &["version"])?;
    if !matches!(root.attribute("version"), Some("1.0" | "2.0" | "3.0")) {
        return Err(error(root, "expected version 1.0, 2.0, or 3.0"));
    }
    let mut sheet = Stylesheet {
        templates: vec![],
        globals: vec![],
        functions: Default::default(),
        keys: Default::default(),
        xpath_only: xpath::xpath_only_profile(),
        attribute_sets: BTreeMap::new(),
        decimal_formats: Default::default(),
        document_uris: vec![],
        outputs: BTreeMap::new(),
        space_rules: vec![],
        default_mode: default_mode(root)?,
        modes: BTreeMap::new(),
        omit_declaration: false,
        standalone: None,
        byte_order_mark: false,
        indent: false,
        markup: crate::result::Markup::Xml,
        escape_uri_attributes: true,
        include_content_type: true,
        media_type: None,
        encoding: Default::default(),
        cdata_elements: Default::default(),
        character_maps: Default::default(),
        char_map: Default::default(),
        doctype: None,
        text_output: false,
        backwards: root.attribute("version") == Some("1.0"),
        version: match root.attribute("version") {
            Some("1.0") => 1,
            Some("2.0") => 2,
            _ => 3,
        },
    };
    let mut output_properties = BTreeMap::new();
    let mut functions: BTreeMap<(String, usize), xpath::UserFunction> = BTreeMap::new();
    let mut keys: BTreeMap<String, Vec<xpath::KeyDefinition>> = BTreeMap::new();
    let mut attribute_sets: BTreeMap<String, AttributeSet> = BTreeMap::new();
    let mut character_maps: BTreeMap<String, CharacterMap> = BTreeMap::new();
    let mut decimal_formats: BTreeMap<String, crate::format_number::Format> = BTreeMap::new();
    let mut precedence_counter = 0u32;
    let mut global_precedence: BTreeMap<String, u32> = BTreeMap::new();
    let mut function_precedence: BTreeMap<(String, usize), u32> = BTreeMap::new();
    declarations(
        root,
        &mut Declarations {
            sheet: &mut sheet,
            output_properties: &mut output_properties,
            functions: &mut functions,
            keys: &mut keys,
            attribute_sets: &mut attribute_sets,
            character_maps: &mut character_maps,
            decimal_formats: &mut decimal_formats,
            counter: &mut precedence_counter,
            global_precedence: &mut global_precedence,
            function_precedence: &mut function_precedence,
        },
        resolver,
        0,
        None,
    )?;
    for name in character_maps.keys() {
        sheet.character_maps.insert(
            name.clone(),
            resolve_character_maps(&character_maps, std::slice::from_ref(name))?,
        );
    }
    output_properties.entry(String::new()).or_default();
    for (name, group) in output_properties {
        let mut settings = crate::result::Serialization::default();
        apply_output_properties(&mut settings, &group.properties)?;
        settings.cdata_elements = std::sync::Arc::new(group.cdata_elements);
        settings.char_map = std::sync::Arc::new(resolve_character_maps(
            &character_maps,
            &group.character_maps,
        )?);
        sheet.outputs.insert(name, settings);
    }
    let primary = &sheet.outputs[""];
    sheet.text_output = primary.text_output;
    sheet.markup = primary.markup;
    sheet.escape_uri_attributes = primary.escape_uri_attributes;
    sheet.include_content_type = primary.include_content_type;
    sheet.media_type = primary.media_type.clone();
    sheet.encoding = primary.encoding;
    sheet.omit_declaration = primary.omit_declaration;
    sheet.standalone = primary.standalone;
    sheet.byte_order_mark = primary.byte_order_mark;
    sheet.indent = primary.indent;
    sheet.doctype = primary.doctype.clone();
    sheet.cdata_elements = primary.cdata_elements.clone();
    sheet.char_map = primary.char_map.clone();
    sheet.functions = std::sync::Arc::new(functions);
    sheet.keys = std::sync::Arc::new(keys);
    sheet.attribute_sets = attribute_sets;
    sheet.decimal_formats = std::sync::Arc::new(decimal_formats);
    sheet.document_uris = xpath::take_document_uris();
    validate(&sheet)?;
    Ok(sheet)
}

// Check lexical references even in branches that will not execute.
fn validate(sheet: &Stylesheet) -> Result<(), Diagnostic> {
    use std::collections::BTreeSet;
    type Scope = BTreeSet<String>;
    struct TemplateNames<'a> {
        named: BTreeMap<String, &'a Template>,
        attribute_sets: BTreeSet<String>,
        character_maps: BTreeSet<String>,
        backwards: bool,
    }
    fn expression(e: &Expr, scope: &Scope) -> Result<(), Diagnostic> {
        let mut pending = vec![e];
        while let Some(e) = pending.pop() {
            match e {
                Expr::Variable(n) if !scope.contains(n) => {
                    return Err(Diagnostic::new(
                        "XPST0008",
                        format!("unknown variable: {n}"),
                        "compile",
                    ));
                }
                Expr::For(name, select, body)
                | Expr::Let(name, select, body)
                | Expr::Quantified(_, name, select, body) => {
                    expression(select, scope)?;
                    let mut inner = scope.clone();
                    inner.insert(name.clone());
                    expression(body, &inner)?;
                }
                Expr::Path(base, steps) => {
                    pending.push(base);
                    for s in steps {
                        pending.extend(&s.predicates);
                    }
                }
                Expr::Filter(base, ps) => {
                    pending.push(base);
                    pending.extend(ps);
                }
                Expr::Binary(_, a, b) | Expr::PathMap(a, b) => {
                    pending.push(a);
                    pending.push(b);
                }
                Expr::If(c, y, n) => {
                    pending.push(c);
                    pending.push(y);
                    pending.push(n);
                }
                Expr::Unary(_, e) | Expr::TypeOp(_, e, _) | Expr::QNameCast(e, _, _, _) => {
                    pending.push(e)
                }
                Expr::Call(_, args) | Expr::Sequence(args) | Expr::ArraySquare(args) => {
                    pending.extend(args)
                }
                Expr::InlineFunction(def) => {
                    let mut inner = scope.clone();
                    inner.extend(def.params.iter().map(|(name, _)| name.clone()));
                    expression(&def.body, &inner)?;
                }
                Expr::DynamicCall(callee, args) => {
                    pending.push(callee);
                    pending.extend(args);
                }
                Expr::Partial(callee, slots) => {
                    pending.push(callee);
                    pending.extend(slots.iter().flatten());
                }
                Expr::MapConstructor(entries) => {
                    for (key, value) in entries {
                        pending.push(key);
                        pending.push(value);
                    }
                }
                Expr::ArrayCurly(inner) => pending.push(inner),
                Expr::Lookup(base, key) => {
                    pending.push(base);
                    if let Some(key) = key {
                        pending.push(key);
                    }
                }
                Expr::Sorted(inner, keys) => {
                    pending.push(inner);
                    for key in keys {
                        pending.push(&key.select);
                    }
                }
                Expr::Evaluate {
                    xpath,
                    context,
                    params,
                    ..
                } => {
                    pending.push(xpath);
                    if let Some(context) = context {
                        pending.push(context);
                    }
                    pending.extend(params.iter().map(|(_, e)| e));
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn instructions(
        body: &[Instruction],
        mut scope: Scope,
        templates: &TemplateNames<'_>,
    ) -> Result<(), Diagnostic> {
        fn parts(es: &[Expr], scope: &Scope) -> Result<(), Diagnostic> {
            for e in es {
                expression(e, scope)?;
            }
            Ok(())
        }
        fn cname(n: &ComputedName, scope: &Scope) -> Result<(), Diagnostic> {
            parts(&n.parts, scope)?;
            if let Some(es) = &n.namespace {
                parts(es, scope)?;
            }
            Ok(())
        }
        fn value(
            c: &Content,
            scope: &Scope,
            templates: &TemplateNames<'_>,
        ) -> Result<(), Diagnostic> {
            match c {
                Content::Select(e, sep) => {
                    expression(e, scope)?;
                    parts(sep, scope)
                }
                Content::Body(body, sep) => {
                    parts(sep, scope)?;
                    instructions(body, scope.clone(), templates)
                }
            }
        }
        for i in body {
            match i {
                Instruction::Value(_, Some(es)) => parts(es, &scope)?,
                Instruction::Apply(_, _, keys, _) | Instruction::ForEach(_, keys, _) => {
                    for k in keys {
                        expression(&k.select, &scope)?;
                    }
                }
                _ => {}
            }
            if let Instruction::Apply(_, _, _, params)
            | Instruction::NextMatch(params)
            | Instruction::ApplyImports(params) = i
            {
                for p in params {
                    match &p.value {
                        BindingValue::Select(e) => expression(e, &scope)?,
                        BindingValue::Body(body)
                        | BindingValue::Document(body)
                        | BindingValue::Raw(body) => instructions(body, scope.clone(), templates)?,
                    }
                }
            }
            match i {
                Instruction::ComputedElement(n, _, body) => {
                    cname(n, &scope)?;
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Attribute(n, c) => {
                    cname(n, &scope)?;
                    value(c, &scope, templates)?;
                }
                Instruction::AnalyzeString {
                    select,
                    regex,
                    flags,
                    matching,
                    non_matching,
                } => {
                    expression(select, &scope)?;
                    parts(regex, &scope)?;
                    parts(flags, &scope)?;
                    instructions(matching, scope.clone(), templates)?;
                    instructions(non_matching, scope.clone(), templates)?;
                }
                Instruction::Iterate {
                    select,
                    params,
                    on_completion,
                    body,
                } => {
                    expression(select, &scope)?;
                    let mut inner = scope.clone();
                    for p in params {
                        if let BindingValue::Select(e) = &p.value {
                            expression(e, &scope)?;
                        }
                        inner.insert(p.name.clone());
                    }
                    instructions(on_completion, inner.clone(), templates)?;
                    instructions(body, inner, templates)?;
                }
                Instruction::Number(spec) => {
                    for e in [&spec.value, &spec.select].into_iter().flatten() {
                        expression(e, &scope)?;
                    }
                    parts(&spec.format, &scope)?;
                    parts(&spec.ordinal, &scope)?;
                    parts(&spec.grouping_separator, &scope)?;
                    parts(&spec.grouping_size, &scope)?;
                }
                Instruction::WherePopulated(body)
                | Instruction::OnEmpty(body)
                | Instruction::OnNonEmpty(body) => {
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Populated {
                    main,
                    on_empty,
                    on_non_empty,
                } => {
                    instructions(main, scope.clone(), templates)?;
                    instructions(on_empty, scope.clone(), templates)?;
                    instructions(on_non_empty, scope.clone(), templates)?;
                }
                Instruction::ResultDocument(spec) => {
                    for name in &spec.character_maps {
                        if !templates.character_maps.contains(name) {
                            return Err(Diagnostic::new(
                                "XTSE1590",
                                format!("unknown character map: {name}"),
                                "compile",
                            ));
                        }
                    }
                    parts(&spec.href, &scope)?;
                    for value in [
                        &spec.doctype_public,
                        &spec.doctype_system,
                        &spec.indent,
                        &spec.escape_uri_attributes,
                        &spec.include_content_type,
                        &spec.media_type,
                        &spec.omit_declaration,
                        &spec.byte_order_mark,
                        &spec.standalone,
                        &spec.encoding,
                        &spec.method,
                        &spec.output_version,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        parts(value, &scope)?;
                    }
                    if let Some((value, _)) = &spec.cdata {
                        parts(value, &scope)?;
                    }
                    if let Some((format, _)) = &spec.format {
                        parts(format, &scope)?;
                    }
                    instructions(&spec.body, scope.clone(), templates)?;
                }
                Instruction::Namespace(name, content) => {
                    parts(name, &scope)?;
                    value(content, &scope, templates)?;
                }
                Instruction::Try { body, catches } => {
                    instructions(body, scope.clone(), templates)?;
                    for catch in catches {
                        let mut inner = scope.clone();
                        for name in ERROR_VARIABLES {
                            inner.insert(format!("Q{{{ERROR_NAMESPACE}}}{name}"));
                        }
                        instructions(&catch.body, inner, templates)?;
                    }
                }
                Instruction::NextIteration(params) => {
                    for p in params {
                        if let BindingValue::Select(e) = &p.value {
                            expression(e, &scope)?;
                        }
                    }
                }
                Instruction::Break(body) => instructions(body, scope.clone(), templates)?,
                Instruction::UseSets(names) => {
                    for name in names {
                        if !templates.attribute_sets.contains(name) {
                            return Err(Diagnostic::new(
                                "XTSE0710",
                                format!("unknown attribute set: {name}"),
                                "compile",
                            ));
                        }
                    }
                }
                Instruction::LiteralAttributes(attributes) => {
                    for (_, value) in attributes {
                        parts(value, &scope)?;
                    }
                }
                Instruction::Comment(c)
                | Instruction::Message(c, _)
                | Instruction::AssertFailure(c) => value(c, &scope, templates)?,
                Instruction::Pi(n, c) => {
                    parts(n, &scope)?;
                    value(c, &scope, templates)?;
                }
                Instruction::Text(_) | Instruction::NextMatch(_) | Instruction::ApplyImports(_) => {
                }
                Instruction::TextValue(items) => parts(items, &scope)?,
                Instruction::Value(e, _)
                | Instruction::Apply(e, _, _, _)
                | Instruction::CopyOf(e, _)
                | Instruction::Sequence(e) => expression(e, &scope)?,
                Instruction::Bind(b) => {
                    match &b.value {
                        BindingValue::Select(e) => expression(e, &scope)?,
                        BindingValue::Body(body)
                        | BindingValue::Document(body)
                        | BindingValue::Raw(body) => instructions(body, scope.clone(), templates)?,
                    }
                    scope.insert(b.name.clone());
                }
                Instruction::Element {
                    attributes, body, ..
                } => {
                    for (_, parts) in attributes {
                        for e in parts {
                            expression(e, &scope)?;
                        }
                    }
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Document(body) => instructions(body, scope.clone(), templates)?,
                Instruction::Copy { select, body, .. } => {
                    if let Some(e) = select {
                        expression(e, &scope)?;
                    }
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::ForEach(e, _, body) | Instruction::If(e, body) => {
                    expression(e, &scope)?;
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::ForEachGroup(e, grouping, keys, body) => {
                    expression(e, &scope)?;
                    if let Grouping::By(g) | Grouping::Adjacent(g) = grouping {
                        expression(g, &scope)?;
                    }
                    if let Grouping::Starting(branches) | Grouping::Ending(branches) = grouping {
                        for branch in branches {
                            for predicate in crate::pattern::expressions(branch) {
                                expression(predicate, &scope)?;
                            }
                        }
                    }
                    for k in keys {
                        expression(&k.select, &scope)?;
                    }
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Choose(branches) => {
                    for (test, body) in branches {
                        if let Some(e) = test {
                            expression(e, &scope)?;
                        }
                        instructions(body, scope.clone(), templates)?;
                    }
                }
                Instruction::Call(name, params) => {
                    let target = templates.named.get(name).ok_or_else(|| {
                        Diagnostic::new(
                            "XTSE0650",
                            format!("unknown named template: {name}"),
                            "compile",
                        )
                    })?;
                    for p in &target.params {
                        if p.required
                            && !p.tunnel
                            && !params.iter().any(|a| !a.tunnel && a.name == p.name)
                        {
                            return Err(Diagnostic::new(
                                "XTSE0690",
                                format!("required parameter not supplied: {}", p.name),
                                "compile",
                            ));
                        }
                    }
                    if !templates.backwards {
                        for p in params {
                            if !p.tunnel
                                && !target.params.iter().any(|a| !a.tunnel && a.name == p.name)
                            {
                                return Err(Diagnostic::new(
                                    "XTSE0680",
                                    format!("unknown template parameter: {}", p.name),
                                    "compile",
                                ));
                            }
                        }
                    }
                    for p in params {
                        match &p.value {
                            BindingValue::Select(e) => expression(e, &scope)?,
                            BindingValue::Body(body)
                            | BindingValue::Document(body)
                            | BindingValue::Raw(body) => {
                                instructions(body, scope.clone(), templates)?
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    let globals: Scope = sheet.globals.iter().map(|b| b.name.clone()).collect();
    for (name, set) in &sheet.attribute_sets {
        let mut pending: Vec<&String> = set.uses.iter().collect();
        let mut seen = BTreeSet::new();
        while let Some(used) = pending.pop() {
            let Some(inner) = sheet.attribute_sets.get(used) else {
                return Err(Diagnostic::new(
                    "XTSE0710",
                    format!("unknown attribute set: {used}"),
                    "compile",
                ));
            };
            if used == name {
                return Err(Diagnostic::new(
                    "XTSE0720",
                    format!("attribute set uses itself: {name}"),
                    "compile",
                ));
            }
            if seen.insert(used) {
                pending.extend(&inner.uses);
            }
        }
    }
    let templates = TemplateNames {
        attribute_sets: sheet.attribute_sets.keys().cloned().collect(),
        character_maps: sheet.character_maps.keys().cloned().collect(),
        named: sheet
            .templates
            .iter()
            .filter_map(|t| t.name.clone().map(|n| (n, t)))
            .collect(),
        backwards: sheet.backwards,
    };
    for b in &sheet.globals {
        match &b.value {
            BindingValue::Select(e) => expression(e, &globals)?,
            BindingValue::Body(body) | BindingValue::Document(body) | BindingValue::Raw(body) => {
                instructions(body, globals.clone(), &templates)?
            }
        }
    }
    for definitions in sheet.keys.values() {
        for definition in definitions {
            for branch in &definition.branches {
                for predicate in crate::pattern::expressions(branch) {
                    expression(predicate, &globals)?;
                }
            }
            expression(&definition.value, &globals)?;
        }
    }
    for function in sheet.functions.values() {
        let mut scope = globals.clone();
        scope.extend(function.params.iter().map(|(p, _)| p.clone()));
        expression(&function.body, &scope)?;
    }
    for t in &sheet.templates {
        if let Some(branches) = &t.pattern {
            for b in branches {
                for expression_in_pattern in crate::pattern::expressions(b) {
                    expression(expression_in_pattern, &globals)?;
                }
            }
        }
        let mut scope = globals.clone();
        for p in &t.params {
            match &p.value {
                BindingValue::Select(e) => expression(e, &scope)?,
                BindingValue::Body(body)
                | BindingValue::Document(body)
                | BindingValue::Raw(body) => instructions(body, scope.clone(), &templates)?,
            }
            scope.insert(p.name.clone());
        }
        instructions(&t.body, scope, &templates)?;
    }
    Ok(())
}
