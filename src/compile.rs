use crate::{
    Diagnostic, Stylesheet,
    xpath::{self, Expr},
};
use roxmltree::Node;
use std::collections::BTreeMap;
pub(crate) const XSL: &str = "http://www.w3.org/1999/XSL/Transform";

#[derive(Debug)]
pub(crate) enum Instruction {
    Text(String),
    Value(Expr, Option<Vec<Expr>>),
    Apply(Expr, String, Vec<Sort>),
    Copy(Vec<Instruction>),
    CopyOf(Expr),
    ComputedElement(ComputedName, Vec<Instruction>),
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
    Body(Vec<Instruction>),
}
#[derive(Debug)]
pub(crate) struct Binding {
    pub name: String,
    pub value: Expr,
    pub parameter: bool,
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
fn name(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some('a'..='z' | 'A'..='Z' | '_'))
        && c.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
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
    namespaces
        .entry("fn".into())
        .or_insert_with(|| "http://www.w3.org/2005/xpath-functions".into());
    namespaces
}
fn expr(n: Node<'_, '_>, s: &str) -> Result<Expr, Diagnostic> {
    let namespaces = namespaces(n);
    xpath::parse(s, namespaces).map_err(|mut e| {
        let pos = n.document().text_pos_at(n.range().start);
        e.line = pos.row;
        e.column = pos.col;
        e
    })
}
fn attrs(n: Node<'_, '_>, allowed: &[&str]) -> Result<(), Diagnostic> {
    for a in n.attributes() {
        if a.namespace().is_some() || !allowed.contains(&a.name()) {
            return Err(error(n, format!("unsupported attribute: {}", a.name())));
        }
    }
    Ok(())
}
fn empty(n: Node<'_, '_>) -> Result<(), Diagnostic> {
    if n.children()
        .any(|c| c.is_element() || c.is_text() && !c.text().unwrap_or_default().trim().is_empty())
    {
        return Err(error(n, "instruction content is unsupported"));
    }
    Ok(())
}
fn binding(n: Node<'_, '_>, parameter: bool) -> Result<Binding, Diagnostic> {
    attrs(n, &["name", "select"])?;
    let id = required(n, "name")?;
    if !name(id) {
        return Err(error(n, "expected an unprefixed binding name"));
    }
    let value = if let Some(s) = n.attribute("select") {
        empty(n)?;
        expr(n, s)?
    } else {
        if n.children().any(|c| c.is_element()) {
            return Err(error(
                n,
                "binding content currently supports text only; use select for values",
            ));
        }
        Expr::String(
            n.children()
                .filter(|c| c.is_text())
                .filter_map(|c| c.text())
                .collect(),
        )
    };
    Ok(Binding {
        name: id.into(),
        value,
        parameter,
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
            let mut closed = false;
            for c in chars.by_ref() {
                if let Some(q) = quote {
                    expression.push(c);
                    if c == q {
                        quote = None;
                    }
                } else if c == '\'' || c == '"' {
                    quote = Some(c);
                    expression.push(c);
                } else if c == '}' {
                    closed = true;
                    break;
                } else if c == '{' {
                    return Err(error(n, "nested AVT braces are unsupported"));
                } else {
                    expression.push(c);
                }
            }
            if !closed {
                return Err(error(n, "unclosed attribute value template"));
            }
            parts.push(expr(n, &expression)?);
        } else if c == '}' {
            if chars.next() != Some('}') {
                return Err(error(n, "unescaped closing AVT brace"));
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
        if n.attribute("separator").is_some() {
            return Err(error(
                n,
                "separator with a sequence-constructor body is not implemented",
            ));
        }
        Ok(Content::Body(body(n, depth, false)?))
    }
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
        } else if c.is_element() || c.is_text() && !c.text().unwrap_or_default().trim().is_empty() {
            if n.tag_name().name() == "apply-templates" {
                return Err(error(
                    c,
                    "apply-templates content currently supports sort keys only",
                ));
            }
            content = true;
        }
    }
    Ok(keys)
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
            if !t.trim().is_empty() {
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
                        attrs(n, &["name", "namespace"])?;
                        out.push(Instruction::ComputedElement(
                            computed_name(n)?,
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
                        attrs(n, &["select", "separator"])?;
                        out.push(Instruction::Comment(content(n, depth + 1)?));
                    }
                    "processing-instruction" => {
                        attrs(n, &["name", "select", "separator"])?;
                        out.push(Instruction::Pi(
                            avt(n, required(n, "name")?)?,
                            content(n, depth + 1)?,
                        ));
                    }
                    "copy" => {
                        attrs(n, &[])?;
                        out.push(Instruction::Copy(body(n, depth + 1, false)?));
                    }
                    "copy-of" => {
                        attrs(n, &["select"])?;
                        empty(n)?;
                        out.push(Instruction::CopyOf(expr(n, required(n, "select")?)?));
                    }
                    "variable" => out.push(Instruction::Bind(binding(n, false)?)),
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
                            if c.is_text() && !c.text().unwrap_or_default().trim().is_empty() {
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
                    "call-template" => {
                        attrs(n, &["name"])?;
                        let id = required(n, "name")?;
                        if !name(id) {
                            return Err(error(n, "expected unprefixed template name"));
                        }
                        let mut params = vec![];
                        for c in n.children() {
                            if c.is_text() && !c.text().unwrap_or_default().trim().is_empty() {
                                return Err(error(c, "unexpected call-template text"));
                            }
                            if !c.is_element() {
                                continue;
                            }
                            if c.tag_name().namespace() != Some(XSL)
                                || c.tag_name().name() != "with-param"
                            {
                                return Err(error(c, "expected with-param"));
                            }
                            let b = binding(c, true)?;
                            if params.iter().any(|p: &Binding| p.name == b.name) {
                                return Err(error(c, "duplicate with-param"));
                            }
                            params.push(b);
                        }
                        out.push(Instruction::Call(id.into(), params));
                    }
                    other => {
                        return Err(error(n, format!("unsupported XSLT instruction: {other}")));
                    }
                }
            } else {
                let mut attributes = vec![];
                for a in n.attributes() {
                    if a.namespace() == Some(XSL) {
                        return Err(error(n, "unsupported XSLT attribute on literal result"));
                    }
                    attributes.push((
                        crate::result::attribute_name(n, a.name(), a.namespace()),
                        avt(n, a.value())?,
                    ));
                }
                let mut element = crate::result::source_element(n, false);
                element.namespaces.retain(|_, uri| uri != XSL);
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
fn mode(n: Node<'_, '_>, value: &str, current: bool) -> Result<String, Diagnostic> {
    if value == "#default" {
        return Ok(String::new());
    }
    if current && value == "#current" {
        return Ok(value.into());
    }
    if !name(value) {
        return Err(error(n, "expected an unprefixed mode name or #default"));
    }
    Ok(value.into())
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
        omit_declaration: false,
        text_output: false,
        backwards: root.attribute("version") == Some("1.0"),
    };
    let mut output_seen = false;
    for n in root.children() {
        if n.is_text() && !n.text().unwrap_or_default().trim().is_empty() {
            return Err(error(n, "unexpected top-level text"));
        }
        if !n.is_element() {
            continue;
        }
        if n.tag_name().namespace() != Some(XSL) {
            return Err(error(n, "unsupported top-level element"));
        }
        match n.tag_name().name() {
            "param" | "variable" => {
                let b = binding(n, n.tag_name().name() == "param")?;
                if sheet.globals.iter().any(|g| g.name == b.name) {
                    return Err(error(n, "duplicate global binding"));
                }
                sheet.globals.push(b);
            }
            "template" => {
                attrs(n, &["match", "name", "mode", "priority"])?;
                let p = n
                    .attribute("match")
                    .map(|p| crate::pattern::parse(p, namespaces(n)))
                    .transpose()?;
                let id = n.attribute("name").map(str::to_owned);
                if p.is_none() && id.is_none() {
                    return Err(error(n, "template requires match or name"));
                }
                if let Some(id) = &id
                    && (!name(id) || sheet.templates.iter().any(|t| t.name.as_ref() == Some(id)))
                {
                    return Err(error(n, "invalid or duplicate named template"));
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
                        let b = binding(c, true)?;
                        if params.iter().any(|p: &Binding| p.name == b.name) {
                            return Err(error(c, "duplicate template parameter"));
                        }
                        params.push(b);
                    } else if c.is_element()
                        || c.is_text() && !c.text().unwrap_or_default().trim().is_empty()
                    {
                        content = true;
                    }
                }
                let modes = if n.attribute("mode") == Some("#all") {
                    vec!["#all".into()]
                } else {
                    n.attribute("mode")
                        .unwrap_or("#default")
                        .split_whitespace()
                        .map(|m| mode(n, m, false))
                        .collect::<Result<Vec<_>, _>>()?
                };
                if modes.is_empty() {
                    return Err(error(n, "empty mode list"));
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
                attrs(n, &["method", "omit-xml-declaration"])?;
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
                Expr::Unary(_, e) => pending.push(e),
                Expr::Call(_, args) | Expr::Sequence(args) => pending.extend(args),
                _ => {}
            }
        }
        Ok(())
    }
    fn instructions(
        body: &[Instruction],
        mut scope: Scope,
        templates: &Scope,
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
        fn value(c: &Content, scope: &Scope, templates: &Scope) -> Result<(), Diagnostic> {
            match c {
                Content::Select(e, sep) => {
                    expression(e, scope)?;
                    parts(sep, scope)
                }
                Content::Body(body) => instructions(body, scope.clone(), templates),
            }
        }
        for i in body {
            match i {
                Instruction::Value(_, Some(es)) => parts(es, &scope)?,
                Instruction::Apply(_, _, keys) | Instruction::ForEach(_, keys, _) => {
                    for k in keys {
                        expression(&k.select, &scope)?;
                    }
                }
                _ => {}
            }
            match i {
                Instruction::ComputedElement(n, body) => {
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
                Instruction::Text(_) => {}
                Instruction::Value(e, _) | Instruction::Apply(e, _, _) | Instruction::CopyOf(e) => {
                    expression(e, &scope)?
                }
                Instruction::Bind(b) => {
                    expression(&b.value, &scope)?;
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
                Instruction::Copy(body) => instructions(body, scope.clone(), templates)?,
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
                    if !templates.contains(name) {
                        return Err(Diagnostic::new(
                            "XTSE0650",
                            format!("unknown named template: {name}"),
                            "compile",
                        ));
                    }
                    for p in params {
                        expression(&p.value, &scope)?;
                    }
                }
            }
        }
        Ok(())
    }
    let globals: Scope = sheet.globals.iter().map(|b| b.name.clone()).collect();
    let templates: Scope = sheet
        .templates
        .iter()
        .filter_map(|t| t.name.clone())
        .collect();
    for b in &sheet.globals {
        expression(&b.value, &globals)?;
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
            expression(&p.value, &scope)?;
            scope.insert(p.name.clone());
        }
        instructions(&t.body, scope, &templates)?;
    }
    Ok(())
}
