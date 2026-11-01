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
    ValueBody(Content),
    Apply(Expr, String, Vec<Sort>, Vec<Binding>),
    Copy {
        select: Option<Expr>,
        namespaces: bool,
        inherit: bool,
        sets: Vec<Instruction>,
        body: Vec<Instruction>,
    },
    CopyOf(Expr, bool),
    Document(Vec<Instruction>),
    Sequence(Expr),
    MapEntry {
        key: Expr,
        body: Vec<Instruction>,
    },
    Map(Vec<Instruction>),
    ComputedElement(ComputedName, bool, Vec<Instruction>),
    Attribute(ComputedName, Content),
    Comment(Content),
    Message(Content, bool),
    /// The failing branch of `xsl:assert`: raises XTMM9001 with the message text.
    AssertFailure(Content),
    Pi(Vec<Expr>, Content),
    Element {
        element: crate::result::Element,
        in_scope_namespaces: BTreeMap<String, String>,
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
    /// `xsl:on-empty` / `xsl:on-non-empty` markers in a conditional constructor.
    OnEmpty(Vec<Instruction>),
    OnNonEmpty(Vec<Instruction>),
    /// A sequence constructor containing conditional content.
    Populated {
        main: Vec<Instruction>,
        has_on_empty: bool,
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
    pub undeclare_prefixes: Option<Vec<Expr>>,
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
    pub letter_value: Option<Vec<Expr>>,
    pub lang: Option<Vec<Expr>>,
    pub grouping_separator: Vec<Expr>,
    pub grouping_size: Vec<Expr>,
    pub start_at: Option<Vec<Expr>>,
}
#[derive(Debug)]
pub(crate) enum Grouping {
    By(Expr, bool),
    Adjacent(Expr, bool),
    Starting(Vec<crate::pattern::Branch>),
    Ending(Vec<crate::pattern::Branch>),
}
/// Sort settings that are attribute value templates, resolved when the sort runs.
#[derive(Debug, Clone, Default)]
pub(crate) struct DynamicSort {
    pub stable: Option<Vec<Expr>>,
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
impl Sort {
    pub(crate) fn expressions(&self) -> impl Iterator<Item = &Expr> {
        std::iter::once(&self.select).chain(
            [
                &self.dynamic.stable,
                &self.dynamic.data_type,
                &self.dynamic.order,
                &self.dynamic.case_order,
                &self.dynamic.collation,
            ]
            .into_iter()
            .flatten()
            .flatten(),
        )
    }
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
    let root = n.document().root_element();
    let version = if root.tag_name().namespace() == Some(XSL) {
        root.attribute("version")
    } else {
        root.attribute((XSL, "version"))
    };
    match version {
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
fn unqualified_attribute<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.attributes()
        .find(|attribute| attribute.namespace().is_none() && attribute.name() == name)
        .map(|attribute| attribute.value())
}
fn required<'a>(n: Node<'a, '_>, a: &str) -> Result<&'a str, Diagnostic> {
    unqualified_attribute(n, a)
        .ok_or_else(|| static_error(n, "XTSE0010", format!("missing required attribute: {a}")))
}

fn choose_branches<'a>(n: Node<'a, 'a>) -> Result<Vec<(Option<Expr>, Node<'a, 'a>)>, Diagnostic> {
    attrs(n, &[])?;
    let mut branches = vec![];
    let mut otherwise = false;
    for c in n.children() {
        if c.is_text() && !xml_space(c.text().unwrap_or_default()) {
            return Err(static_error(c, "XTSE0010", "unexpected text in xsl:choose"));
        }
        if !c.is_element() {
            continue;
        }
        if otherwise || c.tag_name().namespace() != Some(XSL) {
            return Err(static_error(c, "XTSE0010", "invalid xsl:choose branch"));
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
                return Err(static_error(
                    c,
                    "XTSE0010",
                    "xsl:choose requires xsl:when followed by optional xsl:otherwise",
                ));
            }
        };
        branches.push((test, c));
    }
    if branches.is_empty() {
        return Err(static_error(
            n,
            "XTSE0010",
            "xsl:choose requires at least one xsl:when",
        ));
    }
    Ok(branches)
}
thread_local! {
    static MODULE_BASE_URI: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}
fn with_module_base<T>(base: Option<String>, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) {
            MODULE_BASE_URI.with(|value| *value.borrow_mut() = self.0.take());
        }
    }
    let old = MODULE_BASE_URI.with(|value| std::mem::replace(&mut *value.borrow_mut(), base));
    let _restore = Restore(old);
    f()
}
fn stylesheet_base_uri(n: Node<'_, '_>) -> Option<String> {
    let ancestors: Vec<_> = n.ancestors().collect();
    let mut base = MODULE_BASE_URI.with(|value| value.borrow().clone());
    for node in ancestors.into_iter().rev() {
        if let Some(reference) = node.attribute(("http://www.w3.org/XML/1998/namespace", "base")) {
            base = crate::uri_resolution::resolve(reference, base.as_deref()).ok();
        }
    }
    base
}
fn namespaces(n: Node<'_, '_>) -> BTreeMap<String, String> {
    let namespace_node = if n.is_element() {
        n
    } else {
        n.parent_element().unwrap_or(n)
    };
    let mut namespaces: BTreeMap<_, _> = namespace_node
        .namespaces()
        .map(|ns| (ns.name().unwrap_or("").into(), ns.uri().into()))
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
    if let Some(uri) = stylesheet_base_uri(n) {
        namespaces.insert(xpath::STATIC_BASE_URI.into(), uri);
    }
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
        if n.tag_name().namespace() == Some(XSL)
            && n.tag_name().name() == "sequence"
            && a.namespace().is_none()
            && a.name() == "as"
        {
            return Err(static_error(
                n,
                "XTSE0090",
                "xsl:sequence does not have an as attribute",
            ));
        }
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
        if n.tag_name().namespace() == Some(XSL) && a.namespace().is_some_and(|uri| uri != XSL) {
            continue;
        }
        if a.namespace().is_some() || !allowed.contains(&a.name()) {
            if n.tag_name().namespace() == Some(XSL)
                && matches!(n.tag_name().name(), "variable" | "param" | "with-param")
            {
                // Visibility is a valid (but not yet implemented) property of
                // global variables; parameters and local variables cannot use it.
                let global_variable_visibility = a.namespace().is_none()
                    && a.name() == "visibility"
                    && n.tag_name().name() == "variable"
                    && n.parent_element().is_some_and(|parent| {
                        parent.tag_name().namespace() == Some(XSL)
                            && matches!(
                                parent.tag_name().name(),
                                "stylesheet" | "transform" | "package"
                            )
                    });
                let unsupported_standard = a.namespace().is_none()
                    && matches!(
                        a.name(),
                        "default-collation"
                            | "default-validation"
                            | "extension-element-prefixes"
                            | "version"
                    );
                if !global_variable_visibility && !unsupported_standard {
                    return Err(static_error(
                        n,
                        "XTSE0090",
                        format!("invalid attribute: {}", a.name()),
                    ));
                }
            }
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
fn required_empty(n: Node<'_, '_>) -> Result<(), Diagnostic> {
    let preserve_whitespace = n
        .ancestors()
        .filter_map(|ancestor| {
            ancestor.attribute(("http://www.w3.org/XML/1998/namespace", "space"))
        })
        .next()
        == Some("preserve");
    for child in n.children() {
        if child.is_element() && !use_when_excludes(child)?
            || child.is_text()
                && (preserve_whitespace || !xml_space(child.text().unwrap_or_default()))
        {
            return Err(static_error(n, "XTSE0260", "instruction must be empty"));
        }
    }
    Ok(())
}
fn sequence_select_content(n: Node<'_, '_>) -> Result<(), Diagnostic> {
    let preserve_whitespace = n
        .ancestors()
        .filter_map(|ancestor| {
            ancestor.attribute(("http://www.w3.org/XML/1998/namespace", "space"))
        })
        .next()
        == Some("preserve");
    if n.children().any(|child| {
        child.is_text() && (preserve_whitespace || !xml_space(child.text().unwrap_or_default()))
            || child.is_element()
                && !(child.tag_name().namespace() == Some(XSL)
                    && child.tag_name().name() == "fallback")
    }) {
        return Err(static_error(
            n,
            "XTSE3185",
            "select cannot be combined with sequence constructor content",
        ));
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
        | Instruction::MapEntry { .. }
        | Instruction::Map(_)
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
        | Instruction::MapEntry { .. }
        | Instruction::Map(_)
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
        | Instruction::ValueBody(_)
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
        const { std::cell::RefCell::new(xpath::Variables::new()) };
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
    let separator = avt(n, unqualified_attribute(n, "separator").unwrap_or(" "))?;
    if let Some(s) = unqualified_attribute(n, "select") {
        empty(n)?;
        Ok(Content::Select(expr(n, s)?, separator))
    } else {
        let separator = avt(
            n,
            unqualified_attribute(n, "separator").unwrap_or(
                if matches!(n.tag_name().name(), "attribute" | "value-of") {
                    ""
                } else {
                    " "
                },
            ),
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
            if c.attribute("stable").is_some() && !keys.is_empty() {
                return Err(static_error(
                    c,
                    "XTSE1017",
                    "stable is permitted only on the first sort key",
                ));
            }
            let dynamic_attr = |name: &str| -> Result<Option<Vec<Expr>>, Diagnostic> {
                match c.attribute(name) {
                    Some(v) if v.contains(['{', '}']) => avt(c, v).map(Some),
                    _ => Ok(None),
                }
            };
            let dynamic = DynamicSort {
                stable: dynamic_attr("stable")?,
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
            if dynamic.stable.is_none() {
                boolean_attribute(c, "stable", true)?;
            }
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
            if n.tag_name().namespace() == Some(XSL)
                && n.tag_name().name() == "fallback"
                && parent.tag_name().namespace() == Some(XSL)
                && matches!(
                    parent.tag_name().name(),
                    "sequence" | "on-empty" | "on-non-empty" | "map-entry" | "map" | "value-of"
                )
            {
                continue;
            }
            if n.tag_name().namespace() == Some(XSL) {
                match n.tag_name().name() {
                    "param" if skip_params => continue,
                    "function" => {
                        return Err(static_error(
                            n,
                            "XTSE0010",
                            "xsl:function must be a top-level declaration",
                        ));
                    }
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
                    "value-of" => {
                        attrs(n, &["select", "separator"])?;
                        if let Some(select) = unqualified_attribute(n, "select") {
                            sequence_select_content(n).map_err(|mut error| {
                                error.code = "XTSE0870".into();
                                error
                            })?;
                            out.push(Instruction::Value(
                                expr(n, select)?,
                                unqualified_attribute(n, "separator")
                                    .map(|s| avt(n, s))
                                    .transpose()?,
                            ));
                        } else {
                            out.push(Instruction::ValueBody(content(n, depth + 1)?));
                        }
                    }
                    "apply-templates" => {
                        attrs(n, &["select", "mode"])?;
                        let sorts = sorts(n)?;
                        let value = if let Some(s) = unqualified_attribute(n, "select") {
                            expr(n, s)?
                        } else {
                            expr(n, "node()")?
                        };
                        out.push(Instruction::Apply(
                            value,
                            mode(
                                n,
                                unqualified_attribute(n, "mode").unwrap_or("#default"),
                                true,
                            )?,
                            sorts,
                            with_params(n, depth, true)?,
                        ));
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
                        for name in ["on-duplicates", "use-attribute-sets"] {
                            if n.attributes()
                                .any(|a| a.namespace().is_none() && a.name() == name)
                            {
                                return Err(static_error(
                                    n,
                                    "XTSE0090",
                                    format!("invalid attribute on xsl:map: {name}"),
                                ));
                            }
                        }
                        attrs(n, &[])?;
                        out.push(Instruction::Map(body(n, depth + 1, false)?));
                    }
                    "map-entry" => match map_entry_expr(n, depth) {
                        Ok(expression) => out.push(Instruction::Sequence(expression)),
                        Err(error)
                            if error.code == "RUST_XSLT_UNSUPPORTED"
                                && unqualified_attribute(n, "select").is_none() =>
                        {
                            out.push(Instruction::MapEntry {
                                key: expr(n, required(n, "key")?)?,
                                body: body(n, depth + 1, false)?,
                            });
                        }
                        Err(error) => return Err(error),
                    },
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
                        if let Some(value) = n.attribute("schema-aware")
                            && !value.contains(['{', '}'])
                        {
                            boolean_value(n, Some(value), false)?;
                        }
                        let mut params = vec![];
                        let mut constructed_params = vec![];
                        for c in n.children() {
                            if c.is_element() && use_when_excludes(c)? {
                                continue;
                            }
                            if c.is_element()
                                && c.tag_name().namespace() == Some(XSL)
                                && c.tag_name().name() == "fallback"
                            {
                                continue;
                            }
                            if c.is_element()
                                && c.tag_name().namespace() == Some(XSL)
                                && c.tag_name().name() == "with-param"
                            {
                                let mut b = binding(c, true, depth)?;
                                if b.tunnel {
                                    return Err(static_error(
                                        c,
                                        "XTSE0020",
                                        "xsl:evaluate parameters cannot be tunnel parameters",
                                    ));
                                }
                                if params.iter().any(|(name, _, _)| *name == b.name) {
                                    return Err(static_error(
                                        c,
                                        "XTSE0670",
                                        "duplicate xsl:evaluate parameter",
                                    ));
                                }
                                if matches!(b.value, BindingValue::Select(_)) {
                                    let BindingValue::Select(e) = b.value else {
                                        unreachable!()
                                    };
                                    params.push((b.name, e, b.ty));
                                } else {
                                    let name = std::mem::replace(
                                        &mut b.name,
                                        format!(
                                            "\u{0}evaluate-param:{}:{}",
                                            n.range().start,
                                            constructed_params.len()
                                        ),
                                    );
                                    params.push((name, Expr::Variable(b.name.clone()), None));
                                    constructed_params.push(Instruction::Bind(b));
                                }
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
                        let evaluate = Instruction::Sequence(Expr::Evaluate {
                            xpath: Box::new(expr(n, required(n, "xpath")?)?),
                            context: n
                                .attribute("context-item")
                                .map(|c| expr(n, c).map(Box::new))
                                .transpose()?,
                            namespace_context: n
                                .attribute("namespace-context")
                                .map(|c| expr(n, c).map(Box::new))
                                .transpose()?,
                            schema_aware: n
                                .attribute("schema-aware")
                                .map(|s| avt(n, s))
                                .transpose()?,
                            base_uri: n.attribute("base-uri").map(|s| avt(n, s)).transpose()?,
                            stylesheet_base_uri: stylesheet_base_uri(n),
                            params,
                            result_type: n
                                .attribute("as")
                                .map(|s| crate::seqtype::SequenceType::parse(s, &namespaces(n)))
                                .transpose()?,
                            with_params: n
                                .attribute("with-params")
                                .map(|s| expr(n, s).map(Box::new))
                                .transpose()?,
                            namespaces: namespaces(n),
                        });
                        if constructed_params.is_empty() {
                            out.push(evaluate);
                        } else {
                            constructed_params.push(evaluate);
                            out.push(Instruction::If(
                                Expr::Call("true".into(), vec![]),
                                constructed_params,
                            ));
                        }
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
                        if let Some(value) = n.attribute("start-at")
                            && !value.contains('{')
                            && !value.contains('}')
                            && crate::numbering::parse_start_at(value).is_err()
                        {
                            return Err(static_error(
                                n,
                                "XTSE0020",
                                "invalid xsl:number start-at list",
                            ));
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
                        if let Some(value) = n.attribute("lang")
                            && !value.contains('{')
                            && !value.contains('}')
                            && crate::string_type::StringType::Language
                                .cast(value)
                                .is_err()
                        {
                            return Err(static_error(n, "XTSE0020", "invalid xsl:number language"));
                        }
                        if let Some(value) = n.attribute("letter-value")
                            && !value.contains('{')
                            && !value.contains('}')
                            && !matches!(value, "alphabetic" | "traditional")
                        {
                            return Err(static_error(
                                n,
                                "XTSE0020",
                                "invalid xsl:number letter-value",
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
                            lang: n.attribute("lang").map(|value| avt(n, value)).transpose()?,
                            letter_value: n
                                .attribute("letter-value")
                                .map(|value| avt(n, value))
                                .transpose()?,
                            grouping_separator: optional_avt("grouping-separator")?,
                            grouping_size: optional_avt("grouping-size")?,
                            start_at: n
                                .attribute("start-at")
                                .map(|value| avt(n, value))
                                .transpose()?,
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
                                "undeclare-prefixes",
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
                            .map(|value| avt(n, value))
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
                            undeclare_prefixes: flag("undeclare-prefixes")?,
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
                                sequence_select_content(n)?;
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
                        if let Some(select) = n.attribute("select") {
                            sequence_select_content(n)?;
                            out.push(Instruction::Sequence(expr(n, select)?));
                        } else {
                            out.push(Instruction::If(
                                Expr::Call("true".into(), vec![]),
                                body(n, depth + 1, false)?,
                            ));
                        }
                    }
                    "copy" => {
                        attrs(
                            n,
                            &[
                                "select",
                                "copy-namespaces",
                                "inherit-namespaces",
                                "use-attribute-sets",
                            ],
                        )?;
                        out.push(Instruction::Copy {
                            select: n.attribute("select").map(|s| expr(n, s)).transpose()?,
                            namespaces: boolean_attribute(n, "copy-namespaces", true)?,
                            inherit: boolean_attribute(n, "inherit-namespaces", true)?,
                            sets: n
                                .attribute("use-attribute-sets")
                                .map(|names| set_names(n, names).map(Instruction::UseSets))
                                .transpose()?
                                .into_iter()
                                .collect(),
                            body: body(n, depth + 1, false)?,
                        });
                    }
                    "copy-of" => {
                        for attribute in n
                            .attributes()
                            .filter(|attribute| attribute.namespace().is_none())
                        {
                            if !matches!(
                                attribute.name(),
                                "select"
                                    | "copy-namespaces"
                                    | "copy-accumulators"
                                    | "type"
                                    | "validation"
                                    | "default-collation"
                                    | "default-mode"
                                    | "default-validation"
                                    | "exclude-result-prefixes"
                                    | "expand-text"
                                    | "extension-element-prefixes"
                                    | "use-when"
                                    | "version"
                                    | "xpath-default-namespace"
                            ) {
                                return Err(static_error(
                                    n,
                                    "XTSE0090",
                                    format!("invalid xsl:copy-of attribute: {}", attribute.name()),
                                ));
                            }
                        }
                        attrs(n, &["select", "copy-namespaces"])?;
                        required_empty(n)?;
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
                        // These attributes belonged to an earlier draft and are
                        // not part of the XSLT 3.0 instruction.
                        if let Some(name) = ["bind-group", "bind-grouping-key"]
                            .into_iter()
                            .find(|name| n.attribute(*name).is_some())
                        {
                            return Err(static_error(
                                n,
                                "XTSE0090",
                                format!("invalid grouping attribute: {name}"),
                            ));
                        }
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
                        let composite = boolean_attribute(n, "composite", false)?;
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
                        if !matches!(*attribute, "group-by" | "group-adjacent")
                            && (n.attribute("collation").is_some()
                                || n.attribute("composite").is_some())
                        {
                            return Err(static_error(
                                n,
                                "XTSE1090",
                                "collation and composite require group-by or group-adjacent",
                            ));
                        }
                        if let Some(collation) = n.attribute("collation")
                            && collation
                                != "http://www.w3.org/2005/xpath-functions/collation/codepoint"
                        {
                            return Err(error(n, "unsupported grouping collation"));
                        }
                        let value = n.attribute(*attribute).unwrap();
                        let grouping = match *attribute {
                            "group-by" => Grouping::By(expr(n, value)?, composite),
                            "group-adjacent" => Grouping::Adjacent(expr(n, value)?, composite),
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
                        let mut branches = vec![];
                        for (test, c) in choose_branches(n)? {
                            branches.push((test, body(c, depth + 1, false)?));
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
                            "version",
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
                let in_scope_namespaces = element.namespaces.clone();
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
                    in_scope_namespaces,
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
        let on_empty = out
            .iter()
            .position(|i| matches!(i, Instruction::OnEmpty(_)));
        if on_empty.is_some_and(|index| index + 1 != out.len()) {
            return Err(static_error(
                parent,
                "XTSE0010",
                "xsl:on-empty must be the final instruction in its sequence constructor",
            ));
        }
        return Ok(vec![Instruction::Populated {
            main: out,
            has_on_empty: on_empty.is_some(),
        }]);
    }
    Ok(out)
}
fn map_entry_expr(n: Node<'_, '_>, depth: usize) -> Result<Expr, Diagnostic> {
    attrs(n, &["key", "select"])?;
    let key = expr(n, required(n, "key")?)?;
    let value = if let Some(select) = unqualified_attribute(n, "select") {
        sequence_select_content(n).map_err(|mut error| {
            error.code = "XTSE3280".into();
            error
        })?;
        expr(n, select)?
    } else {
        function_expression(n, body(n, depth + 1, false)?, false)?
    };
    Ok(Expr::Call("map:entry".into(), vec![key, value]))
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
    sheet.unsupported_encoding = get("encoding")
        .filter(|value| crate::result::Encoding::parse(value).is_none())
        .map(str::to_owned);
    sheet.indent = get("indent") == Some("yes");
    sheet.escape_uri_attributes = get("escape-uri-attributes") != Some("no");
    sheet.undeclare_prefixes = get("undeclare-prefixes") == Some("yes");
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

struct ModeProperty<T> {
    value: Option<(u32, T, bool)>,
}
impl<T> Default for ModeProperty<T> {
    fn default() -> Self {
        Self { value: None }
    }
}
impl<T: Clone + PartialEq> ModeProperty<T> {
    fn add(&mut self, precedence: u32, value: Option<T>) {
        let Some(value) = value else { return };
        match &mut self.value {
            Some((rank, old, conflict)) if *rank == precedence => *conflict |= *old != value,
            Some((rank, _, _)) if *rank > precedence => {}
            slot => *slot = Some((precedence, value, false)),
        }
    }
    fn resolved(&self) -> Result<Option<T>, Diagnostic> {
        match &self.value {
            Some((_, _, true)) => Err(Diagnostic::new(
                "XTSE0545",
                "conflicting mode declarations",
                "compile",
            )),
            Some((_, value, false)) => Ok(Some(value.clone())),
            None => Ok(None),
        }
    }
}

#[derive(Default)]
struct ModeDeclarations {
    policy: ModeProperty<crate::mode::OnNoMatch>,
    multiple: ModeProperty<bool>,
    visibility: ModeProperty<crate::mode::Visibility>,
    warning: ModeProperty<bool>,
    warning_no_match: ModeProperty<bool>,
    streamable: ModeProperty<bool>,
    typed: ModeProperty<crate::mode::Typed>,
    accumulators: ModeProperty<crate::mode::AccumulatorUse>,
}

#[derive(Debug)]
pub(crate) struct AccumulatorRule {
    pub branches: Vec<crate::pattern::Branch>,
    pub end: bool,
    pub select: Expr,
}
#[derive(Debug)]
pub(crate) struct Accumulator {
    pub initial: Expr,
    pub ty: Option<crate::seqtype::SequenceType>,
    pub rules: Vec<AccumulatorRule>,
    pub precedence: u32,
}

fn accumulator_use(
    n: Node<'_, '_>,
    value: &str,
) -> Result<crate::mode::AccumulatorUse, Diagnostic> {
    let words: Vec<_> = value.split_whitespace().collect();
    if words == ["#all"] {
        return Ok(crate::mode::AccumulatorUse::All);
    }
    let mut names = BTreeSet::new();
    for word in words {
        if word.starts_with('#') || !names.insert(component_name(n, word)?) {
            return Err(static_error(
                n,
                "XTSE3300",
                "invalid or duplicate accumulator name",
            ));
        }
    }
    Ok(crate::mode::AccumulatorUse::Names(names))
}

fn add_accumulator(
    n: Node<'_, '_>,
    sheet: &mut Stylesheet,
    precedence: u32,
) -> Result<(), Diagnostic> {
    attrs(n, &["name", "initial-value", "as", "streamable"])?;
    let name = component_name(n, required(n, "name")?)?;
    check_component_namespace(n, &name, false)?;
    boolean_attribute(n, "streamable", false)?;
    let initial = expr(n, required(n, "initial-value")?)?;
    let ty = n
        .attribute("as")
        .map(|s| crate::seqtype::SequenceType::parse(s, &namespaces(n)))
        .transpose()?;
    let mut rules = Vec::new();
    for child in n.children() {
        if child.is_text() && !xml_space(child.text().unwrap_or_default()) {
            return Err(static_error(
                child,
                "XTSE0010",
                "unexpected accumulator text",
            ));
        }
        if !child.is_element() || use_when_excludes(child)? {
            continue;
        }
        if child.tag_name().namespace() != Some(XSL)
            || child.tag_name().name() != "accumulator-rule"
        {
            return Err(static_error(
                child,
                "XTSE0010",
                "expected xsl:accumulator-rule",
            ));
        }
        attrs(child, &["match", "phase", "select"])?;
        let end = match child.attribute("phase").unwrap_or("start").trim() {
            "start" => false,
            "end" => true,
            _ => return Err(static_error(child, "XTSE0020", "invalid accumulator phase")),
        };
        let branches = crate::pattern::parse(
            required(child, "match")?,
            namespaces(child),
            xslt_version(child),
        )?;
        let select = child.attribute("select").ok_or_else(|| {
            error(
                child,
                "accumulator rule sequence constructors are not implemented",
            )
        })?;
        empty(child)?;
        rules.push(AccumulatorRule {
            branches,
            end,
            select: expr(child, select)?,
        });
    }
    if rules.is_empty() {
        return Err(static_error(n, "XTSE0010", "accumulator requires a rule"));
    }
    if let Some(previous) = sheet.accumulators.get(&name) {
        if previous.precedence == precedence {
            return Err(static_error(n, "XTSE0010", "duplicate accumulator"));
        }
        if previous.precedence > precedence {
            return Ok(());
        }
    }
    sheet.accumulators.insert(
        name,
        Accumulator {
            initial,
            ty,
            rules,
            precedence,
        },
    );
    Ok(())
}

#[derive(Debug)]
pub(crate) struct NamespaceAlias {
    pub target: String,
    pub prefix: String,
    precedence: u32,
    conflict: bool,
}

fn add_namespace_alias(
    n: Node<'_, '_>,
    sheet: &mut Stylesheet,
    precedence: u32,
) -> Result<(), Diagnostic> {
    attrs(n, &["stylesheet-prefix", "result-prefix"])?;
    empty(n)?;
    let resolve = |name: &str| -> Result<(String, String), Diagnostic> {
        let prefix = required(n, name)?.trim();
        if prefix == "#default" {
            return Ok((
                String::new(),
                n.lookup_namespace_uri(None).unwrap_or("").to_owned(),
            ));
        }
        let uri = n.lookup_namespace_uri(Some(prefix)).ok_or_else(|| {
            static_error(n, "XTSE0812", format!("unbound namespace prefix: {prefix}"))
        })?;
        Ok((prefix.to_owned(), uri.to_owned()))
    };
    let (_, literal) = resolve("stylesheet-prefix")?;
    let (prefix, target) = resolve("result-prefix")?;
    let candidate = NamespaceAlias {
        target,
        prefix,
        precedence,
        conflict: false,
    };
    match sheet.namespace_aliases.get_mut(&literal) {
        Some(old) if old.precedence > precedence => {}
        Some(old) if old.precedence == precedence => {
            old.conflict |= old.target != candidate.target;
            old.target = candidate.target;
            old.prefix = candidate.prefix;
        }
        _ => {
            sheet.namespace_aliases.insert(literal, candidate);
        }
    }
    Ok(())
}

struct Declarations<'a> {
    sheet: &'a mut Stylesheet,
    output_properties: &'a mut BTreeMap<String, OutputGroup>,
    mode_declarations: &'a mut BTreeMap<String, ModeDeclarations>,
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
    module_version(root)?;
    if root.tag_name().namespace() != Some(XSL) {
        let precedence = inherited.unwrap_or_else(|| {
            let own = *state.counter;
            *state.counter += 1;
            own
        });
        state
            .sheet
            .templates
            .push(simplified_template(root, precedence)?);
        return Ok(());
    }
    let Declarations {
        sheet,
        output_properties,
        mode_declarations,
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
        let module_uri =
            crate::uri_resolution::resolve(href, stylesheet_base_uri(n).as_deref()).ok();
        let imported = resolver(module_uri.as_deref().unwrap_or(href))?;
        let doc = crate::parse(&imported, "compile")?;
        let imported_root = doc.root_element();
        if let Some(uri) = &module_uri {
            sheet
                .stylesheet_sources
                .insert(uri.clone(), imported.clone());
        }

        with_module_base(module_uri, || {
            declarations(
                imported_root,
                &mut Declarations {
                    sheet,
                    output_properties,
                    mode_declarations,
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
            )
        })?;
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
            "namespace-alias" => add_namespace_alias(n, sheet, precedence)?,
            "accumulator" => add_accumulator(n, sheet, precedence)?,
            "mode" => {
                use crate::mode::OnNoMatch;
                attrs(
                    n,
                    &[
                        "name",
                        "on-no-match",
                        "on-multiple-match",
                        "warning-on-multiple-match",
                        "warning-on-no-match",
                        "streamable",
                        "use-accumulators",
                        "visibility",
                        "typed",
                    ],
                )?;
                empty(n)?;
                if boolean_attribute(n, "streamable", false)? {
                    return Err(error(n, "XSLT streamable modes are not implemented"));
                }
                let accumulator_use = n
                    .attribute("use-accumulators")
                    .map(|value| accumulator_use(n, value))
                    .transpose()?;
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
                let warning = n
                    .attribute("warning-on-multiple-match")
                    .map(|_| boolean_attribute(n, "warning-on-multiple-match", true))
                    .transpose()?;
                let warning_no_match = n
                    .attribute("warning-on-no-match")
                    .map(|_| boolean_attribute(n, "warning-on-no-match", false))
                    .transpose()?;
                let typed = n
                    .attribute("typed")
                    .map(|value| match value.trim() {
                        "yes" | "true" | "1" => Ok(crate::mode::Typed::Required),
                        "no" | "false" | "0" => Ok(crate::mode::Typed::Untyped),
                        "unspecified" => Ok(crate::mode::Typed::Unspecified),
                        "lax" => Ok(crate::mode::Typed::Lax),
                        "strict" => Ok(crate::mode::Typed::Strict),
                        _ => Err(static_error(n, "XTSE0020", "invalid typed mode setting")),
                    })
                    .transpose()?;
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
                let entry = mode_declarations.entry(name).or_default();
                entry.policy.add(precedence, policy);
                entry.multiple.add(precedence, multiple);
                entry.visibility.add(precedence, visibility);
                entry.warning.add(precedence, warning);
                entry.warning_no_match.add(precedence, warning_no_match);
                entry.typed.add(precedence, typed);
                entry.accumulators.add(precedence, accumulator_use);
                entry
                    .streamable
                    .add(precedence, n.attribute("streamable").map(|_| false));
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
                attrs(
                    n,
                    &["match", "name", "mode", "priority", "as", "visibility"],
                )?;
                if let Some(visibility) = n.attribute("visibility") {
                    if visibility.trim() != "private" {
                        return Err(error(n, "only private template visibility is supported"));
                    }
                    if n.attribute("name").is_none() {
                        return Err(static_error(
                            n,
                            "XTSE0500",
                            "template visibility requires a name",
                        ));
                    }
                }
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
                            .map_or_else(
                                || attribute.value().trim().to_ascii_lowercase(),
                                |encoding| encoding.name().to_owned(),
                            ),
                        "html-version" => {
                            let value = attribute.value().trim();
                            let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
                            let mut parts = unsigned.split('.');
                            let integer = parts.next().unwrap_or_default();
                            let fraction = parts.next();
                            if parts.next().is_some()
                                || (integer.is_empty() && fraction.is_none_or(str::is_empty))
                                || !integer.bytes().all(|b| b.is_ascii_digit())
                                || fraction.is_some_and(|s| !s.bytes().all(|b| b.is_ascii_digit()))
                            {
                                return Err(static_error(n, "XTSE0020", "invalid html-version"));
                            }
                            value.to_owned()
                        }
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
                let module_uri =
                    crate::uri_resolution::resolve(href, stylesheet_base_uri(n).as_deref()).ok();
                let included = resolver(module_uri.as_deref().unwrap_or(href))?;
                let doc = crate::parse(&included, "compile")?;
                let included_root = doc.root_element();
                if let Some(uri) = &module_uri {
                    sheet
                        .stylesheet_sources
                        .insert(uri.clone(), included.clone());
                }

                with_module_base(module_uri, || {
                    declarations(
                        included_root,
                        &mut Declarations {
                            sheet,
                            output_properties,
                            mode_declarations,
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
                    )
                })?;
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
                let preserve_space = n
                    .ancestors()
                    .filter_map(|ancestor| {
                        ancestor.attribute(("http://www.w3.org/XML/1998/namespace", "space"))
                    })
                    .next()
                    == Some("preserve");
                let mut has_content = false;
                for child in n.children() {
                    if child.is_element() {
                        if use_when_excludes(child)? {
                            continue;
                        }
                        if child.tag_name().namespace() == Some(XSL)
                            && child.tag_name().name() == "template"
                        {
                            return Err(static_error(
                                child,
                                "XTSE0010",
                                "xsl:template must be a top-level declaration",
                            ));
                        }
                        has_content = true;
                    } else if child.is_text()
                        && (preserve_space || !xml_space(child.text().unwrap_or_default()))
                    {
                        has_content = true;
                    }
                }
                let value = match (n.attribute("use"), has_content) {
                    (Some(select), false) => expr(n, select)?,
                    (None, true) => function_expression(n, body(n, 1, false)?, false)?,
                    _ => {
                        return Err(static_error(
                            n,
                            "XTSE1205",
                            "xsl:key requires either a use attribute or a sequence constructor",
                        ));
                    }
                };
                keys.entry(id).or_default().push(xpath::KeyDefinition {
                    branches,
                    value,
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
                let override_old = n
                    .attribute("override")
                    .map(|value| boolean_value(n, Some(value), true))
                    .transpose()?;
                let override_new = n
                    .attribute("override-extension-function")
                    .map(|value| boolean_value(n, Some(value), true))
                    .transpose()?;
                if override_old
                    .zip(override_new)
                    .is_some_and(|(old, new)| old != new)
                {
                    return Err(static_error(
                        n,
                        "XTSE0020",
                        "function override attributes disagree",
                    ));
                }
                boolean_attribute(n, "cache", false)?;
                if let Some(value) = n.attribute("new-each-time")
                    && value.trim() != "maybe"
                {
                    boolean_value(n, Some(value), false)?;
                }
                let id = component_name(
                    n,
                    n.attribute("name")
                        .ok_or_else(|| static_error(n, "XTSE0010", "function requires a name"))?,
                )?;
                if crate::qname::parts(&id).0.is_none_or(str::is_empty) {
                    return Err(static_error(
                        n,
                        "XTSE0740",
                        "function names need a namespace",
                    ));
                }
                check_component_namespace(n, &id, false)?;
                let result = n
                    .attribute("as")
                    .map(|s| crate::seqtype::SequenceType::parse(s, &namespaces(n)))
                    .transpose()?;
                let dynamic_visible = match n.attribute("visibility").unwrap_or("private").trim() {
                    "public" | "final" | "abstract" => true,
                    "private" | "hidden" => false,
                    _ => return Err(static_error(n, "XTSE0020", "invalid function visibility")),
                };
                let mut params: Vec<(String, Option<crate::seqtype::SequenceType>)> = vec![];
                let mut content = false;
                for c in n.children() {
                    if c.is_element()
                        && c.tag_name().namespace() == Some(XSL)
                        && c.tag_name().name() == "param"
                    {
                        if content {
                            return Err(static_error(
                                c,
                                "XTSE0010",
                                "function parameters must precede the body",
                            ));
                        }
                        attrs(c, &["name", "as", "required", "tunnel", "static", "select"])?;
                        if c.attribute("select").is_some()
                            || c.children().any(|child| {
                                child.is_element()
                                    || child.is_text()
                                        && (!xml_space(child.text().unwrap_or_default())
                                            || c.ancestors()
                                                .filter_map(|ancestor| {
                                                    ancestor.attribute((
                                                        "http://www.w3.org/XML/1998/namespace",
                                                        "space",
                                                    ))
                                                })
                                                .next()
                                                == Some("preserve"))
                            })
                        {
                            return Err(static_error(
                                c,
                                "XTSE0760",
                                "function parameters cannot have default values",
                            ));
                        }
                        if !boolean_attribute(c, "required", true)?
                            || boolean_attribute(c, "tunnel", false)?
                            || boolean_attribute(c, "static", false)?
                        {
                            return Err(static_error(
                                c,
                                "XTSE0020",
                                "invalid function parameter setting",
                            ));
                        }
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
                let function_instructions = body(n, 1, true)?;
                let text_result = result.as_ref().is_some_and(|ty| {
                    matches!(ty.kind, crate::seqtype::Kind::String) && ty.max == 1
                }) && can_lower_text_function(&function_instructions);
                let function_body = if can_lower_function(&function_instructions) || text_result {
                    xpath::UserFunctionBody::Expression(function_expression(
                        n,
                        function_instructions,
                        text_result,
                    )?)
                } else {
                    xpath::UserFunctionBody::Constructor(function_instructions)
                };
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
                        dynamic_visible,
                    },
                );
            }
            other => return Err(error(n, format!("unsupported declaration: {other}"))),
        }
    }
    Ok(())
}
// Conservatively detect references, including inside closures and predicates.
// Dynamic evaluation is treated as a possible reference without inspecting
// the expression it will compile at run time.
fn may_reference_variable(expression: &Expr, name: &str) -> bool {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match expression {
            Expr::Variable(found) if found == name => return true,
            Expr::Variable(_)
            | Expr::String(_)
            | Expr::Number(_)
            | Expr::Context
            | Expr::Root
            | Expr::FunctionRef(..) => {}
            Expr::For(_, a, b)
            | Expr::Let(_, a, b, _)
            | Expr::Quantified(_, _, a, b)
            | Expr::Binary(_, a, b)
            | Expr::PathMap(a, b) => pending.extend([a.as_ref(), b.as_ref()]),
            Expr::Path(base, steps) => {
                pending.push(base);
                pending.extend(steps.iter().flat_map(|step| &step.predicates));
            }
            Expr::Filter(base, expressions) | Expr::DynamicCall(base, expressions) => {
                pending.push(base);
                pending.extend(expressions);
            }
            Expr::Unary(_, inner)
            | Expr::TypeOp(_, inner, _)
            | Expr::QNameCast(inner, _, _, _)
            | Expr::StaticName(inner, _, _)
            | Expr::LookupContext(inner, _)
            | Expr::ArrayCurly(inner) => pending.push(inner),
            Expr::Call(_, args) | Expr::Sequence(args) | Expr::ArraySquare(args) => {
                pending.extend(args)
            }
            Expr::If(condition, yes, no) => {
                pending.extend([condition.as_ref(), yes.as_ref(), no.as_ref()])
            }
            Expr::InlineFunction(function) => pending.push(&function.body),
            Expr::Partial(callee, args) => {
                pending.push(callee);
                pending.extend(args.iter().flatten());
            }
            Expr::MapConstructor(entries) => {
                for (key, value) in entries {
                    pending.extend([key, value]);
                }
            }
            Expr::Lookup(base, key) => {
                pending.push(base);
                pending.extend(key.iter().map(|key| key.as_ref()));
            }
            Expr::Sorted(base, keys) => {
                pending.push(base);
                for key in keys {
                    pending.extend(key.expressions());
                }
            }
            Expr::Evaluate { .. } => return true,
        }
    }
    false
}

fn can_lower_function(body: &[Instruction]) -> bool {
    body.iter().all(|instruction| match instruction {
        Instruction::Bind(Binding {
            value: BindingValue::Select(_),
            ..
        })
        | Instruction::Sequence(_) => true,
        Instruction::Bind(Binding {
            value: BindingValue::Raw(body),
            ..
        })
        | Instruction::Map(body)
        | Instruction::ForEach(_, _, body)
        | Instruction::If(_, body) => can_lower_function(body),
        Instruction::Choose(branches) => branches.iter().all(|(_, body)| can_lower_function(body)),
        _ => false,
    })
}

// A single value-of per execution path may be atomized directly only when
// the function's declared result is an optional/single xs:string. No caller can
// observe the intermediate text-node identity in that case.
fn can_lower_text_function(body: &[Instruction]) -> bool {
    let mut results = 0;
    for instruction in body {
        match instruction {
            Instruction::Bind(Binding {
                value: BindingValue::Select(_),
                ..
            }) => {}
            Instruction::Bind(Binding {
                value: BindingValue::Raw(body),
                ..
            }) if can_lower_function(body) => {}
            Instruction::Value(_, None) => results += 1,
            Instruction::If(_, body) if can_lower_text_function(body) => results += 1,
            Instruction::Choose(branches)
                if branches
                    .iter()
                    .all(|(_, body)| can_lower_text_function(body)) =>
            {
                results += 1
            }
            _ => return false,
        }
        if results > 1 {
            return false;
        }
    }
    true
}

// Lower pure sequence constructors into XPath expressions. Branch-local
// bindings remain scoped to their branch, and bindings affect only what follows.
fn function_expression(
    n: Node<'_, '_>,
    body: Vec<Instruction>,
    text_result: bool,
) -> Result<Expr, Diagnostic> {
    fn append(head: Expr, tail: Expr) -> Expr {
        if matches!(&tail, Expr::Sequence(items) if items.is_empty()) {
            head
        } else {
            Expr::Sequence(vec![head, tail])
        }
    }
    let mut tail = Expr::Sequence(vec![]);
    for instruction in body.into_iter().rev() {
        tail = match instruction {
            Instruction::Bind(Binding {
                name,
                value: BindingValue::Select(select),
                ty,
                ..
            }) => {
                let select = match ty {
                    Some(ty) => Expr::TypeOp("variable".into(), Box::new(select), ty),
                    None => select,
                };
                let unused = !may_reference_variable(&tail, &name);
                Expr::Let(name, Box::new(select), Box::new(tail), unused)
            }
            Instruction::Bind(Binding {
                name,
                value: BindingValue::Raw(body),
                ty,
                ..
            }) => {
                // Lower only constructors that preserve their original XDM items.
                // Node-creating instructions use the constructor runtime instead.
                let select = function_expression(n, body, false)?;
                let select = match ty {
                    Some(ty) => Expr::TypeOp("variable".into(), Box::new(select), ty),
                    None => select,
                };
                let unused = !may_reference_variable(&tail, &name);
                Expr::Let(name, Box::new(select), Box::new(tail), unused)
            }
            Instruction::Sequence(expression) => append(expression, tail),
            Instruction::Value(expression, None) if text_result => {
                append(Expr::Call("\0xslt-value-of".into(), vec![expression]), tail)
            }
            Instruction::Map(body) => {
                let maps = function_expression(n, body, false)?;
                append(Expr::Call("\0xslt-map".into(), vec![maps]), tail)
            }
            Instruction::ForEach(select, sorts, body) => {
                let select = if sorts.is_empty() {
                    select
                } else {
                    Expr::Sorted(Box::new(select), sorts)
                };
                let body = function_expression(n, body, false)?;
                append(
                    Expr::Binary("instruction-map".into(), Box::new(select), Box::new(body)),
                    tail,
                )
            }
            Instruction::If(test, body) => append(
                Expr::If(
                    Box::new(test),
                    Box::new(function_expression(n, body, text_result)?),
                    Box::new(Expr::Sequence(vec![])),
                ),
                tail,
            ),
            Instruction::Choose(branches) => {
                let mut choice = Expr::Sequence(vec![]);
                for (test, branch) in branches.into_iter().rev() {
                    let result = function_expression(n, branch, text_result)?;
                    choice = match test {
                        Some(test) => Expr::If(Box::new(test), Box::new(result), Box::new(choice)),
                        None => result,
                    };
                }
                append(choice, tail)
            }
            _ => {
                return Err(error(
                    n,
                    "unsupported instruction in expression-based sequence constructor",
                ));
            }
        };
    }
    Ok(tail)
}

fn module_version<'a>(root: Node<'a, '_>) -> Result<&'a str, Diagnostic> {
    let version = if root.tag_name().namespace() != Some(XSL) {
        root.attribute((XSL, "version")).ok_or_else(|| {
            static_error(
                root,
                "XTSE0150",
                "simplified stylesheet requires xsl:version",
            )
        })?
    } else {
        if !matches!(root.tag_name().name(), "stylesheet" | "transform") {
            return Err(error(root, "expected xsl:stylesheet or xsl:transform"));
        }
        attrs(root, &["version"])?;
        root.attribute("version").unwrap_or_default()
    };
    if !matches!(version, "1.0" | "2.0" | "3.0") {
        return Err(error(root, "expected version 1.0, 2.0, or 3.0"));
    }
    Ok(version)
}

fn simplified_template(root: Node<'_, '_>, precedence: u32) -> Result<Template, Diagnostic> {
    Ok(Template {
        pattern: Some(crate::pattern::parse(
            "/",
            namespaces(root),
            xslt_version(root),
        )?),
        modes: vec![String::new()],
        priority: None,
        name: None,
        params: vec![],
        result_type: None,
        body: body(root.document().root(), 0, false)?,
        precedence,
    })
}

pub(crate) fn compile(
    source: &str,
    resolver: Option<Resolver<'_>>,
) -> Result<Stylesheet, Diagnostic> {
    with_module_base(None, || compile_inner(source, resolver))
}
pub(crate) fn compile_with_base(
    source: &str,
    base: &str,
    resolver: Option<Resolver<'_>>,
) -> Result<Stylesheet, Diagnostic> {
    if base.contains('#') || crate::uri_resolution::resolve(base, None).is_err() {
        return Err(Diagnostic::new(
            "RUST_XSLT_INVALID_URI",
            "stylesheet URI must be an absolute URI without a fragment",
            "compile",
        ));
    }
    with_module_base(Some(base.to_owned()), || compile_inner(source, resolver))
}
fn compile_inner(source: &str, resolver: Option<Resolver<'_>>) -> Result<Stylesheet, Diagnostic> {
    xpath::take_document_uris();
    STATIC_VARIABLES.with(|v| v.borrow_mut().clear());
    let doc = crate::parse(source, "compile")?;
    let root = doc.root_element();
    let simplified = root.tag_name().namespace() != Some(XSL);
    let version = module_version(root)?;
    let mut sheet = Stylesheet {
        templates: vec![],
        namespace_aliases: BTreeMap::new(),
        globals: vec![],
        functions: Default::default(),
        keys: Default::default(),
        xpath_only: xpath::xpath_only_profile(),
        attribute_sets: BTreeMap::new(),
        decimal_formats: Default::default(),
        document_uris: vec![],
        stylesheet_sources: BTreeMap::new(),
        outputs: BTreeMap::new(),
        space_rules: vec![],
        default_mode: if simplified {
            String::new()
        } else {
            default_mode(root)?
        },
        modes: BTreeMap::new(),
        accumulators: BTreeMap::new(),
        omit_declaration: false,
        standalone: None,
        byte_order_mark: false,
        indent: false,
        markup: crate::result::Markup::Xml,
        escape_uri_attributes: true,
        undeclare_prefixes: false,
        include_content_type: true,
        media_type: None,
        encoding: Default::default(),
        unsupported_encoding: None,
        cdata_elements: Default::default(),
        character_maps: Default::default(),
        char_map: Default::default(),
        doctype: None,
        text_output: false,
        backwards: version == "1.0",
        version: match version {
            "1.0" => 1,
            "2.0" => 2,
            _ => 3,
        },
    };
    MODULE_BASE_URI.with(|base| {
        if let Some(uri) = base.borrow().as_ref() {
            sheet
                .stylesheet_sources
                .insert(uri.clone(), source.to_owned());
        }
    });
    let mut output_properties = BTreeMap::new();
    let mut mode_declarations = BTreeMap::new();
    let mut functions: BTreeMap<(String, usize), xpath::UserFunction> = BTreeMap::new();
    let mut keys: BTreeMap<String, Vec<xpath::KeyDefinition>> = BTreeMap::new();
    let mut attribute_sets: BTreeMap<String, AttributeSet> = BTreeMap::new();
    let mut character_maps: BTreeMap<String, CharacterMap> = BTreeMap::new();
    let mut decimal_formats: BTreeMap<String, crate::format_number::Format> = BTreeMap::new();
    let mut precedence_counter = 0u32;
    let mut global_precedence: BTreeMap<String, u32> = BTreeMap::new();
    let mut function_precedence: BTreeMap<(String, usize), u32> = BTreeMap::new();
    if simplified {
        sheet.templates.push(simplified_template(root, 0)?);
    } else {
        declarations(
            root,
            &mut Declarations {
                sheet: &mut sheet,
                output_properties: &mut output_properties,
                mode_declarations: &mut mode_declarations,
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
    }
    if sheet.namespace_aliases.values().any(|alias| alias.conflict) {
        return Err(Diagnostic::new(
            "XTSE0810",
            "conflicting namespace aliases",
            "compile",
        ));
    }
    for (name, declarations) in mode_declarations {
        let _ = declarations.warning.resolved()?;
        let _ = declarations.warning_no_match.resolved()?;
        let _ = declarations.streamable.resolved()?;
        let typed = declarations.typed.resolved()?;
        if typed == Some(crate::mode::Typed::Strict) {
            return Err(Diagnostic::new(
                "RUST_XSLT_UNSUPPORTED",
                "strict typed modes require schema support",
                "compile",
            ));
        }
        sheet.modes.insert(
            name,
            crate::mode::Mode {
                on_no_match: declarations.policy.resolved()?,
                fail_on_multiple: declarations.multiple.resolved()?,
                visibility: declarations.visibility.resolved()?,
                typed,
                accumulators: declarations.accumulators.resolved()?,
            },
        );
    }
    for mode in sheet.modes.values() {
        if let Some(crate::mode::AccumulatorUse::Names(names)) = &mode.accumulators {
            for name in names {
                if !sheet.accumulators.contains_key(name) {
                    return Err(Diagnostic::new(
                        "XTSE3300",
                        format!("unknown accumulator: {name}"),
                        "compile",
                    ));
                }
            }
        }
    }
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
    sheet.undeclare_prefixes = primary.undeclare_prefixes;
    sheet.include_content_type = primary.include_content_type;
    sheet.media_type = primary.media_type.clone();
    sheet.encoding = primary.encoding;
    sheet.unsupported_encoding = primary.unsupported_encoding.clone();
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

fn validate_expression(
    e: &Expr,
    scope: &BTreeSet<String>,
    functions: &xpath::FunctionTable,
) -> Result<(), Diagnostic> {
    validate_expression_inner(e, scope, functions, false)
}
pub(crate) fn validate_dynamic_expression(
    e: &Expr,
    scope: &BTreeSet<String>,
    functions: &xpath::FunctionTable,
) -> Result<(), Diagnostic> {
    validate_expression_inner(e, scope, functions, true)
}
fn validate_expression_inner(
    e: &Expr,
    scope: &BTreeSet<String>,
    functions: &xpath::FunctionTable,
    dynamic_only: bool,
) -> Result<(), Diagnostic> {
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
            | Expr::Let(name, select, body, _)
            | Expr::Quantified(_, name, select, body) => {
                validate_expression_inner(select, scope, functions, dynamic_only)?;
                let mut inner = scope.clone();
                inner.insert(name.clone());
                validate_expression_inner(body, &inner, functions, dynamic_only)?;
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
            Expr::Unary(_, e)
            | Expr::TypeOp(_, e, _)
            | Expr::QNameCast(e, _, _, _)
            | Expr::StaticName(e, _, _)
            | Expr::LookupContext(e, _) => pending.push(e),
            Expr::Call(name, args) if name.starts_with("Q{") => {
                if !functions
                    .get(&(name.clone(), args.len()))
                    .is_some_and(|f| !dynamic_only || f.dynamic_visible)
                {
                    return Err(Diagnostic::new(
                        "XPST0017",
                        format!("function {name} is not available with the requested arity"),
                        "compile",
                    ));
                }
                pending.extend(args);
            }
            Expr::FunctionRef(name, arity, _) if name.starts_with("Q{") => {
                if !functions
                    .get(&(name.clone(), *arity))
                    .is_some_and(|f| !dynamic_only || f.dynamic_visible)
                {
                    return Err(Diagnostic::new(
                        "XPST0017",
                        format!("function {name} is not available with the requested arity"),
                        "compile",
                    ));
                }
            }
            Expr::Call(_, args) | Expr::Sequence(args) | Expr::ArraySquare(args) => {
                pending.extend(args)
            }
            Expr::InlineFunction(def) => {
                let mut inner = scope.clone();
                inner.extend(def.params.iter().map(|(name, _)| name.clone()));
                validate_expression_inner(&def.body, &inner, functions, dynamic_only)?;
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
                    pending.extend(key.expressions());
                }
            }
            Expr::Evaluate {
                xpath,
                context,
                namespace_context,
                schema_aware,
                base_uri,
                params,
                with_params,
                ..
            } => {
                pending.extend(with_params.iter().map(|e| e.as_ref()));
                pending.extend(namespace_context.iter().map(|e| e.as_ref()));
                pending.extend(schema_aware.iter().flatten());
                pending.extend(base_uri.iter().flatten());
                pending.push(xpath);
                if let Some(context) = context {
                    pending.push(context);
                }
                pending.extend(params.iter().map(|(_, e, _)| e));
            }
            _ => {}
        }
    }
    Ok(())
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
        functions: &'a xpath::FunctionTable,
    }
    fn instructions(
        body: &[Instruction],
        mut scope: Scope,
        templates: &TemplateNames<'_>,
    ) -> Result<(), Diagnostic> {
        fn parts(
            es: &[Expr],
            scope: &Scope,
            templates: &TemplateNames<'_>,
        ) -> Result<(), Diagnostic> {
            for e in es {
                validate_expression(e, scope, templates.functions)?;
            }
            Ok(())
        }
        fn cname(
            n: &ComputedName,
            scope: &Scope,
            templates: &TemplateNames<'_>,
        ) -> Result<(), Diagnostic> {
            parts(&n.parts, scope, templates)?;
            if let Some(es) = &n.namespace {
                parts(es, scope, templates)?;
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
                    validate_expression(e, scope, templates.functions)?;
                    parts(sep, scope, templates)
                }
                Content::Body(body, sep) => {
                    parts(sep, scope, templates)?;
                    instructions(body, scope.clone(), templates)
                }
            }
        }
        for i in body {
            match i {
                Instruction::Value(_, Some(es)) => parts(es, &scope, templates)?,
                Instruction::Apply(_, _, keys, _) | Instruction::ForEach(_, keys, _) => {
                    for k in keys {
                        for e in k.expressions() {
                            validate_expression(e, &scope, templates.functions)?;
                        }
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
                        BindingValue::Select(e) => {
                            validate_expression(e, &scope, templates.functions)?
                        }
                        BindingValue::Body(body)
                        | BindingValue::Document(body)
                        | BindingValue::Raw(body) => instructions(body, scope.clone(), templates)?,
                    }
                }
            }
            match i {
                Instruction::ComputedElement(n, _, body) => {
                    cname(n, &scope, templates)?;
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Attribute(n, c) => {
                    cname(n, &scope, templates)?;
                    value(c, &scope, templates)?;
                }
                Instruction::AnalyzeString {
                    select,
                    regex,
                    flags,
                    matching,
                    non_matching,
                } => {
                    validate_expression(select, &scope, templates.functions)?;
                    parts(regex, &scope, templates)?;
                    parts(flags, &scope, templates)?;
                    instructions(matching, scope.clone(), templates)?;
                    instructions(non_matching, scope.clone(), templates)?;
                }
                Instruction::Iterate {
                    select,
                    params,
                    on_completion,
                    body,
                } => {
                    validate_expression(select, &scope, templates.functions)?;
                    let mut inner = scope.clone();
                    for p in params {
                        if let BindingValue::Select(e) = &p.value {
                            validate_expression(e, &scope, templates.functions)?;
                        }
                        inner.insert(p.name.clone());
                    }
                    instructions(on_completion, inner.clone(), templates)?;
                    instructions(body, inner, templates)?;
                }
                Instruction::Number(spec) => {
                    for e in [&spec.value, &spec.select].into_iter().flatten() {
                        validate_expression(e, &scope, templates.functions)?;
                    }
                    parts(&spec.format, &scope, templates)?;
                    parts(&spec.ordinal, &scope, templates)?;
                    if let Some(value) = &spec.lang {
                        parts(value, &scope, templates)?;
                    }
                    if let Some(value) = &spec.letter_value {
                        parts(value, &scope, templates)?;
                    }
                    parts(&spec.grouping_separator, &scope, templates)?;
                    parts(&spec.grouping_size, &scope, templates)?;
                    if let Some(value) = &spec.start_at {
                        parts(value, &scope, templates)?;
                    }
                }
                Instruction::WherePopulated(body)
                | Instruction::OnEmpty(body)
                | Instruction::OnNonEmpty(body) => {
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Populated { main, .. } => {
                    instructions(main, scope.clone(), templates)?;
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
                    parts(&spec.href, &scope, templates)?;
                    for value in [
                        &spec.doctype_public,
                        &spec.doctype_system,
                        &spec.indent,
                        &spec.escape_uri_attributes,
                        &spec.undeclare_prefixes,
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
                        parts(value, &scope, templates)?;
                    }
                    if let Some((value, _)) = &spec.cdata {
                        parts(value, &scope, templates)?;
                    }
                    if let Some((format, _)) = &spec.format {
                        parts(format, &scope, templates)?;
                    }
                    instructions(&spec.body, scope.clone(), templates)?;
                }
                Instruction::Namespace(name, content) => {
                    parts(name, &scope, templates)?;
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
                            validate_expression(e, &scope, templates.functions)?;
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
                        parts(value, &scope, templates)?;
                    }
                }
                Instruction::ValueBody(c)
                | Instruction::Comment(c)
                | Instruction::Message(c, _)
                | Instruction::AssertFailure(c) => value(c, &scope, templates)?,
                Instruction::Pi(n, c) => {
                    parts(n, &scope, templates)?;
                    value(c, &scope, templates)?;
                }
                Instruction::Text(_) | Instruction::NextMatch(_) | Instruction::ApplyImports(_) => {
                }
                Instruction::TextValue(items) => parts(items, &scope, templates)?,
                Instruction::Value(e, _)
                | Instruction::Apply(e, _, _, _)
                | Instruction::CopyOf(e, _)
                | Instruction::Sequence(e) => validate_expression(e, &scope, templates.functions)?,
                Instruction::Bind(b) => {
                    match &b.value {
                        BindingValue::Select(e) => {
                            validate_expression(e, &scope, templates.functions)?
                        }
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
                            validate_expression(e, &scope, templates.functions)?;
                        }
                    }
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Document(body) => instructions(body, scope.clone(), templates)?,
                Instruction::Map(body) => instructions(body, scope.clone(), templates)?,
                Instruction::MapEntry { key, body } => {
                    validate_expression(key, &scope, templates.functions)?;
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Copy {
                    select, sets, body, ..
                } => {
                    if let Some(e) = select {
                        validate_expression(e, &scope, templates.functions)?;
                    }
                    instructions(sets, scope.clone(), templates)?;
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::ForEach(e, _, body) | Instruction::If(e, body) => {
                    validate_expression(e, &scope, templates.functions)?;
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::ForEachGroup(e, grouping, keys, body) => {
                    validate_expression(e, &scope, templates.functions)?;
                    if let Grouping::By(g, _) | Grouping::Adjacent(g, _) = grouping {
                        validate_expression(g, &scope, templates.functions)?;
                    }
                    if let Grouping::Starting(branches) | Grouping::Ending(branches) = grouping {
                        for branch in branches {
                            for predicate in crate::pattern::expressions(branch) {
                                validate_expression(predicate, &scope, templates.functions)?;
                            }
                        }
                    }
                    for k in keys {
                        for e in k.expressions() {
                            validate_expression(e, &scope, templates.functions)?;
                        }
                    }
                    instructions(body, scope.clone(), templates)?;
                }
                Instruction::Choose(branches) => {
                    for (test, body) in branches {
                        if let Some(e) = test {
                            validate_expression(e, &scope, templates.functions)?;
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
                            BindingValue::Select(e) => {
                                validate_expression(e, &scope, templates.functions)?
                            }
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
        functions: &sheet.functions,
    };
    for set in sheet.attribute_sets.values() {
        instructions(&set.body, globals.clone(), &templates)?;
    }
    for b in &sheet.globals {
        match &b.value {
            BindingValue::Select(e) => validate_expression(e, &globals, templates.functions)?,
            BindingValue::Body(body) | BindingValue::Document(body) | BindingValue::Raw(body) => {
                instructions(body, globals.clone(), &templates)?
            }
        }
    }
    for accumulator in sheet.accumulators.values() {
        validate_expression(&accumulator.initial, &globals, templates.functions)?;
        let mut scope = globals.clone();
        scope.insert("value".into());
        for rule in &accumulator.rules {
            for branch in &rule.branches {
                for predicate in crate::pattern::expressions(branch) {
                    validate_expression(predicate, &globals, templates.functions)?;
                }
            }
            validate_expression(&rule.select, &scope, templates.functions)?;
        }
    }
    for definitions in sheet.keys.values() {
        for definition in definitions {
            for branch in &definition.branches {
                for predicate in crate::pattern::expressions(branch) {
                    validate_expression(predicate, &globals, templates.functions)?;
                }
            }
            validate_expression(&definition.value, &globals, templates.functions)?;
        }
    }
    for function in sheet.functions.values() {
        let mut scope = globals.clone();
        scope.extend(function.params.iter().map(|(p, _)| p.clone()));
        match &function.body {
            xpath::UserFunctionBody::Expression(body) => {
                validate_expression(body, &scope, templates.functions)?
            }
            xpath::UserFunctionBody::Constructor(body) => instructions(body, scope, &templates)?,
        }
    }
    for t in &sheet.templates {
        if let Some(branches) = &t.pattern {
            for b in branches {
                for expression_in_pattern in crate::pattern::expressions(b) {
                    validate_expression(expression_in_pattern, &globals, templates.functions)?;
                }
            }
        }
        let mut scope = globals.clone();
        for p in &t.params {
            match &p.value {
                BindingValue::Select(e) => validate_expression(e, &scope, templates.functions)?,
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
