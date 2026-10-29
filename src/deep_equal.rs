//! Iterative deep equality for supported atomic values and untyped XDM nodes.
use crate::{
    Diagnostic,
    xpath::{Budget, Item, Value, XNode, atomize},
};
use std::{cmp::Ordering, collections::BTreeMap};
fn kind(n: XNode<'_>) -> u8 {
    if n.is_attribute() {
        0
    } else if n.is_document() {
        1
    } else if n.is_element() {
        2
    } else if n.is_text() {
        3
    } else if n.is_comment() {
        4
    } else {
        5
    }
}
pub(crate) fn evaluate(args: &[Value<'_>], budget: &mut Budget) -> Result<bool, Diagnostic> {
    if let Some(collation) = args.get(2) {
        let [item] = collation.as_slice() else {
            return Err(Diagnostic::new(
                "XPTY0004",
                "collation requires a singleton string",
                "run",
            ));
        };
        let item = atomize(item);
        if !matches!(item, Item::String(_) | Item::Untyped(_)) {
            return Err(Diagnostic::new(
                "XPTY0004",
                "collation requires a string",
                "run",
            ));
        }
        if item.string() != "http://www.w3.org/2005/xpath-functions/collation/codepoint" {
            return Err(Diagnostic::new("FOCH0002", "unsupported collation", "run"));
        }
    }
    if args[0].len() != args[1].len() {
        return Ok(false);
    }
    let mut pending = vec![];
    for (a, b) in args[0].iter().zip(&args[1]) {
        budget.tick()?;
        let equal = match (a, b) {
            (Item::Node(a), Item::Node(b)) => {
                pending.push((*a, *b));
                true
            }
            (Item::Number(a), Item::Number(b)) => {
                a.compare(b) == Some(Ordering::Equal)
                    || a.compare(a).is_none() && b.compare(b).is_none()
            }
            (Item::String(a) | Item::Untyped(a), Item::String(b) | Item::Untyped(b)) => a == b,
            (Item::Boolean(a), Item::Boolean(b)) => a == b,
            (Item::QName(a), Item::QName(b)) => a.equal(b),
            _ => false,
        };
        if !equal {
            return Ok(false);
        }
    }
    while let Some((a, b)) = pending.pop() {
        budget.tick()?;
        if kind(a) != kind(b) || a.name() != b.name() {
            return Ok(false);
        }
        if a == b {
            continue;
        }
        if a.is_element() {
            let mut attributes = BTreeMap::new();
            for attribute in a.attributes() {
                budget.tick()?;
                attributes.insert(attribute.name(), attribute.string());
            }
            let other = b.attributes();
            if attributes.len() != other.len() {
                return Ok(false);
            }
            for attribute in other {
                budget.tick()?;
                if attributes.get(&attribute.name()) != Some(&attribute.string()) {
                    return Ok(false);
                }
            }
        }
        if a.is_document() || a.is_element() {
            let content = |n: &XNode<'_>| !n.is_comment() && !n.is_pi();
            let mut left = a.children().filter(content);
            let mut right = b.children().filter(content);
            loop {
                match (left.next(), right.next()) {
                    (None, None) => break,
                    (Some(a), Some(b)) => {
                        budget.tick()?;
                        pending.push((a, b));
                    }
                    _ => return Ok(false),
                }
            }
        } else if a.string() != b.string() {
            return Ok(false);
        }
    }
    Ok(true)
}
