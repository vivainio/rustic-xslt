//! Match patterns use the XPath step model, with reverse ancestry matching.
use crate::{
    Diagnostic,
    xpath::{self, Axis, Budget, Expr, Focus, Item, Step, Test, Variables, XNode},
};
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub(crate) struct Branch {
    pub steps: Vec<Step>,
    pub absolute: bool,
    pub priority: f64,
}
pub(crate) fn parse(
    source: &str,
    namespaces: BTreeMap<String, String>,
) -> Result<Vec<Branch>, Diagnostic> {
    let e = xpath::parse(source, namespaces)?;
    let mut pending = vec![e];
    let mut branches = vec![];
    while let Some(e) = pending.pop() {
        match e {
            Expr::Binary(op, a, b) if op == "|" => {
                pending.push(*b);
                pending.push(*a);
            }
            Expr::Root => branches.push(Branch {
                steps: vec![],
                absolute: true,
                priority: 0.5,
            }),
            Expr::Path(base, steps)
                if matches!(*base, Expr::Root | Expr::Context)
                    && steps
                        .iter()
                        .all(|s| matches!(s.axis, Axis::Child | Axis::Attribute)) =>
            {
                let priority = if steps.len() != 1 || !steps[0].predicates.is_empty() {
                    0.5
                } else if matches!(
                    steps[0].test,
                    Test::Name(_)
                        | Test::Pi(Some(_))
                        | Test::Element(Some(_))
                        | Test::AttributeKind(Some(_))
                ) {
                    0.0
                } else if matches!(steps[0].test, Test::Namespace(_) | Test::Local(_)) {
                    -0.25
                } else {
                    -0.5
                };
                branches.push(Branch {
                    steps,
                    absolute: matches!(*base, Expr::Root),
                    priority,
                });
            }
            _ => {
                return Err(Diagnostic::new(
                    "RUST_XSLT_UNSUPPORTED",
                    "unsupported match pattern",
                    "compile",
                ));
            }
        }
    }
    Ok(branches)
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
        return Ok(matches!(node, XNode::Tree(n) if n.is_root()));
    }
    let mut current = node;
    for step in branch.steps.iter().rev() {
        budget.tick()?;
        let Some(parent) = current.parent() else {
            return Ok(false);
        };
        if matches!(step.axis, Axis::Attribute) != matches!(current, XNode::Attribute(..)) {
            return Ok(false);
        }
        if !xpath::test_node(&step.test, current, step.axis) {
            return Ok(false);
        }
        if !step.predicates.is_empty() {
            let e = Expr::Path(Box::new(Expr::Context), vec![step.clone()]);
            let focus = Focus {
                item: Item::Node(parent),
                position: 1,
                size: 1,
            };
            if !xpath::eval(&e, &focus, vars, budget, 0, backwards)?
                .iter()
                .any(|v| matches!(v, Item::Node(n) if *n == current))
            {
                return Ok(false);
            }
        }
        current = parent;
    }
    Ok(!branch.absolute || matches!(current, XNode::Tree(n) if n.is_root()))
}
