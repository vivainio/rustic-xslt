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
    /// Selection-anchored patterns such as `key()`, `id()` or `$nodes//item`.
    pub selected: Option<Expr>,
    /// A singleton-focus predicate pattern, including bare `.`.
    pub predicate: Option<Vec<Expr>>,
    /// Additional pattern alternatives that must (or must not) match this item.
    pub required: Vec<Vec<Branch>>,
    pub excluded: Vec<Vec<Branch>>,
    /// A slash whose right operand is a general pattern expression.
    pub path_map: Option<(Box<Branch>, Expr)>,
}

#[derive(Clone)]
struct Parts {
    absolute: bool,
    steps: Vec<Step>,
    gaps: Vec<bool>,
    selected: Option<Expr>,
    predicate: Option<Vec<Expr>>,
    required: Vec<Vec<Parts>>,
    excluded: Vec<Vec<Parts>>,
    path_map: Option<(Box<Parts>, Expr)>,
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
        && !step.explicit_axis
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
        if matches!(step.axis, Axis::Descendant | Axis::DescendantOrSelf) {
            if matches!(step.test, Test::AttributeKind(_)) {
                return Err(unsupported());
            }
            gap = true;
        }
        if matches!(step.axis, Axis::SelfNode) {
            parts.steps.push(step);
            parts.gaps.push(gap);
            gap = false;
            continue;
        }
        if !matches!(
            step.axis,
            Axis::Child | Axis::Attribute | Axis::Descendant | Axis::DescendantOrSelf
        ) {
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
            let start = out.len();
            collect(*a, out)?;
            collect(*b, out)?;
            if out[start..].iter().any(|parts| parts.predicate.is_some()) {
                return Err(Diagnostic::new(
                    "XTSE0340",
                    "predicate patterns cannot be union operands",
                    "compile",
                ));
            }
            Ok(())
        }
        Expr::Binary(op, a, b) if op == "intersect" || op == "except" => {
            let mut left = vec![];
            collect(*a, &mut left)?;
            let mut right = vec![];
            collect(*b, &mut right)?;
            if left.is_empty() || right.is_empty() {
                return Err(unsupported());
            }
            for mut part in left {
                if op == "intersect" {
                    part.required.push(right.clone());
                } else {
                    part.excluded.push(right.clone());
                }
                out.push(part);
            }
            Ok(())
        }
        Expr::Sequence(mut items) if items.len() == 1 => collect(items.remove(0), out),
        Expr::Root => {
            out.push(Parts {
                absolute: true,
                steps: vec![],
                gaps: vec![],
                selected: None,
                predicate: None,
                required: vec![],
                excluded: vec![],
                path_map: None,
            });
            Ok(())
        }
        Expr::Context => {
            out.push(Parts {
                absolute: false,
                steps: vec![],
                gaps: vec![],
                selected: None,
                predicate: Some(vec![]),
                required: vec![],
                excluded: vec![],
                path_map: None,
            });
            Ok(())
        }
        Expr::Filter(base, predicates) => {
            let mut parts = vec![];
            collect(*base, &mut parts)?;
            if parts.is_empty() {
                return Err(unsupported());
            }
            for part in &mut parts {
                part.predicate
                    .get_or_insert_with(Vec::new)
                    .extend(predicates.iter().cloned());
            }
            out.extend(parts);
            Ok(())
        }
        Expr::Variable(name) => {
            out.push(Parts {
                absolute: false,
                steps: vec![],
                gaps: vec![],
                selected: Some(Expr::Variable(name)),
                predicate: None,
                required: vec![],
                excluded: vec![],
                path_map: None,
            });
            Ok(())
        }
        Expr::LookupContext(call, namespaces) if matches!(call.as_ref(), Expr::Call(name, _) if matches!(name.as_str(), "doc" | "document")) =>
        {
            out.push(Parts {
                absolute: false,
                steps: vec![],
                gaps: vec![],
                selected: Some(Expr::LookupContext(call, namespaces)),
                predicate: None,
                required: vec![],
                excluded: vec![],
                path_map: None,
            });
            Ok(())
        }
        Expr::Call(name, args)
            if matches!(
                name.as_str(),
                "key" | "id" | "element-with-id" | "doc" | "document" | "root"
            ) =>
        {
            out.push(Parts {
                absolute: false,
                steps: vec![],
                gaps: vec![],
                selected: Some(Expr::Call(name, args)),
                predicate: None,
                required: vec![],
                excluded: vec![],
                path_map: None,
            });
            Ok(())
        }
        Expr::Path(base, steps) => {
            if matches!(base.as_ref(), Expr::Variable(_)) {
                out.push(Parts {
                    absolute: false,
                    steps: vec![],
                    gaps: vec![],
                    selected: Some(Expr::Path(base, steps)),
                    predicate: None,
                    required: vec![],
                    excluded: vec![],
                    path_map: None,
                });
                return Ok(());
            }
            let mut bases = vec![];
            if matches!(*base, Expr::Context) {
                bases.push(Parts {
                    absolute: false,
                    steps: vec![],
                    gaps: vec![],
                    selected: None,
                    predicate: None,
                    required: vec![],
                    excluded: vec![],
                    path_map: None,
                });
            } else {
                collect(*base, &mut bases)?;
            }
            for mut parts in bases {
                if parts.predicate.is_some() {
                    return Err(unsupported());
                }
                append(&mut parts, steps.clone())?;
                out.push(parts);
            }
            Ok(())
        }
        Expr::PathMap(left, right) => {
            let mut lefts = vec![];
            collect(*left, &mut lefts)?;
            for mut left in lefts {
                left.path_map = Some((Box::new(left.clone()), *right.clone()));
                out.push(left);
            }
            Ok(())
        }
        _ => Err(unsupported()),
    }
}

pub(crate) fn parse(
    source: &str,
    namespaces: BTreeMap<String, String>,
    version: u8,
) -> Result<Vec<Branch>, Diagnostic> {
    let trimmed = without_leading_comments(source);
    if trimmed
        .strip_prefix('/')
        .is_some_and(|tail| tail.trim_start().starts_with('['))
    {
        return Err(invalid_pattern("a predicate cannot directly follow `/`"));
    }
    if version < 3 && trimmed.starts_with('(') {
        return Err(invalid_pattern("parenthesized patterns require XSLT 3.0"));
    }
    let e = xpath::parse(source, namespaces)?;
    validate_grouping_functions(&e)?;
    validate_version(&e, source, version)?;
    let mut parts = vec![];
    collect(e, &mut parts)?;
    Ok(parts.into_iter().map(into_branch).collect())
}

// Inspect parsed expressions so strings, comments and unrelated function names
// cannot trigger the restrictions on grouping functions in patterns.
fn validate_grouping_functions(expression: &Expr) -> Result<(), Diagnostic> {
    let mut pending = vec![expression];
    while let Some(e) = pending.pop() {
        if let Expr::Call(name, _) | Expr::FunctionRef(name, _, _) = e {
            let code = match name.as_str() {
                "current-group" => Some("XTSE1060"),
                "current-grouping-key" => Some("XTSE1070"),
                _ => None,
            };
            if let Some(code) = code {
                return Err(Diagnostic::new(
                    code,
                    format!("{name} is not permitted in a pattern"),
                    "compile",
                ));
            }
        }
        match e {
            Expr::For(_, a, b)
            | Expr::Let(_, a, b)
            | Expr::Quantified(_, _, a, b)
            | Expr::Binary(_, a, b)
            | Expr::PathMap(a, b) => {
                pending.extend([a.as_ref(), b.as_ref()]);
            }
            Expr::Path(base, steps) => {
                pending.push(base);
                pending.extend(steps.iter().flat_map(|step| &step.predicates));
            }
            Expr::Filter(base, predicates) | Expr::DynamicCall(base, predicates) => {
                pending.push(base);
                pending.extend(predicates);
            }
            Expr::Unary(_, e)
            | Expr::TypeOp(_, e, _)
            | Expr::QNameCast(e, _, _, _)
            | Expr::StaticName(e, _, _)
            | Expr::LookupContext(e, _)
            | Expr::ArrayCurly(e) => pending.push(e),
            Expr::Call(_, args) | Expr::Sequence(args) | Expr::ArraySquare(args) => {
                pending.extend(args)
            }
            Expr::If(c, y, n) => pending.extend([c.as_ref(), y.as_ref(), n.as_ref()]),
            Expr::InlineFunction(def) => pending.push(&def.body),
            Expr::Partial(callee, args) => {
                pending.push(callee);
                pending.extend(args.iter().flatten());
            }
            Expr::MapConstructor(entries) => {
                for (key, value) in entries {
                    pending.extend([key, value]);
                }
            }
            Expr::Lookup(base, key) => {
                pending.push(base);
                pending.extend(key.iter().map(|e| e.as_ref()));
            }
            Expr::Sorted(base, keys) => {
                pending.push(base);
                for key in keys {
                    pending.extend(key.expressions());
                }
            }
            Expr::Evaluate {
                xpath,
                context,
                namespace_context,
                schema_aware,
                base_uri,
                params,
                with_params,
                ..
            } => {
                pending.extend(with_params.iter().map(|e| e.as_ref()));
                pending.extend(namespace_context.iter().map(|e| e.as_ref()));
                pending.extend(schema_aware.iter().flatten());
                pending.extend(base_uri.iter().flatten());
                pending.push(xpath);
                pending.extend(context.iter().map(|e| e.as_ref()));
                pending.extend(params.iter().map(|(_, e, _)| e));
            }
            Expr::String(_)
            | Expr::Number(_)
            | Expr::Variable(_)
            | Expr::Context
            | Expr::Root
            | Expr::FunctionRef(_, _, _) => {}
        }
    }
    Ok(())
}

fn invalid_pattern(message: &str) -> Diagnostic {
    Diagnostic::new("XTSE0340", message, "compile")
}

fn without_leading_comments(mut source: &str) -> &str {
    loop {
        source = source.trim_start();
        if !source.starts_with("(:") {
            return source;
        }
        let mut depth = 1usize;
        let mut index = 2usize;
        while index < source.len() {
            let rest = &source[index..];
            if rest.starts_with("(:") {
                depth += 1;
                index += 2;
            } else if rest.starts_with(":)") {
                depth -= 1;
                index += 2;
                if depth == 0 {
                    break;
                }
            } else {
                index += source[index..].chars().next().unwrap().len_utf8();
            }
        }
        if depth != 0 {
            return source;
        }
        source = &source[index..];
    }
}

fn validate_version(expression: &Expr, source: &str, version: u8) -> Result<(), Diagnostic> {
    match expression {
        Expr::Filter(_, _) if version < 3 => {
            return Err(invalid_pattern("filtered patterns require XSLT 3.0"));
        }
        Expr::Binary(op, _, _) if matches!(op.as_str(), "intersect" | "except") && version < 3 => {
            return Err(invalid_pattern(
                "intersect and except patterns require XSLT 3.0",
            ));
        }
        Expr::Binary(op, _, _)
            if op == "union" && version < 3 && !source.trim_start().starts_with('/') =>
        {
            return Err(invalid_pattern(
                "the `union` keyword in patterns requires XSLT 3.0",
            ));
        }
        Expr::Call(name, _) if name == "root" && version < 3 => {
            return Err(invalid_pattern("function patterns require XSLT 3.0"));
        }
        _ => {}
    }
    Ok(())
}

fn into_branch(p: Parts) -> Branch {
    let priority = if let Some(predicates) = &p.predicate {
        if predicates.is_empty() { -1.0 } else { 1.0 }
    } else if p.steps.len() == 1 && matches!(p.steps[0].axis, Axis::SelfNode) {
        if !p.steps[0].predicates.is_empty() {
            0.5
        } else if matches!(
            p.steps[0].test,
            Test::Name(_)
                | Test::Pi(Some(_))
                | Test::Element(Some(_))
                | Test::AttributeKind(Some(_))
        ) {
            0.0
        } else {
            -0.5
        }
    } else if p.absolute && p.steps.is_empty() && p.selected.is_none() {
        -0.5
    } else if p.absolute
        || p.selected.is_some()
        || p.steps.len() != 1
        || p.gaps.iter().any(|g| *g)
        || !p.steps[0].predicates.is_empty()
    {
        0.5
    } else if matches!(
        p.steps[0].test,
        Test::Name(_) | Test::Pi(Some(_)) | Test::Element(Some(_)) | Test::AttributeKind(Some(_))
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
        predicate: p.predicate,
        required: p
            .required
            .into_iter()
            .map(|group| group.into_iter().map(into_branch).collect())
            .collect(),
        excluded: p
            .excluded
            .into_iter()
            .map(|group| group.into_iter().map(into_branch).collect())
            .collect(),
        path_map: p
            .path_map
            .map(|(prefix, expression)| (Box::new(into_branch(*prefix)), expression)),
    }
}

pub(crate) fn matches(
    branch: &Branch,
    node: XNode<'_>,
    vars: &Variables<'_>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    // Reborrow node and variables with one evaluation lifetime.
    matches_item(branch, &Item::Node(node), vars, budget, backwards)
}

pub(crate) fn matches_item<'a>(
    branch: &Branch,
    item: &Item<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    budget.tick()?;
    let base_matches = match item {
        Item::Node(node) if branch.path_map.is_some() => {
            let (prefix, expression) = branch.path_map.as_ref().unwrap();
            path_map_contains(prefix, expression, *node, vars, budget, backwards)?
        }
        Item::Node(node) if branch.selected.is_some() && branch.steps.is_empty() => {
            let mut scope = vars.clone();
            scope.insert(xpath::CURRENT_ITEM.into(), vec![Item::Node(*node)]);
            selection_contains(
                branch.selected.as_ref().unwrap(),
                *node,
                &scope,
                budget,
                backwards,
            )?
        }
        Item::Node(node)
            if branch.absolute || !branch.steps.is_empty() || branch.selected.is_some() =>
        {
            matches_inner(branch, *node, vars, budget, backwards)?
        }
        Item::Node(node) if branch.predicate.is_none() => node.is_document(),
        _ if branch.selected.is_some() => false,
        _ => branch.predicate.is_some(),
    };
    if !base_matches {
        return Ok(false);
    }
    if let Some(predicates) = &branch.predicate {
        let mut scope = vars.clone();
        scope.insert(xpath::CURRENT_ITEM.into(), vec![item.clone()]);
        if xpath::filter(vec![item.clone()], predicates, &scope, budget, 0, backwards)?.is_empty() {
            return Ok(false);
        }
    }
    for group in &branch.required {
        let mut found = false;
        for candidate in group {
            if matches_item(candidate, item, vars, budget, backwards)? {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    for group in &branch.excluded {
        for candidate in group {
            if matches_item(candidate, item, vars, budget, backwards)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(crate) fn expressions(branch: &Branch) -> std::vec::IntoIter<&Expr> {
    fn collect<'a>(branch: &'a Branch, expressions: &mut Vec<&'a Expr>) {
        expressions.extend(branch.selected.iter());
        expressions.extend(branch.predicate.iter().flatten());
        expressions.extend(branch.steps.iter().flat_map(|step| &step.predicates));
        if let Some((prefix, expression)) = &branch.path_map {
            expressions.push(expression);
            collect(prefix, expressions);
        }
        for candidate in branch.required.iter().flatten() {
            collect(candidate, expressions);
        }
        for candidate in branch.excluded.iter().flatten() {
            collect(candidate, expressions);
        }
    }
    let mut result = vec![];
    collect(branch, &mut result);
    result.into_iter()
}

fn path_map_contains<'a>(
    prefix: &Branch,
    expression: &Expr,
    candidate: XNode<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    let mut context = Some(candidate);
    while let Some(node) = context {
        budget.tick()?;
        if matches_item(prefix, &Item::Node(node), vars, budget, backwards)? {
            let focus = Focus {
                item: Item::Node(node),
                position: 1,
                size: 1,
            };
            if xpath::eval(expression, &focus, vars, budget, 1, backwards)?
                .iter()
                .any(|item| matches!(item, Item::Node(found) if *found == candidate))
            {
                return Ok(true);
            }
        }
        context = node.parent();
    }
    Ok(false)
}

fn matches_inner<'a>(
    branch: &Branch,
    node: XNode<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    let mut current_scope = vars.clone();
    current_scope.insert(xpath::CURRENT_ITEM.into(), vec![Item::Node(node)]);
    let vars = &current_scope;
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
    Ok(xpath::eval(selected, &focus, vars, budget, 1, backwards)?
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
    if step.explicit_axis && matches!(step.axis, Axis::Child) && node.is_document() {
        return Ok(false);
    }
    if matches!(step.axis, Axis::Descendant | Axis::DescendantOrSelf) {
        if index == 0 {
            if branch.absolute && !node.root().is_document() {
                return Ok(false);
            }
            if let Some(selected) = &branch.selected {
                let expression = Expr::Path(Box::new(selected.clone()), vec![step.clone()]);
                let focus = Focus {
                    item: Item::Node(node),
                    position: 1,
                    size: 1,
                };
                return Ok(
                    xpath::eval(&expression, &focus, vars, budget, 1, backwards)?
                        .iter()
                        .any(|item| matches!(item, Item::Node(candidate) if *candidate == node)),
                );
            }
            return axis_contains(
                step,
                node.parent().unwrap_or_else(|| node.root()),
                node,
                vars,
                budget,
                backwards,
            );
        }
        let mut context = Some(node);
        while let Some(candidate) = context {
            budget.tick()?;
            if step_matches(branch, index - 1, candidate, vars, budget, backwards)?
                && axis_contains(step, candidate, node, vars, budget, backwards)?
            {
                return Ok(true);
            }
            context = candidate.parent();
        }
        return Ok(false);
    }
    if !matches!(step.axis, Axis::SelfNode)
        && matches!(step.axis, Axis::Attribute) != node.is_attribute()
    {
        return Ok(false);
    }
    if !xpath::test_node(&step.test, node, step.axis) {
        return Ok(false);
    }
    let parent = node.parent();
    if !step.predicates.is_empty() {
        match if matches!(step.axis, Axis::SelfNode) {
            Some(node)
        } else {
            parent
        } {
            Some(parent) => {
                let e = Expr::Path(Box::new(Expr::Context), vec![step.clone()]);
                let focus = Focus {
                    item: Item::Node(parent),
                    position: 1,
                    size: 1,
                };
                if !xpath::eval(&e, &focus, vars, budget, 1, backwards)?
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
    if matches!(step.axis, Axis::SelfNode) {
        if index == 0 {
            if let Some(selected) = &branch.selected {
                return selection_contains(selected, node, vars, budget, backwards);
            }
            return Ok(if branch.absolute {
                node.is_document()
            } else {
                true
            });
        }
        if branch.gaps[index] {
            let mut candidate = Some(node);
            while let Some(ancestor) = candidate {
                budget.tick()?;
                if step_matches(branch, index - 1, ancestor, vars, budget, backwards)? {
                    return Ok(true);
                }
                candidate = ancestor.parent();
            }
            return Ok(false);
        }
        return step_matches(branch, index - 1, node, vars, budget, backwards);
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

fn axis_contains<'a>(
    step: &Step,
    context: XNode<'a>,
    candidate: XNode<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    let expression = Expr::Path(Box::new(Expr::Context), vec![step.clone()]);
    let focus = Focus {
        item: Item::Node(context),
        position: 1,
        size: 1,
    };
    Ok(
        xpath::eval(&expression, &focus, vars, budget, 1, backwards)?
            .iter()
            .any(|item| matches!(item, Item::Node(node) if *node == candidate)),
    )
}
