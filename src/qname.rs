//! Canonical expanded component names; no default namespace applies.
use crate::Diagnostic;
use std::collections::BTreeMap;

fn expanded(uri: &str, local: &str) -> String {
    if uri.is_empty() {
        local.into()
    } else {
        format!("Q{{{uri}}}{local}")
    }
}
pub(crate) fn resolve(
    text: &str,
    namespaces: &BTreeMap<String, String>,
    code: &str,
) -> Result<String, Diagnostic> {
    let invalid = || Diagnostic::new(code, format!("invalid QName: {text}"), "compile");
    if text.starts_with("Q{") {
        let name = external(text).map_err(|_| invalid())?;
        if code == "XPST0003" && parts(&name).0 == Some("http://www.w3.org/2000/xmlns/") {
            return Err(Diagnostic::new(
                "XQST0070",
                "xmlns namespace is forbidden in XPath EQNames",
                "compile",
            ));
        }
        return Ok(name);
    }
    if let Some((prefix, local)) = text.split_once(':') {
        if !xml_ncname(prefix) || !xml_ncname(local) {
            return Err(invalid());
        }
        let uri = namespaces.get(prefix).ok_or_else(|| {
            Diagnostic::new(
                if code == "XPST0003" {
                    "XPST0081"
                } else {
                    "XTSE0280"
                },
                format!("unbound prefix: {prefix}"),
                "compile",
            )
        })?;
        Ok(expanded(uri, local))
    } else if xml_ncname(text) {
        Ok(text.into())
    } else {
        Err(invalid())
    }
}
/// API names have no prefix context. Accept local names and URIQualifiedNames.
pub(crate) fn external(text: &str) -> Result<String, Diagnostic> {
    if let Some(rest) = text.strip_prefix("Q{") {
        if let Some((uri, local)) = rest.split_once('}')
            && !uri.contains('{')
            && xml_ncname(local)
        {
            return Ok(expanded(&normalize_uri(uri), local));
        }
    } else if xml_ncname(text) {
        return Ok(text.into());
    }
    Err(Diagnostic::new(
        "RUST_XSLT_INVALID_NAME",
        format!("expected a local name or Q{{uri}}local: {text}"),
        "run",
    ))
}

/// Split a canonical expanded name produced by this module.
pub(crate) fn parts(name: &str) -> (Option<&str>, &str) {
    if let Some(rest) = name.strip_prefix("Q{")
        && let Some((uri, local)) = rest.split_once('}')
    {
        (Some(uri), local)
    } else {
        (None, name)
    }
}

/// Braced URI literals apply XML whitespace collapsing.
pub(crate) fn normalize_uri(uri: &str) -> String {
    uri.split([' ', '\t', '\r', '\n'])
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// An atomic QName retains its lexical prefix, but equality uses only uri/local.
#[derive(Debug, Clone)]
pub(crate) struct AtomicQName {
    pub uri: Option<String>,
    pub local: String,
    pub prefix: Option<String>,
}
pub(crate) fn xml_ncname_start(c: char) -> bool {
    matches!(c, 'A'..='Z' | '_' | 'a'..='z' | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}' | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
}
pub(crate) fn xml_ncname_char(c: char) -> bool {
    xml_ncname_start(c)
        || matches!(c, '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}
pub(crate) fn xml_ncname(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(xml_ncname_start) && chars.all(xml_ncname_char)
}

impl AtomicQName {
    pub fn new(uri: &str, lexical: &str) -> Result<Self, Diagnostic> {
        let (prefix, local) = lexical
            .split_once(':')
            .map_or((None, lexical), |(p, l)| (Some(p), l));
        if !xml_ncname(local) || prefix.is_some_and(|p| !xml_ncname(p) || uri.is_empty()) {
            return Err(Diagnostic::new(
                "FOCA0002",
                "invalid lexical QName or absent prefixed namespace",
                "run",
            ));
        }
        Ok(Self {
            uri: if uri.is_empty() {
                None
            } else {
                Some(uri.into())
            },
            local: local.into(),
            prefix: prefix.map(str::to_owned),
        })
    }
    pub fn cast(text: &str, namespaces: &BTreeMap<String, String>) -> Result<Self, Diagnostic> {
        let lexical = text.trim_matches([' ', '\t', '\n', '\r']);
        let (prefix, local) = lexical
            .split_once(':')
            .map_or((None, lexical), |(p, l)| (Some(p), l));
        if !xml_ncname(local) || prefix.is_some_and(|p| !xml_ncname(p)) {
            return Err(Diagnostic::new("FORG0001", "invalid lexical QName", "run"));
        }
        let uri = if let Some(prefix) = prefix {
            namespaces
                .get(prefix)
                .filter(|uri| !uri.is_empty())
                .ok_or_else(|| {
                    Diagnostic::new("FONS0004", format!("unbound QName prefix: {prefix}"), "run")
                })?
                .as_str()
        } else {
            namespaces.get("").map_or("", String::as_str)
        };
        Self::new(uri, lexical)
    }
    pub fn lexical(&self) -> String {
        self.prefix
            .as_ref()
            .map_or_else(|| self.local.clone(), |p| format!("{p}:{}", self.local))
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.uri == other.uri && self.local == other.local
    }
    pub fn error_code(&self) -> String {
        if self.uri.as_deref() == Some("http://www.w3.org/2005/xqt-errors") {
            self.local.clone()
        } else {
            format!(
                "Q{{{}}}{}",
                self.uri.as_deref().unwrap_or_default(),
                self.local
            )
        }
    }
}
