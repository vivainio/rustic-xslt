//! QName resolution and namespace binding inspection on XDM elements.
use crate::{
    Diagnostic,
    qname::AtomicQName,
    xpath::{Budget, Item, Value, XNode},
};
use std::collections::{BTreeMap, BTreeSet};
fn element<'a>(value: &[Item<'a>]) -> Result<XNode<'a>, Diagnostic> {
    match value {
        [Item::Node(node)] if node.is_element() => Ok(*node),
        _ => Err(Diagnostic::new(
            "XPTY0004",
            "namespace function requires a singleton element",
            "run",
        )),
    }
}
pub(crate) fn evaluate<'a>(
    name: &str,
    args: &[Value<'a>],
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    if name == "in-scope-prefixes" {
        let node = element(&args[0])?;
        let mut prefixes = BTreeSet::from(["xml".to_owned()]);
        for ns in node.raw.namespaces() {
            budget.tick()?;
            if !ns.uri().is_empty() {
                prefixes.insert(ns.name().unwrap_or("").to_owned());
            }
        }
        return Ok(prefixes.into_iter().map(Item::String).collect());
    }
    let lexical = crate::string_functions::string(&args[0], true, false)?;
    let node = element(&args[1])?;
    for _ in lexical.bytes() {
        budget.tick()?;
    }
    if name == "namespace-uri-for-prefix" {
        let uri = if lexical == "xml" {
            Some("http://www.w3.org/XML/1998/namespace")
        } else {
            node.raw.lookup_namespace_uri(if lexical.is_empty() {
                None
            } else {
                Some(&lexical)
            })
        };
        return Ok(uri
            .filter(|uri| !uri.is_empty())
            .map_or_else(Vec::new, |uri| vec![Item::AnyUri(uri.into())]));
    }
    if args[0].is_empty() {
        return Ok(vec![]);
    }
    let mut namespaces = BTreeMap::from([(
        "xml".to_owned(),
        "http://www.w3.org/XML/1998/namespace".to_owned(),
    )]);
    for ns in node.raw.namespaces() {
        budget.tick()?;
        namespaces.insert(ns.name().unwrap_or("").into(), ns.uri().into());
    }
    let name = AtomicQName::cast(&lexical, &namespaces).map_err(|mut error| {
        if error.code == "FORG0001" {
            error.code = "FOCA0002".into();
        }
        error
    })?;
    Ok(vec![Item::QName(name)])
}
