//! Canonical expanded component names; no default namespace applies.
use crate::Diagnostic;
use std::collections::BTreeMap;

fn ncname(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some('a'..='z' | 'A'..='Z' | '_'))
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}
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
    if let Some((prefix, local)) = text.split_once(':') {
        if !ncname(prefix) || !ncname(local) {
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
    } else if ncname(text) {
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
            && ncname(local)
        {
            return Ok(expanded(uri, local));
        }
    } else if ncname(text) {
        return Ok(text.into());
    }
    Err(Diagnostic::new(
        "RUST_XSLT_INVALID_NAME",
        format!("expected a local name or Q{{uri}}local: {text}"),
        "run",
    ))
}
