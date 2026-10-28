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
    Pi(Vec<Expr>, Content),
    Element {
        element: crate::result::Element,
        attributes: Vec<(crate::result::Name, Vec<Expr>)>,
        body: Vec<Instruction>,
    },
    ForEach(Expr, Vec<Sort>, Vec<Instruction>),
    If(Expr, Vec<Instruction>),
    Choose(Vec<(Option<Expr>, Vec<Instruction>)>),
    Bind(Binding),
    Call(String, Vec<Binding>),
    NextMatch(Vec<Binding>),
}
#[derive(Debug)]
pub(crate) struct Sort {
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
    pub body: Vec<Instruction>,
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
        if a.namespace() == Some("http://www.w3.org/XML/1998/namespace") && a.name() == "space" {
            continue;
        }
        if a.namespace().is_none() && a.name() == "default-mode" {
            default_mode(n)?;
            continue;
        }
        if a.namespace().is_none() && a.name() == "exclude-result-prefixes" {
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
            &["name", "select", "as", "required", "tunnel"]
        } else if parameter {
            &["name", "select", "as", "tunnel"]
        } else {
            &["name", "select", "as"]
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
        } else {
            if ty
                .as_ref()
                .is_some_and(|ty| matches!(ty.kind, crate::seqtype::Kind::Attribute))
            {
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
        | Instruction::CopyOf(..)
        | Instruction::Copy { .. }
        | Instruction::Document(_) => true,
        Instruction::ForEach(_, _, b) | Instruction::If(_, b) => top_sequence(b),
        Instruction::Choose(bs) => bs.iter().any(|(_, b)| top_sequence(b)),
        _ => false,
    })
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
            attrs(c, &["select", "data-type", "order", "case-order"])?;
            empty(c)?;
            let numeric = match c.attribute("data-type").unwrap_or("text") {
                "text" => false,
                "number" => true,
                _ => return Err(error(c, "unsupported sort data-type")),
            };
            let descending = match c.attribute("order").unwrap_or("ascending") {
                "ascending" => false,
                "descending" => true,
                _ => return Err(error(c, "unsupported sort order")),
            };
            let case_order = c.attribute("case-order").unwrap_or("");
            if !["", "upper-first", "lower-first"].contains(&case_order) {
                return Err(error(c, "unsupported case-order"));
            }
            keys.push(Sort {
                select: expr(c, c.attribute("select").unwrap_or("."))?,
                numeric,
                descending,
                case_order: case_order.into(),
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
                out.push(Instruction::Text(t.into()));
            }
        } else if n.is_element() {
            if n.tag_name().namespace() == Some(XSL) {
                match n.tag_name().name() {
                    "param" if skip_params => continue,
                    "text" => {
                        attrs(n, &[])?;
                        if n.children().any(|c| c.is_element()) {
                            return Err(error(n, "xsl:text cannot contain elements"));
                        }
                        out.push(Instruction::Text(
                            n.children()
                                .filter(|c| c.is_text())
                                .filter_map(|c| c.text())
                                .collect(),
                        ));
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
                    "sort"
                        if parent.tag_name().namespace() == Some(XSL)
                            && parent.tag_name().name() == "for-each" =>
                    {
                        continue;
                    }
                    "element" => {
                        attrs(n, &["name", "namespace", "inherit-namespaces"])?;
                        out.push(Instruction::ComputedElement(
                            computed_name(n)?,
                            boolean_attribute(n, "inherit-namespaces", true)?,
                            body(n, depth + 1, false)?,
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
                            "inherit-namespaces",
                            "default-mode",
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
                out.push(Instruction::Element {
                    element,
                    attributes,
                    body: body(n, depth + 1, false)?,
                });
            }
        }
    }
    Ok(out)
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
pub(crate) fn compile(source: &str) -> Result<Stylesheet, Diagnostic> {
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
        space_rules: vec![],
        default_mode: default_mode(root)?,
        modes: BTreeMap::new(),
        omit_declaration: false,
        text_output: false,
        backwards: root.attribute("version") == Some("1.0"),
        version: match root.attribute("version") {
            Some("1.0") => 1,
            Some("2.0") => 2,
            _ => 3,
        },
    };
    let mut output_seen = false;
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
        match n.tag_name().name() {
            "mode" => {
                use crate::mode::OnNoMatch;
                attrs(
                    n,
                    &[
                        "name",
                        "on-no-match",
                        "on-multiple-match",
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
                if sheet.globals.iter().any(|g| g.name == b.name) {
                    return Err(static_error(n, "XTSE0630", "duplicate global binding"));
                }
                sheet.globals.push(b);
            }
            "template" => {
                attrs(n, &["match", "name", "mode", "priority"])?;
                let p = n
                    .attribute("match")
                    .map(|p| crate::pattern::parse(p, namespaces(n)))
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
                if let Some(id) = &id
                    && sheet.templates.iter().any(|t| t.name.as_ref() == Some(id))
                {
                    return Err(static_error(n, "XTSE0660", "duplicate named template"));
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
                sheet.templates.push(Template {
                    pattern: p,
                    modes,
                    priority,
                    name: id,
                    params,
                    body: body(n, 0, true)?,
                });
            }
            "output" => {
                attrs(
                    n,
                    &[
                        "method",
                        "omit-xml-declaration",
                        "encoding",
                        "indent",
                        "version",
                    ],
                )?;
                if n.attribute("encoding")
                    .is_some_and(|s| !s.eq_ignore_ascii_case("UTF-8"))
                {
                    return Err(error(
                        n,
                        "only UTF-8 output encoding is currently supported",
                    ));
                }
                if n.attribute("indent").is_some_and(|s| s != "no") {
                    return Err(error(n, "indentation is not implemented yet"));
                }
                if n.attribute("version").is_some_and(|s| s != "1.0") {
                    return Err(error(n, "only XML 1.0 output is currently supported"));
                }
                empty(n)?;
                if output_seen {
                    return Err(error(n, "multiple xsl:output declarations are unsupported"));
                }
                output_seen = true;
                sheet.text_output = match n.attribute("method").unwrap_or("xml") {
                    "xml" => false,
                    "text" => true,
                    _ => return Err(error(n, "unsupported output method")),
                };
                sheet.omit_declaration = match n.attribute("omit-xml-declaration").unwrap_or("no") {
                    "yes" => true,
                    "no" => false,
                    _ => return Err(error(n, "invalid omit-xml-declaration")),
                };
            }
            other => return Err(error(n, format!("unsupported declaration: {other}"))),
        }
    }
    validate(&sheet)?;
    Ok(sheet)
}

// Check lexical references even in branches that will not execute.
fn validate(sheet: &Stylesheet) -> Result<(), Diagnostic> {
    use std::collections::BTreeSet;
    type Scope = BTreeSet<String>;
    struct TemplateNames<'a> {
        named: BTreeMap<String, &'a Template>,
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
                Expr::Binary(_, a, b) => {
                    pending.push(a);
                    pending.push(b);
                }
                Expr::If(c, y, n) => {
                    pending.push(c);
                    pending.push(y);
                    pending.push(n);
                }
                Expr::Unary(_, e) | Expr::TypeOp(_, e, _) => pending.push(e),
                Expr::Call(_, args) | Expr::Sequence(args) => pending.extend(args),
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
            if let Instruction::Apply(_, _, _, params) | Instruction::NextMatch(params) = i {
                for p in params {
                    match &p.value {
                        BindingValue::Select(e) => expression(e, &scope)?,
                        BindingValue::Body(body) | BindingValue::Document(body) => {
                            instructions(body, scope.clone(), templates)?
                        }
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
                Instruction::Comment(c) => value(c, &scope, templates)?,
                Instruction::Pi(n, c) => {
                    parts(n, &scope)?;
                    value(c, &scope, templates)?;
                }
                Instruction::Text(_) | Instruction::NextMatch(_) => {}
                Instruction::Value(e, _)
                | Instruction::Apply(e, _, _, _)
                | Instruction::CopyOf(e, _)
                | Instruction::Sequence(e) => expression(e, &scope)?,
                Instruction::Bind(b) => {
                    match &b.value {
                        BindingValue::Select(e) => expression(e, &scope)?,
                        BindingValue::Body(body) | BindingValue::Document(body) => {
                            instructions(body, scope.clone(), templates)?
                        }
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
                            BindingValue::Body(body) | BindingValue::Document(body) => {
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
    let templates = TemplateNames {
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
            BindingValue::Body(body) | BindingValue::Document(body) => {
                instructions(body, globals.clone(), &templates)?
            }
        }
    }
    for t in &sheet.templates {
        if let Some(branches) = &t.pattern {
            for b in branches {
                for s in &b.steps {
                    for p in &s.predicates {
                        expression(p, &globals)?;
                    }
                }
            }
        }
        let mut scope = globals.clone();
        for p in &t.params {
            match &p.value {
                BindingValue::Select(e) => expression(e, &scope)?,
                BindingValue::Body(body) | BindingValue::Document(body) => {
                    instructions(body, scope.clone(), &templates)?
                }
            }
            scope.insert(p.name.clone());
        }
        instructions(&t.body, scope, &templates)?;
    }
    Ok(())
}
