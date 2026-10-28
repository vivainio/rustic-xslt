//! Incremental result construction: keep a start tag pending until content
//! arrives, so copied attributes and namespace fixup remain well-defined.
use crate::Diagnostic;
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub(crate) struct Name {
    pub qname: String,
    pub uri: Option<String>,
    pub local: String,
}
#[derive(Debug, Clone)]
pub(crate) struct Element {
    pub name: Name,
    pub namespaces: BTreeMap<String, String>,
    pub attributes: Vec<(Name, String)>,
    pub inherit_namespaces: bool,
}
struct Open {
    element: Element,
    pending: bool,
    inherited: BTreeMap<String, String>,
}
struct Simple {
    separator: String,
    parts: Vec<(String, bool)>,
}
pub(crate) struct Output {
    pub value: String,
    pub max: usize,
    pub text: bool,
    stack: Vec<Open>,
    document: bool,
    simple: Option<Simple>,
    adjacent_atomic: bool,
}
impl Output {
    pub fn new(max: usize, text: bool) -> Self {
        Self {
            value: String::new(),
            max,
            text,
            stack: vec![],
            document: false,
            simple: None,
            adjacent_atomic: false,
        }
    }
    pub fn simple(max: usize, separator: String) -> Self {
        let mut value = Self::new(max, true);
        value.simple = Some(Simple {
            separator,
            parts: vec![],
        });
        value
    }
    pub fn finish_simple(self) -> Result<String, Diagnostic> {
        let Some(simple) = self.simple else {
            return Ok(self.value);
        };
        let separators = simple
            .parts
            .len()
            .saturating_sub(1)
            .saturating_mul(simple.separator.len());
        if separators > self.max.saturating_sub(self.value.len()) {
            return Err(Diagnostic::new(
                "XPDY0130",
                "simple-content byte limit exceeded",
                "run",
            ));
        }
        Ok(simple
            .parts
            .into_iter()
            .map(|(s, _)| s)
            .collect::<Vec<_>>()
            .join(&simple.separator))
    }
    fn simple_part(&mut self, value: &str, text: bool) -> Result<(), Diagnostic> {
        self.push(value)?;
        let simple = self.simple.as_mut().unwrap();
        if text && simple.parts.last().is_some_and(|(_, previous)| *previous) {
            simple.parts.last_mut().unwrap().0.push_str(value);
        } else {
            simple.parts.push((value.into(), text));
        }
        Ok(())
    }
    pub fn simple_node(&mut self, node: crate::node::XNode<'_>) -> Result<bool, Diagnostic> {
        if self.simple.is_none() || !self.stack.is_empty() {
            return Ok(false);
        }
        if node.is_text() {
            self.text(&node.string())?;
        } else {
            self.simple_part(&node.string(), false)?;
        }
        Ok(true)
    }
    pub fn atomic(&mut self, value: &str) -> Result<(), Diagnostic> {
        if self.simple.is_some() && self.stack.is_empty() {
            return self.simple_part(value, false);
        }
        if self.adjacent_atomic {
            self.text(" ")?;
        }
        self.text(value)?;
        self.adjacent_atomic = true;
        Ok(())
    }
    pub fn document(max: usize) -> Self {
        let mut value = Self::new(max, false);
        value.document = true;
        value
    }
    pub fn push(&mut self, s: &str) -> Result<(), Diagnostic> {
        if s.len() > self.max.saturating_sub(self.value.len()) {
            return Err(Diagnostic::new("XPDY0130", "output limit exceeded", "run"));
        }
        self.value.push_str(s);
        Ok(())
    }
    fn flush_start(&mut self) -> Result<(), Diagnostic> {
        let Some(open) = self.stack.last_mut() else {
            return Ok(());
        };
        if !open.pending {
            return Ok(());
        }
        open.pending = false;
        if self.text {
            return Ok(());
        }
        let e = open.element.clone();
        let inherited = open.inherited.clone();
        self.push("<")?;
        self.push(&e.name.qname)?;
        for (prefix, uri) in &e.namespaces {
            if prefix == "xml" || inherited.get(prefix).map(String::as_str).unwrap_or("") == uri {
                continue;
            }
            self.push(" xmlns")?;
            if !prefix.is_empty() {
                self.push(":")?;
                self.push(prefix)?;
            }
            self.push("=\"")?;
            self.push(&escape(uri, true))?;
            self.push("\"")?;
        }
        for (name, value) in &e.attributes {
            self.push(" ")?;
            self.push(&name.qname)?;
            self.push("=\"")?;
            self.push(&escape(value, true))?;
            self.push("\"")?;
        }
        self.push(">")
    }
    pub fn start(&mut self, mut element: Element) -> Result<(), Diagnostic> {
        self.flush_start()?;
        self.adjacent_atomic = false;
        if self.simple.is_some() && self.stack.is_empty() {
            self.simple_part("", false)?;
        }
        let inherited = self
            .stack
            .last()
            .map(|o| o.element.namespaces.clone())
            .unwrap_or_default();
        let mut namespaces = if self
            .stack
            .last()
            .is_none_or(|o| o.element.inherit_namespaces)
        {
            inherited.clone()
        } else {
            BTreeMap::new()
        };
        namespaces.extend(element.namespaces);
        let prefix = element.name.qname.split_once(':').map_or("", |(p, _)| p);
        namespaces.insert(prefix.into(), element.name.uri.clone().unwrap_or_default());
        element.namespaces = namespaces;
        let attributes = std::mem::take(&mut element.attributes);
        self.stack.push(Open {
            element,
            pending: true,
            inherited,
        });
        for (name, value) in attributes {
            self.attribute(name, value)?;
        }
        Ok(())
    }
    pub fn attribute(&mut self, mut name: Name, value: String) -> Result<(), Diagnostic> {
        if self.simple.is_some() && self.stack.is_empty() {
            return self.simple_part(&value, false);
        }
        let code = if self.stack.is_empty() && self.document {
            "XTDE0420"
        } else {
            "XTDE0410"
        };
        let Some(open) = self.stack.last_mut().filter(|o| o.pending) else {
            return Err(Diagnostic::new(
                code,
                "attribute created without an open result start tag",
                "run",
            ));
        };
        if let Some(uri) = &name.uri {
            let desired = name.qname.split_once(':').map(|(p, _)| p).unwrap_or("");
            let mut prefix = desired.to_owned();
            if prefix.is_empty()
                || open
                    .element
                    .namespaces
                    .get(&prefix)
                    .is_some_and(|u| u != uri)
            {
                let mut index = 1;
                loop {
                    let p = format!("ns{index}");
                    if open.element.namespaces.get(&p).is_none_or(|u| u == uri) {
                        prefix = p;
                        break;
                    }
                    index += 1;
                }
                name.qname = format!("{prefix}:{}", name.local);
            }
            open.element.namespaces.insert(prefix, uri.clone());
        }
        if let Some(a) = open
            .element
            .attributes
            .iter_mut()
            .find(|(n, _)| n.uri == name.uri && n.local == name.local)
        {
            *a = (name, value);
        } else {
            open.element.attributes.push((name, value));
        }
        Ok(())
    }
    pub fn end(&mut self) -> Result<(), Diagnostic> {
        self.adjacent_atomic = false;
        self.flush_start()?;
        let Some(open) = self.stack.pop() else {
            return Err(Diagnostic::new(
                "INTERNAL",
                "unbalanced result element",
                "run",
            ));
        };
        if !self.text {
            self.push("</")?;
            self.push(&open.element.name.qname)?;
            self.push(">")?;
        }
        Ok(())
    }
    pub fn text(&mut self, s: &str) -> Result<(), Diagnostic> {
        if s.is_empty() {
            return Ok(());
        }
        self.adjacent_atomic = false;
        self.flush_start()?;
        if self.simple.is_some() {
            if self.stack.is_empty() {
                return self.simple_part(s, true);
            }
            self.push(s)?;
            self.simple
                .as_mut()
                .unwrap()
                .parts
                .last_mut()
                .unwrap()
                .0
                .push_str(s);
            return Ok(());
        }
        if self.text {
            self.push(s)
        } else {
            self.push(&escape(s, false))
        }
    }
    pub fn comment(&mut self, s: &str) -> Result<(), Diagnostic> {
        self.adjacent_atomic = false;
        if self.simple.is_some() && self.stack.is_empty() {
            return self.simple_part(s, false);
        }
        self.flush_start()?;
        if !self.text {
            self.push("<!--")?;
            self.push(s)?;
            self.push("-->")?;
        }
        Ok(())
    }
    pub fn pi(&mut self, target: &str, s: &str) -> Result<(), Diagnostic> {
        self.adjacent_atomic = false;
        if self.simple.is_some() && self.stack.is_empty() {
            return self.simple_part(s, false);
        }
        self.flush_start()?;
        if !self.text {
            self.push("<?")?;
            self.push(target)?;
            if !s.is_empty() {
                self.push(" ")?;
                self.push(s)?;
            }
            self.push("?>")?;
        }
        Ok(())
    }
}
pub(crate) fn escape(value: &str, attr: bool) -> String {
    let mut s = String::new();
    for c in value.chars() {
        s.push_str(match c {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' if attr => "&quot;",
            '\n' if attr => "&#10;",
            '\r' => "&#13;",
            '\t' if attr => "&#9;",
            _ => {
                s.push(c);
                continue;
            }
        });
    }
    s
}
pub(crate) fn source_name(node: roxmltree::Node<'_, '_>) -> Name {
    let tag = node.tag_name();
    let raw = &node.document().input_text()[node.range().start + 1..];
    let qname = raw
        .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .next()
        .unwrap_or(tag.name());
    Name {
        qname: qname.into(),
        uri: tag
            .namespace()
            .filter(|uri| !uri.is_empty())
            .map(str::to_owned),
        local: tag.name().into(),
    }
}
pub(crate) fn attribute_name(
    node: roxmltree::Node<'_, '_>,
    local: &str,
    uri: Option<&str>,
) -> Name {
    let qname = uri
        .and_then(|u| node.lookup_prefix(u))
        .filter(|p| !p.is_empty())
        .map_or_else(|| local.into(), |p| format!("{p}:{local}"));
    Name {
        qname,
        uri: uri.map(str::to_owned),
        local: local.into(),
    }
}
pub(crate) fn source_element(node: roxmltree::Node<'_, '_>, attributes: bool) -> Element {
    Element {
        name: source_name(node),
        namespaces: node
            .namespaces()
            .map(|ns| (ns.name().unwrap_or("").into(), ns.uri().into()))
            .collect(),
        inherit_namespaces: true,
        attributes: if attributes {
            node.attributes()
                .map(|a| {
                    (
                        attribute_name(node, a.name(), a.namespace()),
                        a.value().into(),
                    )
                })
                .collect()
        } else {
            vec![]
        },
    }
}
