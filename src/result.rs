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
/// The markup flavour of the final serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Markup {
    #[default]
    Xml,
    Html,
    Xhtml,
}
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "basefont", "br", "col", "embed", "frame", "hr", "img", "input", "isindex",
    "link", "meta", "param", "source", "track", "wbr",
];
const BOOLEAN_ATTRIBUTES: &[&str] = &[
    "checked", "compact", "declare", "defer", "disabled", "ismap", "multiple", "nohref",
    "noresize", "noshade", "nowrap", "readonly", "selected",
];
fn html_name(name: &Name, markup: Markup) -> Option<String> {
    let in_html = match markup {
        Markup::Html => name
            .uri
            .as_deref()
            .is_none_or(|u| u.is_empty() || u == XHTML),
        Markup::Xhtml => name.uri.as_deref() == Some(XHTML),
        Markup::Xml => false,
    };
    in_html.then(|| name.local.to_ascii_lowercase())
}
pub(crate) const XHTML: &str = "http://www.w3.org/1999/xhtml";
/// Output encodings. The result is always a Rust string; characters outside the
/// encoding become numeric character references (an error for the text method).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Encoding {
    #[default]
    Utf8,
    Latin1,
    Ascii,
}
/// XML declaration settings shared by principal, secondary and record outputs.
pub(crate) fn xml_declaration(
    encoding: Encoding,
    omit: bool,
    standalone: Option<bool>,
) -> Result<Option<String>, Diagnostic> {
    if omit {
        if standalone.is_some() {
            return Err(Diagnostic::new(
                "SEPM0009",
                "standalone requires an XML declaration",
                "run",
            ));
        }
        return Ok(None);
    }
    let standalone = standalone.map_or(String::new(), |value| {
        format!(" standalone=\"{}\"", if value { "yes" } else { "no" })
    });
    Ok(Some(format!(
        "<?xml version=\"1.0\" encoding=\"{}\"{standalone}?>",
        encoding.name()
    )))
}
impl Encoding {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" => Some(Self::Utf8),
            "iso-8859-1" | "latin1" | "latin-1" => Some(Self::Latin1),
            "us-ascii" | "ascii" => Some(Self::Ascii),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Latin1 => "ISO-8859-1",
            Self::Ascii => "US-ASCII",
        }
    }
    fn limit(self) -> u32 {
        match self {
            Self::Utf8 => u32::MAX,
            Self::Latin1 => 0xFF,
            Self::Ascii => 0x7F,
        }
    }
    /// Re-express `text` within the encoding; `Err` when a character cannot be written.
    pub fn apply(self, text: &str, references: bool) -> Result<String, Diagnostic> {
        if self == Self::Utf8 {
            return Ok(text.to_owned());
        }
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        let mut cdata = false;
        let mut opaque_end: Option<&str> = None;
        let mut in_markup = false;
        let mut in_doctype = false;
        let mut quote = None;
        while !rest.is_empty() {
            if references {
                if let Some(end) = opaque_end {
                    if rest.starts_with(end) {
                        out.push_str(end);
                        rest = &rest[end.len()..];
                        opaque_end = None;
                        continue;
                    }
                } else if cdata && rest.starts_with("]]>") {
                    out.push_str("]]>");
                    rest = &rest[3..];
                    cdata = false;
                    continue;
                } else if !cdata {
                    if !in_markup && rest.starts_with("<!DOCTYPE") {
                        out.push_str("<!DOCTYPE");
                        rest = &rest[9..];
                        in_markup = true;
                        in_doctype = true;
                        continue;
                    }
                    let opening = if rest.starts_with("<!--") {
                        Some(("<!--", "-->"))
                    } else if rest.starts_with("<?") {
                        Some(("<?", "?>"))
                    } else {
                        None
                    };
                    if let Some((start, end)) = opening {
                        out.push_str(start);
                        rest = &rest[start.len()..];
                        opaque_end = Some(end);
                        continue;
                    }
                    if rest.starts_with("<![CDATA[") {
                        out.push_str("<![CDATA[");
                        rest = &rest[9..];
                        cdata = true;
                        continue;
                    }
                }
            }
            let c = rest.chars().next().unwrap();
            rest = &rest[c.len_utf8()..];
            if references && opaque_end.is_none() && !cdata {
                if let Some(delimiter) = quote {
                    if c == delimiter {
                        quote = None;
                    }
                } else if in_markup && matches!(c, '\'' | '"') {
                    quote = Some(c);
                } else if c == '<' {
                    in_markup = true;
                } else if c == '>' {
                    in_markup = false;
                    in_doctype = false;
                }
            }
            if (c as u32) <= self.limit() {
                out.push(c);
            } else if references
                && opaque_end.is_none()
                && !in_doctype
                && (!in_markup || quote.is_some())
            {
                if cdata {
                    out.push_str("]]>");
                }
                out.push_str(&format!("&#x{:X};", c as u32));
                if cdata {
                    out.push_str("<![CDATA[");
                }
            } else {
                return Err(Diagnostic::new(
                    "SERE0008",
                    format!(
                        "character U+{:04X} cannot be represented in {}",
                        c as u32,
                        self.name()
                    ),
                    "run",
                ));
            }
        }
        Ok(out)
    }
}
/// DOCTYPE declaration parts from `doctype-public` / `doctype-system`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Doctype {
    pub public: Option<String>,
    pub system: Option<String>,
}
/// Compiled serialization settings for one named or unnamed output definition.
#[derive(Debug, Clone, Default)]
pub(crate) struct Serialization {
    pub text_output: bool,
    pub markup: Markup,
    pub encoding: Encoding,
    pub omit_declaration: bool,
    pub omit_specified: bool,
    pub standalone: Option<bool>,
    pub byte_order_mark: bool,
    pub indent: bool,
    pub doctype: Option<Doctype>,
    pub cdata_elements: std::sync::Arc<Vec<String>>,
    pub char_map: std::sync::Arc<BTreeMap<char, String>>,
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
    markup: Markup,
    doctype: Option<Doctype>,
    /// Expanded names of elements whose text is written as CDATA sections.
    cdata: std::sync::Arc<Vec<String>>,
    /// Character maps: mapped characters are written verbatim, unescaped.
    char_map: std::sync::Arc<BTreeMap<char, String>>,
}
impl Output {
    pub fn with_char_map(mut self, map: std::sync::Arc<BTreeMap<char, String>>) -> Self {
        self.char_map = map;
        self
    }
    pub fn char_map(&self) -> std::sync::Arc<BTreeMap<char, String>> {
        self.char_map.clone()
    }
    /// Escape `value`, copying mapped characters through unchanged.
    fn escape_mapped(&self, value: &str, attribute: bool) -> String {
        if self.char_map.is_empty() {
            return escape(value, attribute);
        }
        let mut out = String::new();
        let mut plain = String::new();
        for c in value.chars() {
            if let Some(replacement) = self.char_map.get(&c) {
                out.push_str(&escape(&std::mem::take(&mut plain), attribute));
                out.push_str(replacement);
            } else {
                plain.push(c);
            }
        }
        out.push_str(&escape(&plain, attribute));
        out
    }
    pub fn with_cdata(mut self, cdata: std::sync::Arc<Vec<String>>) -> Self {
        self.cdata = cdata;
        self
    }
    pub fn cdata(&self) -> std::sync::Arc<Vec<String>> {
        self.cdata.clone()
    }
    /// Apply the stylesheet's serialization method and DOCTYPE to the result document.
    pub fn with_serialization(mut self, markup: Markup, doctype: Option<Doctype>) -> Self {
        self.markup = markup;
        self.doctype = doctype;
        self
    }
    pub fn serialization(&self) -> (Markup, Option<Doctype>) {
        (self.markup, self.doctype.clone())
    }
    pub fn new(max: usize, text: bool) -> Self {
        Self {
            value: String::new(),
            max,
            text,
            stack: vec![],
            document: false,
            simple: None,
            adjacent_atomic: false,
            markup: Markup::Xml,
            doctype: None,
            cdata: Default::default(),
            char_map: Default::default(),
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
    /// Append already serialized content (the result of a nested transformation).
    pub fn splice(&mut self, s: &str) -> Result<(), Diagnostic> {
        if s.is_empty() {
            return Ok(());
        }
        if self.simple.is_some() {
            return Err(Diagnostic::new(
                "RUST_XSLT_UNSUPPORTED",
                "xsl:try inside simple content constructors is not implemented",
                "run",
            ));
        }
        self.adjacent_atomic = false;
        self.flush_start()?;
        self.push(s)
    }
    /// Add a namespace binding to the pending start tag (`xsl:namespace`).
    pub fn namespace(&mut self, prefix: &str, uri: &str) -> Result<(), Diagnostic> {
        let Some(open) = self.stack.last_mut().filter(|o| o.pending) else {
            return Err(Diagnostic::new(
                "XTDE0410",
                "namespace created without an open result start tag",
                "run",
            ));
        };
        if open
            .element
            .namespaces
            .get(prefix)
            .is_some_and(|existing| existing != uri)
            && open
                .element
                .name
                .qname
                .split_once(':')
                .map_or("", |(p, _)| p)
                == prefix
        {
            return Err(Diagnostic::new(
                "XTDE0430",
                "conflicting namespace declarations for one prefix",
                "run",
            ));
        }
        open.element
            .namespaces
            .insert(prefix.to_owned(), uri.to_owned());
        Ok(())
    }
    pub fn push(&mut self, s: &str) -> Result<(), Diagnostic> {
        if s.len() > self.max.saturating_sub(self.value.len()) {
            return Err(Diagnostic::new("XPDY0130", "output limit exceeded", "run"));
        }
        self.value.push_str(s);
        Ok(())
    }
    fn flush_start(&mut self) -> Result<(), Diagnostic> {
        self.flush_start_with(">")
    }
    fn flush_start_with(&mut self, close: &str) -> Result<(), Diagnostic> {
        let markup = self.markup;
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
        let html_element = html_name(&e.name, markup);
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
            let minimized = markup == Markup::Html
                && html_element.is_some()
                && name.uri.is_none()
                && BOOLEAN_ATTRIBUTES.contains(&name.local.to_ascii_lowercase().as_str())
                && value.eq_ignore_ascii_case(&name.local);
            if minimized {
                continue;
            }
            self.push("=\"")?;
            if markup == Markup::Html {
                self.push(&value.replace('&', "&amp;").replace('"', "&quot;"))?;
            } else {
                let escaped = self.escape_mapped(value, true);
                self.push(&escaped)?;
            }
            self.push("\"")?;
        }
        self.push(close)
    }
    pub fn start(&mut self, mut element: Element) -> Result<(), Diagnostic> {
        self.flush_start()?;
        self.adjacent_atomic = false;
        if self.stack.is_empty()
            && !self.text
            && self.simple.is_none()
            && let Some(doctype) = self.doctype.take()
        {
            let name = if self.markup == Markup::Html {
                "html".to_owned()
            } else {
                element.name.qname.clone()
            };
            let declaration = match (&doctype.public, &doctype.system) {
                (Some(p), Some(s)) => format!("<!DOCTYPE {name} PUBLIC \"{p}\" \"{s}\">\n"),
                (Some(p), None) if self.markup == Markup::Html => {
                    format!("<!DOCTYPE {name} PUBLIC \"{p}\">\n")
                }
                (None, Some(s)) => format!("<!DOCTYPE {name} SYSTEM \"{s}\">\n"),
                (_, None) => String::new(),
            };
            self.push(&declaration)?;
        }
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
        if !self.text
            && let Some(open) = self.stack.last()
            && open.pending
            && html_name(&open.element.name, self.markup)
                .is_some_and(|name| VOID_ELEMENTS.contains(&name.as_str()))
        {
            // HTML void elements have no end tag; XHTML writes them as `<br />`.
            self.flush_start_with(if self.markup == Markup::Xhtml {
                " />"
            } else {
                ">"
            })?;
            self.stack.pop();
            return Ok(());
        }
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
        let raw = self.markup == Markup::Html
            && self
                .stack
                .last()
                .and_then(|open| html_name(&open.element.name, self.markup))
                .is_some_and(|name| name == "script" || name == "style");
        let in_cdata = self.markup != Markup::Html
            && !self.cdata.is_empty()
            && self.stack.last().is_some_and(|open| {
                let name = &open.element.name;
                let expanded = match name.uri.as_deref() {
                    Some(uri) if !uri.is_empty() => format!("Q{{{uri}}}{}", name.local),
                    _ => name.local.clone(),
                };
                self.cdata.contains(&expanded)
            });
        if in_cdata && !self.text {
            let safe = s.replace("]]>", "]]]]><![CDATA[>");
            self.push(&format!("<![CDATA[{safe}]]>"))
        } else if self.text || raw {
            let map = self.char_map.clone();
            if map.is_empty() {
                return self.push(s);
            }
            for character in s.chars() {
                if let Some(replacement) = map.get(&character) {
                    self.push(replacement)?;
                } else {
                    self.push(character.encode_utf8(&mut [0; 4]))?;
                }
            }
            Ok(())
        } else {
            let escaped = self.escape_mapped(s, false);
            self.push(&escaped)
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
