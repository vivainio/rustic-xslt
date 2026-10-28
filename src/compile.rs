use crate::{Diagnostic, Stylesheet};
use roxmltree::Node;
pub(crate) const XSL: &str = "http://www.w3.org/1999/XSL/Transform";

#[derive(Debug, Clone)]
pub(crate) enum Select {
    Children,
    Path { absolute: bool, names: Vec<String> },
    SelfNode,
}
#[derive(Debug)]
pub(crate) enum Instruction {
    Text(String),
    Value(Select),
    Apply(Select),
    Element {
        name: String,
        attributes: Vec<(String, String)>,
        body: Vec<Instruction>,
    },
}
#[derive(Debug)]
pub(crate) struct Template {
    pub pattern: String,
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
pub(crate) fn select(n: Node<'_, '_>, s: &str) -> Result<Select, Diagnostic> {
    let s = s.trim();
    if s == "." {
        return Ok(Select::SelfNode);
    }
    if s == "node()" {
        return Ok(Select::Children);
    }
    let absolute = s.starts_with('/');
    let parts: Vec<_> = s.trim_start_matches('/').split('/').collect();
    if s == "/" {
        return Ok(Select::Path {
            absolute: true,
            names: vec![],
        });
    }
    if s.starts_with("//") || parts.iter().any(|p| !name(p)) {
        return Err(error(n, format!("unsupported XPath expression: {s}")));
    }
    Ok(Select::Path {
        absolute,
        names: parts.into_iter().map(str::to_owned).collect(),
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
fn body(parent: Node<'_, '_>, depth: usize) -> Result<Vec<Instruction>, Diagnostic> {
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
                        attrs(n, &["select"])?;
                        if n.children().any(|c| {
                            c.is_element()
                                || c.is_text() && !c.text().unwrap_or_default().trim().is_empty()
                        }) {
                            return Err(error(n, "instruction content is unsupported"));
                        }
                        let value = if let Some(s) = n.attribute("select") {
                            select(n, s)?
                        } else if n.tag_name().name() == "apply-templates" {
                            Select::Children
                        } else {
                            return Err(error(n, "value-of requires select"));
                        };
                        out.push(if n.tag_name().name() == "value-of" {
                            Instruction::Value(value)
                        } else {
                            Instruction::Apply(value)
                        });
                    }
                    other => {
                        return Err(error(n, format!("unsupported XSLT instruction: {other}")));
                    }
                }
            } else {
                if n.tag_name().namespace().is_some() {
                    return Err(error(n, "namespaced literal results are not implemented"));
                }
                let mut attributes = vec![];
                for a in n.attributes() {
                    if a.namespace().is_some() || a.value().contains(['{', '}']) {
                        return Err(error(
                            n,
                            "namespaced attributes and AVTs are not implemented",
                        ));
                    }
                    attributes.push((a.name().into(), a.value().into()));
                }
                out.push(Instruction::Element {
                    name: n.tag_name().name().into(),
                    attributes,
                    body: body(n, depth + 1)?,
                });
            }
        }
    }
    Ok(out)
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
        omit_declaration: false,
        text_output: false,
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
            "template" => {
                attrs(n, &["match"])?;
                let pattern = n
                    .attribute("match")
                    .ok_or_else(|| error(n, "template requires match"))?;
                let p = select(n, pattern)?;
                if !matches!(p, Select::Path { .. }) {
                    return Err(error(n, "unsupported match pattern"));
                }
                sheet.templates.push(Template {
                    pattern: pattern.trim().into(),
                    body: body(n, 0)?,
                });
            }
            "output" => {
                attrs(n, &["method", "omit-xml-declaration"])?;
                if output_seen || n.children().any(|c| c.is_element()) {
                    return Err(error(n, "multiple or nonempty xsl:output is unsupported"));
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
    Ok(sheet)
}
