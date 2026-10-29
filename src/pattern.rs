//! Match patterns use the XPath step model, with reverse ancestry matching.
//! Patterns may use `/` and `//` separators, parenthesized unions and
//! `key()`/`id()` calls.
use crate::{
    Diagnostic,
    xpath::{self, Axis, Budget, Expr, Focus, Item, Step, Test, Variables, XNode},
};
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub(crate) struct Branch {
    pub steps: Vec<Step>,
    /// `gaps[i]` is true when step `i` follows a `//` (any number of intermediate levels).
    pub gaps: Vec<bool>,
    pub absolute: bool,
    pub priority: f64,
    /// Patterns such as `key('k', 'v')` match the nodes an expression selects.
    pub selected: Option<Expr>,
}

struct Parts {
    absolute: bool,
    steps: Vec<Step>,
    gaps: Vec<bool>,
    selected: Option<Expr>,
}

fn unsupported() -> Diagnostic {
    Diagnostic::new(
        "RUST_XSLT_UNSUPPORTED",
        "unsupported match pattern",
        "compile",
    )
}

fn is_descendant_separator(step: &Step) -> bool {
    matches!(step.axis, Axis::DescendantOrSelf)
        && matches!(step.test, Test::Node)
        && step.predicates.is_empty()
}

fn append(parts: &mut Parts, steps: Vec<Step>) -> Result<(), Diagnostic> {
    let mut gap = false;
    for step in steps {
        if is_descendant_separator(&step) {
            gap = true;
            continue;
        }
        if !matches!(step.axis, Axis::Child | Axis::Attribute) {
            return Err(unsupported());
        }
        parts.steps.push(step);
        parts.gaps.push(gap);
        gap = false;
    }
    if gap {
        return Err(unsupported());
    }
    Ok(())
}

fn collect(e: Expr, out: &mut Vec<Parts>) -> Result<(), Diagnostic> {
    match e {
        Expr::Binary(op, a, b) if op == "|" || op == "union" => {
            collect(*a, out)?;
            collect(*b, out)
        }
        Expr::Sequence(mut items) if items.len() == 1 => collect(items.remove(0), out),
        Expr::Root => {
            out.push(Parts {
                absolute: true,
                steps: vec![],
                gaps: vec![],
                selected: None,
            });
            Ok(())
        }
        Expr::Context => {
            out.push(Parts {
                absolute: false,
                steps: vec![],
                gaps: vec![],
                selected: None,
            });
            Ok(())
        }
        Expr::Call(name, args)
            if matches!(
                name.as_str(),
                "key" | "id" | "element-with-id" | "doc" | "document"
            ) =>
        {
            out.push(Parts {
                absolute: false,
                steps: vec![],
                gaps: vec![],
                selected: Some(Expr::Call(name, args)),
            });
            Ok(())
        }
        Expr::Path(base, steps) => {
            let mut bases = vec![];
            collect(*base, &mut bases)?;
            for mut parts in bases {
                append(&mut parts, steps.clone())?;
                out.push(parts);
            }
            Ok(())
        }
        Expr::PathMap(left, right) => {
            let mut lefts = vec![];
            collect(*left, &mut lefts)?;
            let mut rights = vec![];
            collect(*right, &mut rights)?;
            for left in &lefts {
                for right in &rights {
                    if right.absolute || right.selected.is_some() {
                        return Err(unsupported());
                    }
                    let mut parts = Parts {
                        absolute: left.absolute,
                        steps: left.steps.clone(),
                        gaps: left.gaps.clone(),
                        selected: left.selected.clone(),
                    };
                    parts.steps.extend(right.steps.iter().cloned());
                    parts.gaps.extend(right.gaps.iter().copied());
                    out.push(parts);
                }
            }
            Ok(())
        }
        _ => Err(unsupported()),
    }
}

pub(crate) fn parse(
    source: &str,
    namespaces: BTreeMap<String, String>,
) -> Result<Vec<Branch>, Diagnostic> {
    let e = xpath::parse(source, namespaces)?;
    let mut parts = vec![];
    collect(e, &mut parts)?;
    Ok(parts
        .into_iter()
        .map(|p| {
            let priority = if p.selected.is_some()
                || p.steps.len() != 1
                || p.gaps.iter().any(|g| *g)
                || !p.steps[0].predicates.is_empty()
            {
                0.5
            } else if matches!(
                p.steps[0].test,
                Test::Name(_)
                    | Test::Pi(Some(_))
                    | Test::Element(Some(_))
                    | Test::AttributeKind(Some(_))
            ) {
                0.0
            } else if matches!(p.steps[0].test, Test::Namespace(_) | Test::Local(_)) {
                -0.25
            } else {
                -0.5
            };
            Branch {
                steps: p.steps,
                gaps: p.gaps,
                absolute: p.absolute,
                priority,
                selected: p.selected,
            }
        })
        .collect())
}

pub(crate) fn matches(
    branch: &Branch,
    node: XNode<'_>,
    vars: &Variables<'_>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    // Reborrow node and variables with one evaluation lifetime.
    matches_inner(branch, node, vars, budget, backwards)
}

fn matches_inner<'a>(
    branch: &Branch,
    node: XNode<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    if branch.steps.is_empty() {
        return if let Some(selected) = &branch.selected {
            selection_contains(selected, node, vars, budget, backwards)
        } else {
            Ok(node.is_document())
        };
    }
    step_matches(
        branch,
        branch.steps.len() - 1,
        node,
        vars,
        budget,
        backwards,
    )
}

/// Evaluate a selection anchor in the candidate's document and preserve node identity.
fn selection_contains<'a>(
    selected: &Expr,
    node: XNode<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    let focus = Focus {
        item: Item::Node(node),
        position: 1,
        size: 1,
    };
    Ok(xpath::eval(selected, &focus, vars, budget, 0, backwards)?
        .iter()
        .any(|item| matches!(item, Item::Node(n) if *n == node)))
}

/// Whether `node` matches step `index` and the steps before it match its ancestry.
fn step_matches<'a>(
    branch: &Branch,
    index: usize,
    node: XNode<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    budget.tick()?;
    let step = &branch.steps[index];
    if matches!(step.axis, Axis::Attribute) != node.is_attribute() {
        return Ok(false);
    }
    if !xpath::test_node(&step.test, node, step.axis) {
        return Ok(false);
    }
    let parent = node.parent();
    if !step.predicates.is_empty() {
        match parent {
            Some(parent) => {
                let e = Expr::Path(Box::new(Expr::Context), vec![step.clone()]);
                let focus = Focus {
                    item: Item::Node(parent),
                    position: 1,
                    size: 1,
                };
                if !xpath::eval(&e, &focus, vars, budget, 0, backwards)?
                    .iter()
                    .any(|v| matches!(v, Item::Node(n) if *n == node))
                {
                    return Ok(false);
                }
            }
            None => {
                if xpath::filter(
                    vec![Item::Node(node)],
                    &step.predicates,
                    vars,
                    budget,
                    0,
                    backwards,
                )?
                .is_empty()
                {
                    return Ok(false);
                }
            }
        }
    }
    let gap = branch.gaps[index];
    if index == 0 {
        if let Some(selected) = &branch.selected {
            let mut ancestor = parent;
            while let Some(candidate) = ancestor {
                budget.tick()?;
                if selection_contains(selected, candidate, vars, budget, backwards)? {
                    return Ok(true);
                }
                if !gap {
                    break;
                }
                ancestor = candidate.parent();
            }
            return Ok(false);
        }
        return Ok(match (branch.absolute, gap) {
            // `/p` needs the document node as parent; `//p` any node of a document tree.
            (true, false) => parent.is_some_and(|p| p.is_document()),
            (true, true) => node.root().is_document(),
            // Relative patterns have no anchor; a document node only matches document-node().
            (false, _) => {
                parent.is_some() || !node.is_document() || matches!(step.test, Test::Document)
            }
        });
    }
    if gap {
        let mut ancestor = parent;
        while let Some(candidate) = ancestor {
            budget.tick()?;
            if step_matches(branch, index - 1, candidate, vars, budget, backwards)? {
                return Ok(true);
            }
            ancestor = candidate.parent();
        }
        Ok(false)
    } else {
        match parent {
            Some(parent) => step_matches(branch, index - 1, parent, vars, budget, backwards),
            None => Ok(false),
        }
    }
}
