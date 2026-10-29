//! Iterative deep equality for supported atomic values and untyped XDM nodes.
use crate::collation::Collation;
use crate::{
    Diagnostic, Limits,
    xpath::{Budget, Item, Value, XNode, atomize},
};
use std::{cell::Cell, cmp::Ordering, collections::BTreeMap};
thread_local! {
    static NESTING: Cell<usize> = const { Cell::new(0) };
}
/// Equality of map values and array members; nesting is bounded so that
/// hostile deeply nested containers cannot exhaust the stack.
fn members_equal(a: &Value<'_>, b: &Value<'_>) -> bool {
    NESTING.with(|depth| {
        if depth.get() >= 64 {
            return false;
        }
        depth.set(depth.get() + 1);
        let result =
            evaluate(&[a.clone(), b.clone()], &mut Budget::new(Limits::default())).unwrap_or(false);
        depth.set(depth.get() - 1);
        result
    })
}
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
/// Equality for atomized values, without untyped-to-numeric conversion.
pub(crate) fn atomic_equal(a: &Item<'_>, b: &Item<'_>, nan_equal: bool) -> bool {
    atomic_equal_with(a, b, nan_equal, Collation::Codepoint)
}
/// Equality that compares strings under `collation`.
pub(crate) fn atomic_equal_with(
    a: &Item<'_>,
    b: &Item<'_>,
    nan_equal: bool,
    collation: Collation,
) -> bool {
    match (a, b) {
        (Item::Number(a), Item::Number(b)) => {
            a.compare(b) == Some(Ordering::Equal)
                || nan_equal && a.compare(a).is_none() && b.compare(b).is_none()
        }
        (
            Item::String(a) | Item::Untyped(a) | Item::AnyUri(a) | Item::DerivedString(_, a),
            Item::String(b) | Item::Untyped(b) | Item::AnyUri(b) | Item::DerivedString(_, b),
        ) => collation.equal(a, b),
        (Item::Boolean(a), Item::Boolean(b)) => a == b,
        (Item::QName(a), Item::QName(b)) => a.equal(b),
        (Item::Binary(ak, a), Item::Binary(bk, b)) => ak == bk && a == b,
        (Item::Temporal(a), Item::Temporal(b)) => a.compare(b) == Some(Ordering::Equal),
        (Item::Map(a), Item::Map(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|other| members_equal(v, other)))
        }
        (Item::Array(a), Item::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| members_equal(x, y))
        }
        _ => false,
    }
}
pub(crate) fn evaluate(args: &[Value<'_>], budget: &mut Budget) -> Result<bool, Diagnostic> {
    let mut collation_used = Collation::Codepoint;
    if let Some(collation) = args.get(2) {
        let [item] = collation.as_slice() else {
            return Err(Diagnostic::new(
                "XPTY0004",
                "collation requires a singleton string",
                "run",
            ));
        };
        let item = atomize(item);
        if !matches!(
            item,
            Item::String(_) | Item::Untyped(_) | Item::AnyUri(_) | Item::DerivedString(_, _)
        ) {
            return Err(Diagnostic::new(
                "XPTY0004",
                "collation requires a string",
                "run",
            ));
        }
        collation_used = Collation::parse(&item.string())?;
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
            _ => atomic_equal_with(a, b, true, collation_used),
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
