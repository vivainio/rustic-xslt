use crate::{
    Diagnostic, Stylesheet, TransformOptions,
    compile::{Binding, BindingValue, ComputedName, Content, Grouping, Instruction, Template},
    result::{self, Output},
    xpath::{self, Budget, Focus, Item, Value, Variables, XNode},
};
use std::rc::Rc;
thread_local! {
    static TRY_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
/// Split an error code into its namespace and local part.
fn error_name(code: &str) -> (String, String) {
    match code
        .strip_prefix("Q{")
        .and_then(|rest| rest.split_once('}'))
    {
        Some((uri, local)) => (uri.to_owned(), local.to_owned()),
        None => (crate::compile::ERROR_NAMESPACE.to_owned(), code.to_owned()),
    }
}
fn catch_matches(catch: &crate::compile::Catch, error: &Diagnostic) -> bool {
    let (uri, local) = error_name(&error.code);
    catch.errors.iter().any(|(test_uri, test_local)| {
        test_uri.as_ref().is_none_or(|u| *u == uri)
            && test_local.as_ref().is_none_or(|l| *l == local)
    })
}
fn limit() -> Diagnostic {
    Diagnostic::new("XPDY0130", "execution depth limit exceeded", "run")
}
#[derive(Clone, Copy, PartialEq)]
struct Rule {
    precedence: u32,
    template: usize,
    branch: usize,
    priority: f64,
}
impl Rule {
    fn compare(self, other: Self) -> std::cmp::Ordering {
        self.precedence.cmp(&other.precedence).then(
            self.priority
                .partial_cmp(&other.priority)
                .expect("finite or infinite template priorities")
                .then((self.template, self.branch).cmp(&(other.template, other.branch))),
        )
    }
    /// Ordering on (import precedence, priority) alone.
    fn rank(self, other: Self) -> std::cmp::Ordering {
        self.precedence.cmp(&other.precedence).then(
            self.priority
                .partial_cmp(&other.priority)
                .expect("comparable priorities"),
        )
    }
}
#[derive(Clone)]
struct Frame<'a> {
    focus: Focus<'a>,
    env: Rc<Variables<'a>>,
    depth: usize,
    mode: String,
    tunnel: Rc<Variables<'a>>,
    rule: Option<Rule>,
}
#[derive(Clone, Default)]
struct Supplied<'a> {
    ordinary: Variables<'a>,
    tunnel: Rc<Variables<'a>>,
}
impl<'a> Supplied<'a> {
    fn inherited(tunnel: Rc<Variables<'a>>) -> Self {
        Self {
            ordinary: Variables::new(),
            tunnel,
        }
    }
    fn insert(&mut self, binding: &Binding, value: Value<'a>) {
        if binding.tunnel {
            Rc::make_mut(&mut self.tunnel).insert(binding.name.clone(), value);
        } else {
            self.ordinary.insert(binding.name.clone(), value);
        }
    }
}
enum Task<'a> {
    Apply(Focus<'a>, usize, String, Rc<Supplied<'a>>, Option<Rule>),
    ApplyArguments(
        Vec<Focus<'a>>,
        String,
        &'a [Binding],
        usize,
        Supplied<'a>,
        Frame<'a>,
        Option<Rule>,
    ),
    Body(&'a [Instruction], usize, Frame<'a>),
    Enter(
        &'a Template,
        Focus<'a>,
        Supplied<'a>,
        usize,
        String,
        Option<Rule>,
    ),
    RawCopy(Item<'a>, usize, bool),
    DeepCopy(XNode<'a>, usize, bool),
    FinishDocument(Output, usize),
    FinishTemplate(&'a crate::seqtype::SequenceType, Output, usize, bool, bool),
    FinishRawTemplate(&'a crate::seqtype::SequenceType, Output, usize),
    FinishRawNode(Output, usize, bool),
    FinishRawAttribute(Output, usize),
    Value(Item<'a>, usize, bool),
    Close,
    FinishCapture(Capture, Output),
    Parameters(&'a Template, usize, Variables<'a>, Frame<'a>),
    Arguments(&'a Template, &'a [Binding], usize, Supplied<'a>, Frame<'a>),
    EvaluateBinding(&'a Binding, Frame<'a>, BindNext<'a>),
    FinishBinding(&'a Binding, Frame<'a>, BindNext<'a>, Output, bool),
    FinishRawBinding(&'a Binding, Frame<'a>, BindNext<'a>, Output),
    Iterate(
        &'a Instruction,
        Rc<Value<'a>>,
        usize,
        Variables<'a>,
        Frame<'a>,
    ),
    IterateNext(
        &'a Instruction,
        Rc<Value<'a>>,
        usize,
        Variables<'a>,
        Frame<'a>,
    ),
}
/// The tail position of an `xsl:iterate` body reports back through this record.
struct IterationState<'a> {
    parameters: &'a [Binding],
    next: Option<Variables<'a>>,
    broken: bool,
}
enum BindNext<'a> {
    Local(&'a [Instruction], usize),
    Parameter(&'a Template, usize, Variables<'a>),
    Argument(&'a Template, &'a [Binding], usize, Supplied<'a>),
    ApplyArgument(
        Vec<Focus<'a>>,
        String,
        &'a [Binding],
        usize,
        Supplied<'a>,
        Option<Rule>,
    ),
}
impl BindNext<'_> {
    fn type_error(&self) -> &'static str {
        match self {
            Self::Parameter(..) => "XTTE0600",
            Self::Argument(..) | Self::ApplyArgument(..) => "XTTE0590",
            Self::Local(..) => "XTTE0570",
        }
    }
}
fn complete_binding<'a>(
    binding: &Binding,
    mut value: Value<'a>,
    mut frame: Frame<'a>,
    next: BindNext<'a>,
    tasks: &mut Vec<Task<'a>>,
) -> Result<(), Diagnostic> {
    if let Some(ty) = &binding.ty {
        value = ty.convert(value, next.type_error())?;
    }
    match next {
        BindNext::Local(body, index) => {
            Rc::make_mut(&mut frame.env).insert(binding.name.clone(), value);
            tasks.push(Task::Body(body, index, frame));
        }
        BindNext::Parameter(t, index, supplied) => {
            Rc::make_mut(&mut frame.env).insert(binding.name.clone(), value);
            tasks.push(Task::Parameters(t, index, supplied, frame));
        }
        BindNext::ApplyArgument(foci, mode, params, index, mut args, before) => {
            args.insert(binding, value);
            tasks.push(Task::ApplyArguments(
                foci, mode, params, index, args, frame, before,
            ));
        }
        BindNext::Argument(t, params, index, mut args) => {
            args.insert(binding, value);
            tasks.push(Task::Arguments(t, params, index, args, frame));
        }
    }
    Ok(())
}
fn emit_value<'a>(value: Value<'a>, tasks: &mut Vec<Task<'a>>, depth: usize, namespaces: bool) {
    for item in value.into_iter().rev() {
        tasks.push(Task::Value(item, depth, namespaces));
    }
}
struct RawCapture<'a> {
    items: Value<'a>,
    active: bool,
}
fn raw_active(captures: &[RawCapture<'_>]) -> bool {
    captures.last().is_some_and(|c| c.active)
}
fn suspend_raw(captures: &mut [RawCapture<'_>]) -> bool {
    let active = raw_active(captures);
    if let Some(capture) = captures.last_mut() {
        capture.active = false;
    }
    active
}
fn restore_raw(captures: &mut [RawCapture<'_>], active: bool) {
    if let Some(capture) = captures.last_mut() {
        capture.active = active;
    }
}
fn begin_raw_node<'a>(
    output: &mut Output,
    captures: &mut [RawCapture<'a>],
    tasks: &mut Vec<Task<'a>>,
    depth: usize,
    document: bool,
) {
    let saved = std::mem::replace(output, Output::document(output.max));
    suspend_raw(captures);
    tasks.push(Task::FinishRawNode(saved, depth, document));
}
fn begin_raw_attribute<'a>(
    output: &mut Output,
    captures: &mut [RawCapture<'a>],
    tasks: &mut Vec<Task<'a>>,
    depth: usize,
) -> Result<(), Diagnostic> {
    let saved = std::mem::replace(output, Output::document(output.max));
    suspend_raw(captures);
    tasks.push(Task::FinishRawAttribute(saved, depth));
    output.start(result::Element {
        name: result::Name {
            qname: "_attribute".into(),
            local: "_attribute".into(),
            uri: None,
        },
        namespaces: Default::default(),
        attributes: vec![],
        inherit_namespaces: true,
    })
}
fn raw_text<'a>(
    text: &str,
    trees: &crate::trees::Trees<'a>,
    max: usize,
) -> Result<Value<'a>, Diagnostic> {
    let mut output = Output::document(max);
    output.text(if text.is_empty() { " " } else { text })?;
    let value = trees.construct(
        output.value,
        Some(&crate::seqtype::SequenceType {
            kind: crate::seqtype::Kind::Item,
            min: 0,
            max: usize::MAX,
        }),
        "XTTE0570",
    )?;
    Ok(value
        .into_iter()
        .map(|item| match item {
            Item::Node(node) if text.is_empty() => Item::Node(node.empty_text()),
            other => other,
        })
        .collect())
}
struct Run<'a, 'r> {
    sheet: &'a Stylesheet,
    options: &'r TransformOptions,
    trees: &'r crate::trees::Trees<'a>,
    globals: Rc<Variables<'a>>,
}
enum Capture {
    Namespace(String),
    Attribute(result::Name),
    Comment,
    Message(bool),
    Pi(String),
}
fn parts(
    parts: &[xpath::Expr],
    focus: &Focus<'_>,
    env: &Variables<'_>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<String, Diagnostic> {
    let mut text = String::new();
    for e in parts {
        let mut v = xpath::eval(e, focus, env, budget, 0, backwards)?;
        if backwards {
            v.truncate(1);
        }
        text.push_str(&xpath::simple_string(&v, " ", usize::MAX, backwards)?);
    }
    Ok(text)
}
fn ncname(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}
fn computed_name(
    n: &ComputedName,
    attribute: bool,
    frame: &Frame<'_>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<result::Name, Diagnostic> {
    let text = parts(&n.parts, &frame.focus, &frame.env, budget, backwards)?;
    let text = text.trim();
    let fields: Vec<_> = text.split(':').collect();
    let code = if attribute { "XTDE0850" } else { "XTDE0820" };
    if fields.len() > 2 || fields.iter().any(|s| !ncname(s)) {
        return Err(Diagnostic::new(
            code,
            "computed name is not a supported lexical QName",
            "run",
        ));
    }
    let (mut prefix, local) = if fields.len() == 2 {
        (fields[0].to_owned(), fields[1])
    } else {
        (String::new(), fields[0])
    };
    let uri = if let Some(es) = &n.namespace {
        parts(es, &frame.focus, &frame.env, budget, backwards)?
    } else if !prefix.is_empty() {
        n.bindings.get(&prefix).cloned().ok_or_else(|| {
            Diagnostic::new(
                if attribute { "XTDE0860" } else { "XTDE0830" },
                "unbound computed-name prefix",
                "run",
            )
        })?
    } else if attribute {
        String::new()
    } else {
        n.bindings.get("").cloned().unwrap_or_default()
    };
    if uri == "http://www.w3.org/2000/xmlns/"
        || prefix == "xml" && uri != "http://www.w3.org/XML/1998/namespace"
    {
        return Err(Diagnostic::new(
            if attribute { "XTDE0865" } else { "XTDE0835" },
            "reserved computed namespace",
            "run",
        ));
    }
    if attribute && text == "xmlns" && n.namespace.is_none() {
        return Err(Diagnostic::new(
            "XTDE0855",
            "cannot create an xmlns attribute",
            "run",
        ));
    }
    if prefix == "xmlns" {
        prefix = "ns".into();
    }
    if uri.is_empty() {
        prefix.clear();
    }
    if uri == "http://www.w3.org/XML/1998/namespace" {
        prefix = "xml".into();
    }
    Ok(result::Name {
        qname: if prefix.is_empty() {
            local.into()
        } else {
            format!("{prefix}:{local}")
        },
        local: local.into(),
        uri: if uri.is_empty() { None } else { Some(uri) },
    })
}
fn finish_capture(
    kind: Capture,
    value: String,
    output: &mut Output,
    budget: &mut Budget,
) -> Result<(), Diagnostic> {
    match kind {
        Capture::Attribute(name) => output.attribute(name, value),
        Capture::Namespace(prefix) => output.namespace(&prefix, &value),
        Capture::Comment => {
            let mut value = value.replace("--", "- -");
            while value.contains("--") {
                value = value.replace("--", "- -");
            }
            if value.ends_with('-') {
                value.push(' ');
            }
            output.comment(&value)
        }
        Capture::Message(terminate) => {
            if terminate {
                Err(Diagnostic::new("XTMM9000", value, "run"))
            } else {
                budget.messages.push(value);
                Ok(())
            }
        }
        Capture::Pi(name) => output.pi(&name, &value.replace("?>", "? >")),
    }
}
fn capture<'a>(
    content: &'a Content,
    kind: Capture,
    frame: Frame<'a>,
    sheet: &Stylesheet,
    budget: &mut Budget,
    output: &mut Output,
    tasks: &mut Vec<Task<'a>>,
) -> Result<(), Diagnostic> {
    match content {
        Content::Select(e, sep) => {
            let v = xpath::eval(e, &frame.focus, &frame.env, budget, 0, sheet.backwards)?;
            let sep = parts(sep, &frame.focus, &frame.env, budget, sheet.backwards)?;
            finish_capture(
                kind,
                xpath::simple_string(
                    &v,
                    &sep,
                    output.max.saturating_sub(output.value.len()),
                    sheet.backwards,
                )?,
                output,
                budget,
            )
        }
        Content::Body(body, sep) => {
            let separator = parts(sep, &frame.focus, &frame.env, budget, sheet.backwards)?;
            let captured = Output::simple(output.max.saturating_sub(output.value.len()), separator);
            let saved = std::mem::replace(output, captured);
            tasks.push(Task::FinishCapture(kind, saved));
            tasks.push(Task::Body(body, 0, frame));
            Ok(())
        }
    }
}
type Group<'a> = (Option<Value<'a>>, Value<'a>);
fn form_groups<'a>(
    items: Value<'a>,
    grouping: &Grouping,
    env: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<Vec<Group<'a>>, Diagnostic> {
    let size = items.len();
    let at = |index: usize, item: &Item<'a>| Focus {
        item: item.clone(),
        position: index + 1,
        size,
    };
    let mut groups: Vec<Group<'a>> = vec![];
    match grouping {
        Grouping::By(e) => {
            for (index, item) in items.iter().enumerate() {
                budget.tick()?;
                let keys = xpath::eval(e, &at(index, item), env, budget, 0, backwards)?;
                let mut seen: Vec<Item<'a>> = vec![];
                for key in keys.iter().map(xpath::atomize) {
                    if seen
                        .iter()
                        .any(|k| crate::deep_equal::atomic_equal(k, &key, true))
                    {
                        continue;
                    }
                    match groups.iter_mut().find(|(k, _)| {
                        k.as_ref()
                            .is_some_and(|k| crate::deep_equal::atomic_equal(&k[0], &key, true))
                    }) {
                        Some((_, members)) => members.push(item.clone()),
                        None => groups.push((Some(vec![key.clone()]), vec![item.clone()])),
                    }
                    seen.push(key);
                }
            }
        }
        Grouping::Adjacent(e) => {
            for (index, item) in items.iter().enumerate() {
                budget.tick()?;
                let keys = xpath::eval(e, &at(index, item), env, budget, 0, backwards)?;
                let [key] = keys.as_slice() else {
                    return Err(Diagnostic::new(
                        "XTTE1100",
                        "group-adjacent requires a single grouping key per item",
                        "run",
                    ));
                };
                let key = xpath::atomize(key);
                match groups.last_mut() {
                    Some((Some(k), members))
                        if crate::deep_equal::atomic_equal(&k[0], &key, true) =>
                    {
                        members.push(item.clone());
                    }
                    _ => groups.push((Some(vec![key]), vec![item.clone()])),
                }
            }
        }
        Grouping::Starting(branches) | Grouping::Ending(branches) => {
            let ending = matches!(grouping, Grouping::Ending(_));
            let mut after_end = true;
            for item in &items {
                budget.tick()?;
                let Item::Node(node) = item else {
                    return Err(Diagnostic::new(
                        "XTTE1120",
                        "group-starting-with and group-ending-with require nodes",
                        "run",
                    ));
                };
                let mut hit = false;
                for branch in branches {
                    if crate::pattern::matches(branch, *node, env, budget, backwards)? {
                        hit = true;
                        break;
                    }
                }
                if ending {
                    match groups.last_mut() {
                        Some((_, members)) if !after_end => members.push(item.clone()),
                        _ => groups.push((None, vec![item.clone()])),
                    }
                    after_end = hit;
                } else {
                    match groups.last_mut() {
                        Some((_, members)) if !hit => members.push(item.clone()),
                        _ => groups.push((None, vec![item.clone()])),
                    }
                }
            }
        }
    }
    Ok(groups)
}
fn number_matches<'a>(
    patterns: &[crate::pattern::Branch],
    node: XNode<'a>,
    env: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    for branch in patterns {
        if crate::pattern::matches(branch, node, env, budget, backwards)? {
            return Ok(true);
        }
    }
    Ok(false)
}
/// The sequence of numbers `xsl:number` computes for its position in the tree.
fn number_text<'a>(
    spec: &crate::compile::NumberSpec,
    focus: &Focus<'a>,
    env: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<String, Diagnostic> {
    use crate::compile::NumberLevel;
    let numbers: Vec<u64> = if let Some(value) = &spec.value {
        let items = xpath::eval(value, focus, env, budget, 0, backwards)?;
        let mut out = vec![];
        for item in &items {
            let n = match xpath::atomize(item) {
                xpath::Item::Number(n) => n.double(),
                xpath::Item::Untyped(s) => s.trim().parse().unwrap_or(f64::NAN),
                _ => f64::NAN,
            };
            let rounded = (n + 0.5).floor();
            if !rounded.is_finite() || rounded < 0.0 {
                return Err(Diagnostic::new(
                    "XTDE0980",
                    "xsl:number value must be a non-negative number",
                    "run",
                ));
            }
            out.push(rounded as u64);
        }
        out
    } else {
        let node = match &spec.select {
            Some(select) => match xpath::eval(select, focus, env, budget, 0, backwards)?.as_slice()
            {
                [Item::Node(n)] => *n,
                _ => {
                    return Err(Diagnostic::new(
                        "XTTE1000",
                        "xsl:number select must be a single node",
                        "run",
                    ));
                }
            },
            None => match focus.item {
                Item::Node(n) => n,
                _ => {
                    return Err(Diagnostic::new(
                        "XTTE0990",
                        "xsl:number requires a context node",
                        "run",
                    ));
                }
            },
        };
        let (kind_name, kind) = (
            node.name(),
            (
                node.is_element(),
                node.is_attribute(),
                node.is_text(),
                node.is_comment(),
                node.is_pi(),
            ),
        );
        let mut counted = |candidate: XNode<'a>, budget: &mut Budget| -> Result<bool, Diagnostic> {
            match &spec.count {
                Some(patterns) => number_matches(patterns, candidate, env, budget, backwards),
                None => Ok((
                    candidate.is_element(),
                    candidate.is_attribute(),
                    candidate.is_text(),
                    candidate.is_comment(),
                    candidate.is_pi(),
                ) == kind
                    && (candidate.is_text()
                        || candidate.is_comment()
                        || candidate.name() == kind_name)),
            }
        };
        let is_from = |candidate: XNode<'a>, budget: &mut Budget| -> Result<bool, Diagnostic> {
            match &spec.from {
                Some(patterns) => number_matches(patterns, candidate, env, budget, backwards),
                None => Ok(false),
            }
        };
        let position_among_siblings =
            |target: XNode<'a>,
             counted: &mut dyn FnMut(XNode<'a>, &mut Budget) -> Result<bool, Diagnostic>,
             budget: &mut Budget|
             -> Result<u64, Diagnostic> {
                let mut position = 1;
                if let Some(parent) = target.parent() {
                    if target.is_attribute() {
                        return Ok(1);
                    }
                    for sibling in parent.children() {
                        budget.tick()?;
                        if sibling == target {
                            break;
                        }
                        if counted(sibling, budget)? {
                            position += 1;
                        }
                    }
                }
                Ok(position)
            };
        match spec.level {
            NumberLevel::Single => {
                let mut current = Some(node);
                let mut target = None;
                while let Some(candidate) = current {
                    budget.tick()?;
                    if counted(candidate, budget)? {
                        target = Some(candidate);
                        break;
                    }
                    if is_from(candidate, budget)? {
                        break;
                    }
                    current = candidate.parent();
                }
                match target {
                    Some(target) => vec![position_among_siblings(target, &mut counted, budget)?],
                    None => vec![],
                }
            }
            NumberLevel::Multiple => {
                let mut chain = vec![];
                let mut current = Some(node);
                while let Some(candidate) = current {
                    budget.tick()?;
                    if counted(candidate, budget)? {
                        chain.push(candidate);
                    }
                    if is_from(candidate, budget)? {
                        break;
                    }
                    current = candidate.parent();
                }
                chain.reverse();
                let mut out = vec![];
                for target in chain {
                    out.push(position_among_siblings(target, &mut counted, budget)?);
                }
                out
            }
            NumberLevel::Any => {
                let mut total = 0u64;
                let mut seen_self = false;
                let root = node.root();
                let mut everything = vec![root];
                everything.extend(root.descendants());
                for candidate in everything {
                    budget.tick()?;
                    if is_from(candidate, budget)? {
                        total = 0;
                    }
                    if counted(candidate, budget)? {
                        total += 1;
                    }
                    if candidate == node {
                        seen_self = true;
                        break;
                    }
                }
                if seen_self && total > 0 {
                    vec![total]
                } else {
                    vec![]
                }
            }
        }
    };
    if numbers.is_empty() {
        return Ok(String::new());
    }
    let mut text = |parts: &[xpath::Expr]| parts_text(parts, focus, env, budget, backwards);
    let format = text(&spec.format)?;
    let ordinal = !text(&spec.ordinal)?.is_empty();
    let separator = text(&spec.grouping_separator)?;
    let size = text(&spec.grouping_size)?;
    let grouping =
        (!separator.is_empty() && !size.is_empty()).then(|| crate::numbering::Grouping {
            separator,
            size: size.trim().parse().unwrap_or(0),
        });
    Ok(crate::numbering::format_list(
        &numbers,
        &format,
        ordinal,
        grouping.as_ref(),
    ))
}
fn parts_text<'a>(
    parts_list: &[xpath::Expr],
    focus: &Focus<'a>,
    env: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<String, Diagnostic> {
    parts(parts_list, focus, env, budget, backwards)
}
fn focus_sequence(value: Value<'_>) -> Vec<Focus<'_>> {
    let size = value.len();
    value
        .into_iter()
        .enumerate()
        .map(|(i, item)| Focus {
            item,
            position: i + 1,
            size,
        })
        .collect()
}
/// Principal result, secondary documents (href, content) and message texts.
type AllResults = (String, Vec<(String, String)>, Vec<String>);
/// Transform and also return the documents created by `xsl:result-document`.
pub(crate) fn transform_all(
    sheet: &Stylesheet,
    source: &str,
    options: &TransformOptions,
) -> Result<AllResults, Diagnostic> {
    let prepared = crate::whitespace::prepare(sheet, source, false)?;
    let source = prepared.as_deref().unwrap_or(source);
    let absent = source.is_empty() && options.initial_template.is_some();
    let doc = crate::parse(if absent { "<_absent/>" } else { source }, "parse")?;
    let root = Focus {
        item: if absent {
            Item::Absent
        } else {
            Item::Node(XNode::tree(doc.root()))
        },
        position: 1,
        size: 1,
    };
    let sources = typed_arena::Arena::new();
    let documents = typed_arena::Arena::new();
    let trees = crate::trees::Trees::new(&sources, &documents, options.limits.max_temporary_bytes);
    let mut budget = Budget::new(options.limits);
    let principal = execute(sheet, root, options, &mut budget, true, &trees)?;
    Ok((
        principal,
        std::mem::take(&mut budget.secondary),
        std::mem::take(&mut budget.messages),
    ))
}
pub(crate) fn transform(
    sheet: &Stylesheet,
    source: &str,
    options: &TransformOptions,
) -> Result<String, Diagnostic> {
    let prepared = crate::whitespace::prepare(sheet, source, false)?;
    let source = prepared.as_deref().unwrap_or(source);
    let absent = source.is_empty() && options.initial_template.is_some();
    let doc = crate::parse(if absent { "<_absent/>" } else { source }, "parse")?;
    let root = Focus {
        item: if absent {
            Item::Absent
        } else {
            Item::Node(XNode::tree(doc.root()))
        },
        position: 1,
        size: 1,
    };
    let sources = typed_arena::Arena::new();
    let documents = typed_arena::Arena::new();
    let trees = crate::trees::Trees::new(&sources, &documents, options.limits.max_temporary_bytes);
    execute(
        sheet,
        root,
        options,
        &mut Budget::new(options.limits),
        true,
        &trees,
    )
}
pub(crate) fn transform_record(
    sheet: &Stylesheet,
    source: &str,
    options: &TransformOptions,
    budget: &mut Budget,
    position: usize,
    inherited_preserve: bool,
) -> Result<String, Diagnostic> {
    budget.functions = sheet.functions.clone();
    budget.keys = sheet.keys.clone();
    budget.decimal_formats = sheet.decimal_formats.clone();
    let prepared = crate::whitespace::prepare(sheet, source, inherited_preserve)?;
    let source = prepared.as_deref().unwrap_or(source);
    let doc = crate::parse(source, "parse")?;
    let root = Focus {
        item: Item::Node(XNode::tree(doc.root_element())),
        position,
        size: 1,
    };
    let sources = typed_arena::Arena::new();
    let documents = typed_arena::Arena::new();
    let trees = crate::trees::Trees::new(&sources, &documents, options.limits.max_temporary_bytes);
    execute(sheet, root, options, budget, false, &trees)
}
fn copied_leaf(node: XNode<'_>, output: &mut Output) -> Result<(), Diagnostic> {
    if let Some((name, uri)) = node.attribute {
        output.attribute(result::attribute_name(node.raw, name, uri), node.string())
    } else if node.is_text() {
        output.text(&node.string())
    } else if node.is_comment() {
        output.comment(&node.string())
    } else if node.is_pi() {
        let pi = node.raw.pi().unwrap();
        output.pi(pi.target, pi.value.unwrap_or_default())
    } else {
        Ok(())
    }
}
fn parameter_value<'a>(value: &crate::ParameterValue) -> Result<Value<'a>, Diagnostic> {
    use crate::{ParameterValue as P, number::Number};
    Ok(match value {
        P::Empty => vec![],
        P::String(s) => vec![Item::String(s.clone())],
        P::UntypedAtomic(s) => vec![Item::Untyped(s.clone())],
        P::Boolean(v) => vec![Item::Boolean(*v)],
        P::Integer(s) => vec![Item::Number(Number::cast("integer", s)?)],
        P::Decimal(s) => vec![Item::Number(Number::cast("decimal", s)?)],
        P::Double(n) => vec![Item::Number(Number::Double(*n))],
        P::Float(n) => vec![Item::Number(Number::Float(*n))],
    })
}
fn insert_parameter<'a>(
    parameters: &mut Variables<'a>,
    name: &str,
    value: Value<'a>,
) -> Result<(), Diagnostic> {
    let name = crate::qname::external(name)?;
    if parameters.insert(name.clone(), value).is_some() {
        return Err(Diagnostic::new(
            "RUST_XSLT_INVALID_PARAMETER",
            format!("duplicate external parameter: {name}"),
            "run",
        ));
    }
    Ok(())
}
fn execute<'a>(
    sheet: &'a Stylesheet,
    root: Focus<'a>,
    options: &TransformOptions,
    budget: &mut Budget,
    declaration: bool,
    trees: &crate::trees::Trees<'a>,
) -> Result<String, Diagnostic> {
    budget.functions = sheet.functions.clone();
    budget.keys = sheet.keys.clone();
    budget.decimal_formats = sheet.decimal_formats.clone();
    let mut parameters = Variables::new();
    for (name, value) in &options.parameters {
        budget.tick()?;
        insert_parameter(&mut parameters, name, vec![Item::String(value.clone())])?;
    }
    for (name, value) in &options.typed_parameters {
        budget.tick()?;
        insert_parameter(&mut parameters, name, parameter_value(value)?)?;
    }
    let mut initial_parameters = Supplied::default();
    for (name, value) in &options.initial_parameters {
        budget.tick()?;
        insert_parameter(
            &mut initial_parameters.ordinary,
            name,
            parameter_value(value)?,
        )?;
    }
    for (name, value) in &options.initial_tunnel_parameters {
        budget.tick()?;
        insert_parameter(
            Rc::make_mut(&mut initial_parameters.tunnel),
            name,
            parameter_value(value)?,
        )?;
    }
    let limits = options.limits;
    let mut globals = Variables::new();
    let mut pending: Vec<_> = sheet.globals.iter().collect();
    while !pending.is_empty() {
        let before = pending.len();
        let mut remaining = vec![];
        for b in pending {
            budget.tick()?;
            if let Some(s) = parameters.get(&b.name).filter(|_| b.parameter) {
                let mut v = (*s).clone();
                if let Some(ty) = &b.ty {
                    v = ty.convert(v, "XTTE0590")?;
                }
                globals.insert(format!("{}{}", xpath::GLOBAL_PREFIX, b.name), v.clone());
                globals.insert(b.name.clone(), v);
                continue;
            }
            if b.required {
                return Err(Diagnostic::new(
                    "XTDE0050",
                    format!("required global parameter not supplied: {}", b.name),
                    "run",
                ));
            }
            if b.implicit_required {
                return Err(Diagnostic::new(
                    if sheet.version < 3 {
                        "XTDE0610"
                    } else {
                        "XTDE0700"
                    },
                    format!(
                        "implicitly required global parameter not supplied: {}",
                        b.name
                    ),
                    "run",
                ));
            }
            let evaluated = match &b.value {
                BindingValue::Select(e) => {
                    xpath::eval(e, &root, &globals, budget, 0, sheet.backwards)
                }
                BindingValue::Body(body)
                | BindingValue::Document(body)
                | BindingValue::Raw(body) => {
                    let env = Rc::new(globals.clone());
                    let frame = Frame {
                        focus: root.clone(),
                        env: env.clone(),
                        depth: 0,
                        mode: String::new(),
                        tunnel: Rc::new(Variables::new()),
                        rule: None,
                    };
                    let run = Run {
                        sheet,
                        options,
                        trees,
                        globals: env,
                    };
                    if matches!(b.value, BindingValue::Raw(_)) {
                        let mut captured = vec![];
                        run_tasks(
                            &run,
                            budget,
                            vec![Task::Body(body, 0, frame)],
                            Output::document(limits.max_output_bytes),
                            Some(&mut captured),
                        )
                        .map(|_| captured)
                    } else {
                        run_tasks(
                            &run,
                            budget,
                            vec![Task::Body(body, 0, frame)],
                            Output::document(limits.max_output_bytes),
                            None,
                        )
                        .and_then(|xml| {
                            trees.construct(
                                xml,
                                if matches!(b.value, BindingValue::Document(_)) {
                                    None
                                } else {
                                    b.ty.as_ref()
                                },
                                if b.parameter { "XTTE0600" } else { "XTTE0570" },
                            )
                        })
                    }
                }
            };
            match evaluated {
                Ok(mut v) => {
                    if let Some(ty) = &b.ty {
                        v = ty.convert(v, if b.parameter { "XTTE0600" } else { "XTTE0570" })?;
                    }
                    globals.insert(format!("{}{}", xpath::GLOBAL_PREFIX, b.name), v.clone());
                    globals.insert(b.name.clone(), v);
                }
                Err(e) if e.code == "XPST0008" => remaining.push(b),
                Err(e) => return Err(e),
            }
        }
        if remaining.len() == before {
            return Err(Diagnostic::new(
                "XTDE0640",
                "circular global variable dependency",
                "run",
            ));
        }
        pending = remaining;
    }
    let globals = Rc::new(globals);
    let mut output = Output::new(limits.max_output_bytes, sheet.text_output)
        .with_serialization(sheet.markup, sheet.doctype.clone());
    if declaration && !sheet.text_output && !sheet.omit_declaration {
        output.push("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    }
    let initial_mode = options
        .initial_mode
        .as_deref()
        .map(|name| {
            if name == "#unnamed" {
                Ok(String::new())
            } else {
                crate::qname::external(name)
            }
        })
        .transpose()?
        .unwrap_or_else(|| {
            if options.initial_template.is_some() {
                String::new()
            } else {
                sheet.default_mode.clone()
            }
        });
    if options.initial_template.is_none()
        && options.initial_mode.is_some()
        && !initial_mode.is_empty()
        && initial_mode != sheet.default_mode
    {
        let declared = sheet.modes.get(&initial_mode);
        let implicit = sheet
            .templates
            .iter()
            .any(|t| t.pattern.is_some() && t.modes.contains(&initial_mode));
        if declared.is_none() && !implicit
            || declared.is_some_and(|m| m.visibility == Some(crate::mode::Visibility::Private))
        {
            return Err(Diagnostic::new(
                "XTDE0045",
                format!("ineligible initial mode: {initial_mode}"),
                "run",
            ));
        }
    }
    let tasks = if let Some(name) = &options.initial_template {
        let name = crate::qname::external(name)?;
        let t = sheet
            .templates
            .iter()
            .find(|t| t.name.as_ref() == Some(&name))
            .ok_or_else(|| {
                Diagnostic::new(
                    "XTDE0040",
                    format!("unknown initial template: {name}"),
                    "run",
                )
            })?;
        vec![Task::Enter(
            t,
            root,
            initial_parameters,
            0,
            initial_mode,
            None,
        )]
    } else {
        vec![Task::Apply(
            root,
            0,
            initial_mode,
            Rc::new(initial_parameters),
            None,
        )]
    };
    let run = Run {
        sheet,
        options,
        trees,
        globals,
    };
    let result = run_tasks(&run, budget, tasks, output, None)?;
    if sheet.indent && declaration && !sheet.text_output {
        return Ok(crate::indent::indent(&result));
    }
    Ok(result)
}
fn run_tasks<'a>(
    run: &Run<'a, '_>,
    budget: &mut Budget,
    mut tasks: Vec<Task<'a>>,
    mut output: Output,
    raw_result: Option<&mut Value<'a>>,
) -> Result<String, Diagnostic> {
    let sheet = run.sheet;
    let limits = run.options.limits;
    let globals = run.globals.clone();
    let trees = run.trees;
    let mut captures = if raw_result.is_some() {
        vec![RawCapture {
            items: vec![],
            active: true,
        }]
    } else {
        vec![]
    };
    let mut iterations: Vec<IterationState<'a>> = vec![];
    while let Some(task) = tasks.pop() {
        budget.tick()?;
        match task {
            Task::Enter(t, focus, params, depth, mode, rule) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                tasks.push(Task::Parameters(
                    t,
                    0,
                    params.ordinary,
                    Frame {
                        focus,
                        env: Rc::new((*globals).clone()),
                        depth,
                        mode,
                        tunnel: params.tunnel,
                        rule,
                    },
                ));
            }
            Task::Parameters(t, index, mut supplied, mut frame) => {
                if let Some(p) = t.params.get(index) {
                    let value = if p.tunnel {
                        frame.tunnel.get(&p.name).cloned()
                    } else {
                        supplied.remove(&p.name)
                    };
                    if let Some(mut value) = value {
                        if let Some(ty) = &p.ty {
                            value = ty.convert(value, "XTTE0590")?;
                        }
                        Rc::make_mut(&mut frame.env).insert(p.name.clone(), value);
                        tasks.push(Task::Parameters(t, index + 1, supplied, frame));
                    } else {
                        if p.required || p.implicit_required {
                            return Err(Diagnostic::new(
                                if p.implicit_required && !p.required && sheet.version < 3 {
                                    "XTDE0610"
                                } else {
                                    "XTDE0700"
                                },
                                format!("required template parameter not supplied: {}", p.name),
                                "run",
                            ));
                        }
                        tasks.push(Task::EvaluateBinding(
                            p,
                            frame,
                            BindNext::Parameter(t, index + 1, supplied),
                        ));
                    }
                } else if let Some(ty) = &t.result_type {
                    let unconstrained = matches!(ty.kind, crate::seqtype::Kind::Item)
                        && ty.min == 0
                        && ty.max == usize::MAX;
                    if unconstrained {
                        tasks.push(Task::Body(&t.body, 0, frame));
                    } else if let [Instruction::Sequence(e)] = t.body.as_slice() {
                        let value =
                            xpath::eval(e, &frame.focus, &frame.env, budget, 0, sheet.backwards)?;
                        emit_value(
                            ty.convert(value, "XTTE0505")?,
                            &mut tasks,
                            frame.depth,
                            true,
                        );
                    } else if crate::compile::raw_result_body(&t.body) {
                        let saved = std::mem::replace(
                            &mut output,
                            Output::document(limits.max_output_bytes),
                        );
                        captures.push(RawCapture {
                            items: vec![],
                            active: true,
                        });
                        tasks.push(Task::FinishRawTemplate(ty, saved, frame.depth));
                        tasks.push(Task::Body(&t.body, 0, frame));
                    } else {
                        let saved = std::mem::replace(
                            &mut output,
                            Output::document(limits.max_output_bytes),
                        );
                        let was_raw = suspend_raw(&mut captures);
                        let document = matches!(t.body.as_slice(), [Instruction::Document(_)]);
                        tasks.push(Task::FinishTemplate(
                            ty,
                            saved,
                            frame.depth,
                            document,
                            was_raw,
                        ));
                        if let [Instruction::Document(body)] = t.body.as_slice() {
                            frame.depth += 1;
                            tasks.push(Task::Body(body, 0, frame));
                        } else {
                            tasks.push(Task::Body(&t.body, 0, frame));
                        }
                    }
                } else {
                    tasks.push(Task::Body(&t.body, 0, frame));
                }
            }
            Task::FinishRawAttribute(saved, depth) => {
                output.end()?;
                let xml = std::mem::replace(&mut output, saved).value;
                let value = trees.construct(xml, None, "XTTE0570")?;
                let Item::Node(document) = value[0] else {
                    unreachable!()
                };
                let attribute = document.children().next().unwrap().attributes()[0].detached();
                restore_raw(&mut captures, true);
                emit_value(vec![Item::Node(attribute)], &mut tasks, depth, true);
            }
            Task::FinishRawNode(saved, depth, document) => {
                let xml = std::mem::replace(&mut output, saved).value;
                let nodes = crate::seqtype::SequenceType {
                    kind: crate::seqtype::Kind::Item,
                    min: 0,
                    max: usize::MAX,
                };
                let value =
                    trees.construct(xml, if document { None } else { Some(&nodes) }, "XTTE0570")?;
                restore_raw(&mut captures, true);
                emit_value(value, &mut tasks, depth, true);
            }
            Task::FinishRawTemplate(ty, saved, depth) => {
                output = saved;
                let captured = captures.pop().unwrap().items;
                emit_value(ty.convert(captured, "XTTE0505")?, &mut tasks, depth, true);
            }
            Task::FinishTemplate(ty, saved, depth, document, was_raw) => {
                let xml = std::mem::replace(&mut output, saved).value;
                let value =
                    trees.construct(xml, if document { None } else { Some(ty) }, "XTTE0505")?;
                let value = if document {
                    ty.convert(value, "XTTE0505")?
                } else {
                    value
                };
                restore_raw(&mut captures, was_raw);
                emit_value(value, &mut tasks, depth, true);
            }
            Task::ApplyArguments(foci, mode, params, index, args, frame, before) => {
                if let Some(p) = params.get(index) {
                    tasks.push(Task::EvaluateBinding(
                        p,
                        frame,
                        BindNext::ApplyArgument(foci, mode, params, index + 1, args, before),
                    ));
                } else {
                    let args = Rc::new(args);
                    for focus in foci.into_iter().rev() {
                        tasks.push(Task::Apply(
                            focus,
                            frame.depth + 1,
                            mode.clone(),
                            args.clone(),
                            before,
                        ));
                    }
                }
            }
            Task::Arguments(t, params, index, args, frame) => {
                if let Some(p) = params.get(index) {
                    tasks.push(Task::EvaluateBinding(
                        p,
                        frame,
                        BindNext::Argument(t, params, index + 1, args),
                    ));
                } else {
                    tasks.push(Task::Enter(
                        t,
                        frame.focus,
                        args,
                        frame.depth + 1,
                        frame.mode,
                        frame.rule,
                    ));
                }
            }
            Task::EvaluateBinding(binding, frame, next) => match &binding.value {
                BindingValue::Select(e) => {
                    let value =
                        xpath::eval(e, &frame.focus, &frame.env, budget, 0, sheet.backwards)?;
                    complete_binding(binding, value, frame, next, &mut tasks)?;
                }
                BindingValue::Raw(body) => {
                    let saved =
                        std::mem::replace(&mut output, Output::document(limits.max_output_bytes));
                    captures.push(RawCapture {
                        items: vec![],
                        active: true,
                    });
                    let mut constructor = frame.clone();
                    constructor.depth += 1;
                    tasks.push(Task::FinishRawBinding(binding, frame, next, saved));
                    tasks.push(Task::Body(body, 0, constructor));
                }
                BindingValue::Body(body) | BindingValue::Document(body) => {
                    let was_raw = suspend_raw(&mut captures);
                    let saved =
                        std::mem::replace(&mut output, Output::document(limits.max_output_bytes));
                    let mut constructor = frame.clone();
                    constructor.depth += 1;
                    tasks.push(Task::FinishBinding(binding, frame, next, saved, was_raw));
                    tasks.push(Task::Body(body, 0, constructor));
                }
            },
            Task::FinishRawBinding(binding, frame, next, saved) => {
                output = saved;
                let captured = captures.pop().unwrap().items;
                complete_binding(binding, captured, frame, next, &mut tasks)?;
            }
            Task::FinishBinding(binding, frame, next, saved, was_raw) => {
                let xml = std::mem::replace(&mut output, saved).value;
                let value = trees.construct(
                    xml,
                    if matches!(binding.value, BindingValue::Document(_)) {
                        None
                    } else {
                        binding.ty.as_ref()
                    },
                    next.type_error(),
                )?;
                restore_raw(&mut captures, was_raw);
                complete_binding(binding, value, frame, next, &mut tasks)?;
            }
            Task::Apply(focus, depth, mode, params, before) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                let Item::Node(node) = focus.item else {
                    return Err(Diagnostic::new(
                        "XTTE0520",
                        "apply-templates requires nodes",
                        "run",
                    ));
                };
                let definition = sheet.modes.get(&mode).copied().unwrap_or_default();
                let mut best: Option<Rule> = None;
                let mut ambiguous = false;
                for (ti, t) in sheet.templates.iter().enumerate() {
                    budget.tick()?;
                    if !t.modes.iter().any(|m| m == &mode || m == "#all") {
                        continue;
                    }
                    if let Some(branches) = &t.pattern {
                        for (bi, b) in branches.iter().enumerate() {
                            let candidate = Rule {
                                precedence: t.precedence,
                                template: ti,
                                branch: if t.priority.is_some() { 0 } else { bi },
                                priority: t.priority.unwrap_or(b.priority),
                            };
                            if before.is_some_and(|bound| !candidate.compare(bound).is_lt()) {
                                continue;
                            }
                            if best.is_some_and(|best| candidate.rank(best).is_lt()) {
                                continue;
                            }
                            if crate::pattern::matches(b, node, &globals, budget, sheet.backwards)?
                            {
                                if let Some(previous) = best {
                                    if candidate.rank(previous).is_gt() {
                                        ambiguous = false;
                                    } else if candidate.rank(previous).is_eq()
                                        && candidate.template != previous.template
                                    {
                                        ambiguous = true;
                                    }
                                }
                                if best.is_none_or(|best| candidate.compare(best).is_gt()) {
                                    best = Some(candidate);
                                }
                            }
                        }
                    }
                }
                if ambiguous && definition.fail_on_multiple == Some(true) {
                    return Err(Diagnostic::new(
                        "XTDE0540",
                        "multiple template rules match at the highest priority",
                        "run",
                    ));
                }
                if let Some(rule) = best {
                    tasks.push(Task::Enter(
                        &sheet.templates[rule.template],
                        focus,
                        (*params).clone(),
                        depth,
                        mode,
                        Some(rule),
                    ));
                } else {
                    use crate::mode::OnNoMatch as P;
                    let policy = definition.policy();
                    if policy == P::Fail {
                        return Err(Diagnostic::new(
                            "XTDE0555",
                            "no matching template rule",
                            "run",
                        ));
                    }
                    if policy == P::DeepCopy {
                        if raw_active(&captures) {
                            tasks.push(Task::RawCopy(Item::Node(node), depth, true));
                        } else {
                            tasks.push(Task::DeepCopy(node, depth, true));
                        }
                        continue;
                    }
                    // Even deep-skip descends through a document node.
                    if policy == P::DeepSkip && !node.is_document() {
                        continue;
                    }
                    if node.is_document() || node.is_element() {
                        if policy == P::ShallowCopy && raw_active(&captures) {
                            begin_raw_node(
                                &mut output,
                                &mut captures,
                                &mut tasks,
                                depth,
                                node.is_document(),
                            );
                        }
                        if policy == P::ShallowCopy && node.is_element() {
                            output.start(result::source_element(node.raw, false))?;
                            tasks.push(Task::Close);
                        }
                        let inherited = if sheet.backwards {
                            Rc::new(Supplied::inherited(params.tunnel.clone()))
                        } else {
                            params
                        };
                        let children = node.children().map(Item::Node).collect();
                        for f in focus_sequence(children).into_iter().rev() {
                            tasks.push(Task::Apply(
                                f,
                                depth + 1,
                                mode.clone(),
                                inherited.clone(),
                                None,
                            ));
                        }
                        if matches!(policy, P::ShallowSkip | P::ShallowCopy) {
                            let attributes =
                                node.attributes().into_iter().map(Item::Node).collect();
                            for f in focus_sequence(attributes).into_iter().rev() {
                                tasks.push(Task::Apply(
                                    f,
                                    depth + 1,
                                    mode.clone(),
                                    inherited.clone(),
                                    None,
                                ));
                            }
                        }
                    } else if policy == P::ShallowCopy {
                        if raw_active(&captures) {
                            tasks.push(Task::RawCopy(Item::Node(node), depth, true));
                        } else {
                            copied_leaf(node, &mut output)?;
                        }
                    } else if policy == P::TextOnlyCopy && (node.is_text() || node.is_attribute()) {
                        if raw_active(&captures) {
                            captures.last_mut().unwrap().items.extend(raw_text(
                                &node.string(),
                                trees,
                                limits.max_output_bytes,
                            )?);
                        } else {
                            output.text(&node.string())?;
                        }
                    }
                }
            }
            Task::Body(body, i, frame) => {
                if frame.depth > limits.max_depth {
                    return Err(limit());
                }
                let Some(instruction) = body.get(i) else {
                    continue;
                };
                if let Instruction::Bind(b) = instruction {
                    tasks.push(Task::EvaluateBinding(
                        b,
                        frame,
                        BindNext::Local(body, i + 1),
                    ));
                    continue;
                }
                if raw_active(&captures)
                    && !matches!(
                        instruction,
                        Instruction::Sequence(_)
                            | Instruction::CopyOf(..)
                            | Instruction::Copy { .. }
                            | Instruction::Apply(..)
                            | Instruction::Call(..)
                            | Instruction::NextMatch(_)
                            | Instruction::ApplyImports(_)
                            | Instruction::ForEach(..)
                            | Instruction::ForEachGroup(..)
                            | Instruction::AnalyzeString { .. }
                            | Instruction::Iterate { .. }
                            | Instruction::NextIteration(_)
                            | Instruction::Break(_)
                            | Instruction::Number(_)
                            | Instruction::WherePopulated(_)
                            | Instruction::Namespace(..)
                            | Instruction::ResultDocument(_)
                            | Instruction::If(..)
                            | Instruction::Choose(_)
                            | Instruction::Element { .. }
                            | Instruction::ComputedElement(..)
                            | Instruction::Document(_)
                            | Instruction::Text(_)
                            | Instruction::Value(..)
                            | Instruction::Comment(_)
                            | Instruction::Pi(..)
                            | Instruction::Attribute(..)
                    )
                {
                    return Err(Diagnostic::new(
                        "RUST_XSLT_UNSUPPORTED",
                        "raw constructed-node collection is not implemented",
                        "run",
                    ));
                }
                tasks.push(Task::Body(body, i + 1, frame.clone()));
                let Frame {
                    focus,
                    env,
                    depth,
                    mode,
                    tunnel,
                    rule,
                } = frame;
                match instruction {
                    Instruction::ComputedElement(n, inherit, body) => {
                        let frame = Frame {
                            focus,
                            env,
                            depth: depth + 1,
                            mode,
                            tunnel: tunnel.clone(),
                            rule,
                        };
                        let name = computed_name(n, false, &frame, budget, sheet.backwards)?;
                        if raw_active(&captures) {
                            begin_raw_node(&mut output, &mut captures, &mut tasks, depth, false);
                        }
                        output.start(result::Element {
                            name,
                            namespaces: Default::default(),
                            attributes: vec![],
                            inherit_namespaces: *inherit,
                        })?;
                        tasks.push(Task::Close);
                        tasks.push(Task::Body(body, 0, frame));
                    }
                    Instruction::Attribute(n, content) => {
                        let frame = Frame {
                            focus,
                            env,
                            depth: depth + 1,
                            mode,
                            tunnel: tunnel.clone(),
                            rule,
                        };
                        let name = computed_name(n, true, &frame, budget, sheet.backwards)?;
                        if raw_active(&captures) {
                            begin_raw_attribute(&mut output, &mut captures, &mut tasks, depth)?;
                        }
                        capture(
                            content,
                            Capture::Attribute(name),
                            frame,
                            sheet,
                            budget,
                            &mut output,
                            &mut tasks,
                        )?;
                    }
                    Instruction::Comment(content) => {
                        if raw_active(&captures) {
                            begin_raw_node(&mut output, &mut captures, &mut tasks, depth, false);
                        }
                        capture(
                            content,
                            Capture::Comment,
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                            sheet,
                            budget,
                            &mut output,
                            &mut tasks,
                        )?;
                    }
                    Instruction::Message(content, terminate) => {
                        capture(
                            content,
                            Capture::Message(*terminate),
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                            sheet,
                            budget,
                            &mut output,
                            &mut tasks,
                        )?;
                    }
                    Instruction::Pi(n, content) => {
                        let name = parts(n, &focus, &env, budget, sheet.backwards)?;
                        let name = name.trim().to_owned();
                        if !ncname(&name) || name.eq_ignore_ascii_case("xml") {
                            return Err(Diagnostic::new(
                                "XTDE0890",
                                "invalid processing-instruction target",
                                "run",
                            ));
                        }
                        if raw_active(&captures) {
                            begin_raw_node(&mut output, &mut captures, &mut tasks, depth, false);
                        }
                        capture(
                            content,
                            Capture::Pi(name),
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                            sheet,
                            budget,
                            &mut output,
                            &mut tasks,
                        )?;
                    }
                    Instruction::Bind(_) => unreachable!(),
                    Instruction::Text(t) => {
                        if raw_active(&captures) {
                            captures
                                .last_mut()
                                .unwrap()
                                .items
                                .extend(raw_text(t, trees, output.max)?);
                        } else {
                            output.text(t)?;
                        }
                    }
                    Instruction::Value(e, separator) => {
                        let mut v = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        if sheet.backwards && separator.is_none() {
                            v.truncate(1);
                        }
                        let separator = separator
                            .as_ref()
                            .map(|es| parts(es, &focus, &env, budget, sheet.backwards))
                            .transpose()?
                            .unwrap_or_else(|| " ".into());
                        for _ in &v {
                            budget.tick()?;
                        }
                        let text = xpath::simple_string(
                            &v,
                            &separator,
                            output.max.saturating_sub(output.value.len()),
                            sheet.backwards,
                        )?;
                        if raw_active(&captures) {
                            captures
                                .last_mut()
                                .unwrap()
                                .items
                                .extend(raw_text(&text, trees, output.max)?);
                        } else {
                            output.text(&text)?;
                        }
                    }
                    Instruction::Apply(e, target_mode, sorts, params) => {
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        let value = crate::sort::sort(value, sorts, &env, budget, sheet.backwards)?;
                        let target_mode = if target_mode == "#current" {
                            mode.clone()
                        } else {
                            target_mode.clone()
                        };
                        tasks.push(Task::ApplyArguments(
                            focus_sequence(value),
                            target_mode,
                            params,
                            0,
                            Supplied::inherited(tunnel.clone()),
                            Frame {
                                focus,
                                env,
                                depth,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                            None,
                        ));
                    }
                    Instruction::CopyOf(e, _) | Instruction::Sequence(e) => {
                        let namespaces = match instruction {
                            Instruction::CopyOf(_, value) => *value,
                            _ => true,
                        };
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        if raw_active(&captures) && matches!(instruction, Instruction::CopyOf(..)) {
                            for item in value.into_iter().rev() {
                                tasks.push(Task::RawCopy(item, depth, namespaces));
                            }
                        } else {
                            emit_value(value, &mut tasks, depth, namespaces);
                        }
                    }
                    Instruction::Copy {
                        select,
                        namespaces,
                        inherit,
                        body,
                    } => {
                        let mut focus = focus;
                        if let Some(e) = select {
                            let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                            if value.is_empty() {
                                continue;
                            }
                            if value.len() != 1 {
                                return Err(Diagnostic::new(
                                    "XTTE3180",
                                    "copy select requires at most one item",
                                    "run",
                                ));
                            }
                            focus = Focus {
                                item: value.into_iter().next().unwrap(),
                                position: 1,
                                size: 1,
                            };
                        }
                        let rule = if select.is_some() { None } else { rule };
                        if raw_active(&captures) {
                            match &focus.item {
                                Item::Node(n) if n.is_element() || n.is_document() => {
                                    begin_raw_node(
                                        &mut output,
                                        &mut captures,
                                        &mut tasks,
                                        depth,
                                        n.is_document(),
                                    );
                                }
                                Item::Absent => {}
                                item => {
                                    tasks.push(Task::RawCopy(item.clone(), depth, *namespaces));
                                    continue;
                                }
                            }
                        }
                        match focus.item.clone() {
                            Item::Node(n) if n.is_element() => {
                                let mut element = result::source_element(n.raw, false);
                                if !namespaces {
                                    element.namespaces.clear();
                                }
                                element.inherit_namespaces = *inherit;
                                output.start(element)?;
                                tasks.push(Task::Close);
                                tasks.push(Task::Body(
                                    body,
                                    0,
                                    Frame {
                                        focus,
                                        env,
                                        depth: depth + 1,
                                        mode,
                                        tunnel: tunnel.clone(),
                                        rule,
                                    },
                                ));
                            }
                            Item::Node(n) if n.is_document() => tasks.push(Task::Body(
                                body,
                                0,
                                Frame {
                                    focus,
                                    env,
                                    depth: depth + 1,
                                    mode,
                                    tunnel: tunnel.clone(),
                                    rule,
                                },
                            )),
                            Item::Node(n) => copied_leaf(n, &mut output)?,
                            Item::Absent => {
                                return Err(Diagnostic::new(
                                    "XPDY0002",
                                    "copy requires a context item",
                                    "run",
                                ));
                            }
                            atomic => output.atomic(&atomic.string_compat(sheet.backwards))?,
                        }
                    }
                    Instruction::Document(body) => {
                        if raw_active(&captures) {
                            begin_raw_node(&mut output, &mut captures, &mut tasks, depth, true);
                            tasks.push(Task::Body(
                                body,
                                0,
                                Frame {
                                    focus,
                                    env,
                                    depth: depth + 1,
                                    mode,
                                    tunnel,
                                    rule,
                                },
                            ));
                            continue;
                        }
                        let saved = std::mem::replace(
                            &mut output,
                            Output::document(limits.max_output_bytes),
                        );
                        tasks.push(Task::FinishDocument(saved, depth));
                        tasks.push(Task::Body(
                            body,
                            0,
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                        ));
                    }
                    Instruction::ForEach(e, sorts, body) => {
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        let value = crate::sort::sort(value, sorts, &env, budget, sheet.backwards)?;
                        for f in focus_sequence(value).into_iter().rev() {
                            tasks.push(Task::Body(
                                body,
                                0,
                                Frame {
                                    focus: f,
                                    env: env.clone(),
                                    depth: depth + 1,
                                    mode: mode.clone(),
                                    tunnel: tunnel.clone(),
                                    rule: None,
                                },
                            ));
                        }
                    }
                    Instruction::Namespace(name, content) => {
                        let prefix = parts(name, &focus, &env, budget, sheet.backwards)?;
                        let prefix = prefix.trim().to_owned();
                        if !prefix.is_empty() && !crate::qname::xml_ncname(&prefix) {
                            return Err(Diagnostic::new(
                                "XTDE0920",
                                format!("invalid namespace prefix: {prefix}"),
                                "run",
                            ));
                        }
                        capture(
                            content,
                            Capture::Namespace(prefix),
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                            sheet,
                            budget,
                            &mut output,
                            &mut tasks,
                        )?;
                    }
                    Instruction::ResultDocument(spec) => {
                        use crate::result::Markup;
                        if TRY_DEPTH.with(|d| d.get()) >= 32 {
                            return Err(limit());
                        }
                        let href = parts(&spec.href, &focus, &env, budget, sheet.backwards)?;
                        if href.is_empty() {
                            return Err(Diagnostic::new(
                                "XTDE1490",
                                "xsl:result-document needs an href",
                                "run",
                            ));
                        }
                        if budget
                            .secondary
                            .iter()
                            .any(|(existing, _)| *existing == href)
                        {
                            return Err(Diagnostic::new(
                                "XTDE1490",
                                format!("two result documents for {href}"),
                                "run",
                            ));
                        }
                        let text_method = spec.method == "text";
                        let markup = match spec.method.as_str() {
                            "html" => Markup::Html,
                            "xhtml" => Markup::Xhtml,
                            _ => Markup::Xml,
                        };
                        let nested = Output::new(limits.max_output_bytes, text_method)
                            .with_serialization(markup, None);
                        let inner = Frame {
                            focus,
                            env,
                            depth: depth + 1,
                            mode,
                            tunnel: tunnel.clone(),
                            rule: None,
                        };
                        TRY_DEPTH.with(|d| d.set(d.get() + 1));
                        let result = run_tasks(
                            run,
                            budget,
                            vec![Task::Body(&spec.body, 0, inner)],
                            nested,
                            None,
                        );
                        TRY_DEPTH.with(|d| d.set(d.get() - 1));
                        let mut content = result?;
                        if spec.indent.unwrap_or(false) && !text_method {
                            content = crate::indent::indent(&content);
                        }
                        let omit = spec
                            .omit_declaration
                            .unwrap_or(spec.method == "html" || text_method);
                        if !omit {
                            content =
                                format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{content}");
                            if spec.indent.unwrap_or(false) {
                                content = content.replacen("?><", "?>\n<", 1);
                            }
                        }
                        budget.secondary.push((href, content));
                    }
                    Instruction::WherePopulated(content) => {
                        if TRY_DEPTH.with(|d| d.get()) >= 32 {
                            return Err(limit());
                        }
                        let nested = Output::new(
                            output.max.saturating_sub(output.value.len()),
                            sheet.text_output,
                        )
                        .with_serialization(output.serialization().0, None);
                        let inner = Frame {
                            focus,
                            env,
                            depth: depth + 1,
                            mode,
                            tunnel: tunnel.clone(),
                            rule: None,
                        };
                        TRY_DEPTH.with(|d| d.set(d.get() + 1));
                        let result = run_tasks(
                            run,
                            budget,
                            vec![Task::Body(content, 0, inner)],
                            nested,
                            None,
                        );
                        TRY_DEPTH.with(|d| d.set(d.get() - 1));
                        let text = result?;
                        // Only content that produced something survives.
                        if !text.is_empty() {
                            output.splice(&text)?;
                        }
                    }
                    Instruction::Number(spec) => {
                        let text = number_text(spec, &focus, &env, budget, sheet.backwards)?;
                        if raw_active(&captures) {
                            captures
                                .last_mut()
                                .unwrap()
                                .items
                                .extend(raw_text(&text, trees, output.max)?);
                        } else {
                            output.text(&text)?;
                        }
                    }
                    Instruction::Try {
                        body: try_body,
                        catches,
                    } => {
                        if TRY_DEPTH.with(|d| d.get()) >= 32 {
                            return Err(limit());
                        }
                        let nested = Output::new(
                            output.max.saturating_sub(output.value.len()),
                            sheet.text_output,
                        )
                        .with_serialization(output.serialization().0, None);
                        let try_frame = Frame {
                            focus: focus.clone(),
                            env: env.clone(),
                            depth: depth + 1,
                            mode: mode.clone(),
                            tunnel: tunnel.clone(),
                            rule: None,
                        };
                        TRY_DEPTH.with(|d| d.set(d.get() + 1));
                        let result = run_tasks(
                            run,
                            budget,
                            vec![Task::Body(try_body, 0, try_frame)],
                            nested,
                            None,
                        );
                        TRY_DEPTH.with(|d| d.set(d.get() - 1));
                        match result {
                            Ok(text) => output.splice(&text)?,
                            Err(error) => {
                                // Resource limits are not recoverable errors.
                                let Some(catch) = catches
                                    .iter()
                                    .find(|c| error.code != "XPDY0130" && catch_matches(c, &error))
                                else {
                                    return Err(error);
                                };
                                let (uri, local) = error_name(&error.code);
                                let mut inner = (*env).clone();
                                let name = |n: &str| {
                                    format!("Q{{{}}}{n}", crate::compile::ERROR_NAMESPACE)
                                };
                                inner.insert(
                                    name("code"),
                                    vec![Item::QName(crate::qname::AtomicQName::new(
                                        &uri, &local,
                                    )?)],
                                );
                                inner.insert(
                                    name("description"),
                                    vec![Item::String(error.message.clone())],
                                );
                                inner.insert(name("value"), vec![]);
                                inner.insert(name("module"), vec![]);
                                inner.insert(
                                    name("line-number"),
                                    vec![Item::Number(crate::number::Number::from(
                                        error.line as usize,
                                    ))],
                                );
                                inner.insert(
                                    name("column-number"),
                                    vec![Item::Number(crate::number::Number::from(
                                        error.column as usize,
                                    ))],
                                );
                                inner.insert(name("additional"), vec![]);
                                tasks.push(Task::Body(
                                    &catch.body,
                                    0,
                                    Frame {
                                        focus,
                                        env: Rc::new(inner),
                                        depth: depth + 1,
                                        mode,
                                        tunnel,
                                        rule,
                                    },
                                ));
                            }
                        }
                    }
                    Instruction::Iterate { select, params, .. } => {
                        let items = xpath::eval(select, &focus, &env, budget, 0, sheet.backwards)?;
                        let mut initial = Variables::new();
                        for p in params {
                            let BindingValue::Select(e) = &p.value else {
                                unreachable!()
                            };
                            let mut value =
                                xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                            if let Some(ty) = &p.ty {
                                value = ty.convert(value, "XTTE0570")?;
                            }
                            initial.insert(p.name.clone(), value);
                        }
                        tasks.push(Task::Iterate(
                            instruction,
                            Rc::new(items),
                            0,
                            initial,
                            Frame {
                                focus,
                                env,
                                depth,
                                mode,
                                tunnel,
                                rule,
                            },
                        ));
                    }
                    Instruction::NextIteration(supplied) => {
                        let Some(state) = iterations.last_mut() else {
                            return Err(Diagnostic::new(
                                "XTSE3120",
                                "xsl:next-iteration outside xsl:iterate",
                                "run",
                            ));
                        };
                        let mut next = Variables::new();
                        for declared in state.parameters {
                            next.insert(
                                declared.name.clone(),
                                env.get(&declared.name).cloned().unwrap_or_default(),
                            );
                        }
                        for p in supplied {
                            let BindingValue::Select(e) = &p.value else {
                                unreachable!()
                            };
                            let mut value =
                                xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                            if let Some(declared) =
                                state.parameters.iter().find(|d| d.name == p.name)
                            {
                                if let Some(ty) = &declared.ty {
                                    value = ty.convert(value, "XTTE0570")?;
                                }
                            } else {
                                return Err(Diagnostic::new(
                                    "XTSE3130",
                                    format!("unknown iteration parameter: {}", p.name),
                                    "run",
                                ));
                            }
                            next.insert(p.name.clone(), value);
                        }
                        state.next = Some(next);
                    }
                    Instruction::Break(content) => {
                        if let Some(state) = iterations.last_mut() {
                            state.broken = true;
                        }
                        tasks.push(Task::Body(
                            content,
                            0,
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                                tunnel,
                                rule,
                            },
                        ));
                    }
                    Instruction::AnalyzeString {
                        select,
                        regex,
                        flags,
                        matching,
                        non_matching,
                    } => {
                        let value = xpath::eval(select, &focus, &env, budget, 0, sheet.backwards)?;
                        let input = match value.as_slice() {
                            [] => String::new(),
                            [item] => xpath::atomize(item).string(),
                            _ => {
                                return Err(Diagnostic::new(
                                    "XTTE1540",
                                    "xsl:analyze-string requires a single string",
                                    "run",
                                ));
                            }
                        };
                        let pattern = parts(regex, &focus, &env, budget, sheet.backwards)?;
                        let flags = parts(flags, &focus, &env, budget, sheet.backwards)?;
                        let segments =
                            crate::regex_functions::analyze(&input, &pattern, &flags, budget)?;
                        for segment in segments.into_iter().rev() {
                            let (body, text, groups) = match segment {
                                crate::regex_functions::Segment::Match(groups) => {
                                    (matching, groups[0].clone(), groups)
                                }
                                crate::regex_functions::Segment::NonMatch(text) => {
                                    (non_matching, text, vec![])
                                }
                            };
                            let mut inner = (*env).clone();
                            inner.insert(
                                xpath::REGEX_GROUPS.into(),
                                groups.into_iter().map(Item::String).collect(),
                            );
                            tasks.push(Task::Body(
                                body,
                                0,
                                Frame {
                                    focus: Focus {
                                        item: Item::String(text),
                                        position: 1,
                                        size: 1,
                                    },
                                    env: Rc::new(inner),
                                    depth: depth + 1,
                                    mode: mode.clone(),
                                    tunnel: tunnel.clone(),
                                    rule: None,
                                },
                            ));
                        }
                    }
                    Instruction::UseSets(names) => {
                        fn expand<'s>(
                            sheet: &'s Stylesheet,
                            name: &str,
                            out: &mut Vec<&'s [Instruction]>,
                        ) {
                            if let Some(set) = sheet.attribute_sets.get(name) {
                                for used in &set.uses {
                                    expand(sheet, used, out);
                                }
                                out.push(&set.body);
                            }
                        }
                        let mut bodies = vec![];
                        for name in names {
                            expand(sheet, name, &mut bodies);
                        }
                        for set_body in bodies.into_iter().rev() {
                            tasks.push(Task::Body(
                                set_body,
                                0,
                                Frame {
                                    focus: focus.clone(),
                                    env: env.clone(),
                                    depth: depth + 1,
                                    mode: mode.clone(),
                                    tunnel: tunnel.clone(),
                                    rule: None,
                                },
                            ));
                        }
                    }
                    Instruction::LiteralAttributes(attributes) => {
                        for (name, parts) in attributes {
                            let mut value = String::new();
                            for part in parts {
                                let mut v =
                                    xpath::eval(part, &focus, &env, budget, 0, sheet.backwards)?;
                                if sheet.backwards {
                                    v.truncate(1);
                                }
                                value.push_str(&xpath::simple_string(
                                    &v,
                                    " ",
                                    usize::MAX,
                                    sheet.backwards,
                                )?);
                            }
                            output.attribute(name.clone(), value)?;
                        }
                    }
                    Instruction::ForEachGroup(e, grouping, sorts, body) => {
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        let groups = form_groups(value, grouping, &env, budget, sheet.backwards)?;
                        let mut scopes = vec![];
                        for (key, members) in &groups {
                            let mut inner = (*env).clone();
                            inner.insert(xpath::CURRENT_GROUP.into(), members.clone());
                            match key {
                                Some(key) => inner.insert(xpath::CURRENT_KEY.into(), key.clone()),
                                None => inner.remove(xpath::CURRENT_KEY),
                            };
                            scopes.push(inner);
                        }
                        let order = if sorts.is_empty() {
                            (0..groups.len()).collect()
                        } else {
                            let firsts =
                                groups.iter().map(|(_, m)| m[0].clone()).collect::<Vec<_>>();
                            crate::sort::order(
                                &firsts,
                                sorts,
                                &env,
                                Some(&scopes),
                                budget,
                                sheet.backwards,
                            )?
                        };
                        let size = groups.len();
                        let mut scopes: Vec<_> = scopes.into_iter().map(Some).collect();
                        let mut groups: Vec<_> = groups.into_iter().map(Some).collect();
                        for (index, source) in order.into_iter().enumerate().rev() {
                            let (_, members) = groups[source].take().unwrap();
                            let inner = scopes[source].take().unwrap();
                            let first = members[0].clone();
                            tasks.push(Task::Body(
                                body,
                                0,
                                Frame {
                                    focus: Focus {
                                        item: first,
                                        position: index + 1,
                                        size,
                                    },
                                    env: Rc::new(inner),
                                    depth: depth + 1,
                                    mode: mode.clone(),
                                    tunnel: tunnel.clone(),
                                    rule: None,
                                },
                            ));
                        }
                    }
                    Instruction::If(e, body) => {
                        if xpath::boolean(&xpath::eval(
                            e,
                            &focus,
                            &env,
                            budget,
                            0,
                            sheet.backwards,
                        )?)? {
                            tasks.push(Task::Body(
                                body,
                                0,
                                Frame {
                                    focus,
                                    env,
                                    depth: depth + 1,
                                    mode,
                                    tunnel: tunnel.clone(),
                                    rule,
                                },
                            ));
                        }
                    }
                    Instruction::Choose(branches) => {
                        for (test, body) in branches {
                            if test
                                .as_ref()
                                .map(|e| {
                                    xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)
                                        .and_then(|v| xpath::boolean(&v))
                                })
                                .transpose()?
                                .unwrap_or(true)
                            {
                                tasks.push(Task::Body(
                                    body,
                                    0,
                                    Frame {
                                        focus,
                                        env,
                                        depth: depth + 1,
                                        mode,
                                        tunnel: tunnel.clone(),
                                        rule,
                                    },
                                ));
                                break;
                            }
                        }
                    }
                    Instruction::NextMatch(params) => {
                        let before = rule.ok_or_else(|| {
                            Diagnostic::new(
                                "XTDE0560",
                                "next-match requires a current template rule",
                                "run",
                            )
                        })?;
                        tasks.push(Task::ApplyArguments(
                            vec![focus.clone()],
                            mode.clone(),
                            params,
                            0,
                            Supplied::inherited(tunnel.clone()),
                            Frame {
                                focus,
                                env,
                                depth,
                                mode,
                                tunnel,
                                rule,
                            },
                            Some(before),
                        ));
                    }
                    Instruction::ApplyImports(params) => {
                        let current = rule.ok_or_else(|| {
                            Diagnostic::new(
                                "XTDE0560",
                                "apply-imports requires a current template rule",
                                "run",
                            )
                        })?;
                        // Only rules of lower import precedence are candidates.
                        let before = Rule {
                            precedence: current.precedence,
                            template: 0,
                            branch: 0,
                            priority: f64::NEG_INFINITY,
                        };
                        tasks.push(Task::ApplyArguments(
                            vec![focus.clone()],
                            mode.clone(),
                            params,
                            0,
                            Supplied::inherited(tunnel.clone()),
                            Frame {
                                focus,
                                env,
                                depth,
                                mode,
                                tunnel,
                                rule,
                            },
                            Some(before),
                        ));
                    }
                    Instruction::Call(name, params) => {
                        let t = sheet
                            .templates
                            .iter()
                            .find(|t| t.name.as_ref() == Some(name))
                            .ok_or_else(|| {
                                Diagnostic::new(
                                    "XTSE0650",
                                    format!("unknown named template: {name}"),
                                    "run",
                                )
                            })?;
                        tasks.push(Task::Arguments(
                            t,
                            params,
                            0,
                            Supplied::inherited(tunnel.clone()),
                            Frame {
                                focus,
                                env,
                                depth,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                        ));
                    }
                    Instruction::Element {
                        element,
                        attributes,
                        body,
                    } => {
                        let mut element = element.clone();
                        for (name, parts) in attributes {
                            let mut value = String::new();
                            for part in parts {
                                let mut v =
                                    xpath::eval(part, &focus, &env, budget, 0, sheet.backwards)?;
                                if sheet.backwards {
                                    v.truncate(1);
                                }
                                value.push_str(&xpath::simple_string(
                                    &v,
                                    " ",
                                    usize::MAX,
                                    sheet.backwards,
                                )?);
                            }
                            element.attributes.push((name.clone(), value));
                        }
                        if raw_active(&captures) {
                            begin_raw_node(&mut output, &mut captures, &mut tasks, depth, false);
                        }
                        output.start(element)?;
                        tasks.push(Task::Close);
                        tasks.push(Task::Body(
                            body,
                            0,
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                                tunnel: tunnel.clone(),
                                rule,
                            },
                        ));
                    }
                }
            }
            Task::RawCopy(item, depth, namespaces) => match item {
                Item::Node(node) => {
                    if node.is_text() {
                        captures.last_mut().unwrap().items.extend(raw_text(
                            &node.string(),
                            trees,
                            limits.max_output_bytes,
                        )?);
                        continue;
                    }
                    if node.attribute.is_some() {
                        begin_raw_attribute(&mut output, &mut captures, &mut tasks, depth)?;
                        copied_leaf(node, &mut output)?;
                        continue;
                    }
                    begin_raw_node(
                        &mut output,
                        &mut captures,
                        &mut tasks,
                        depth,
                        node.is_document(),
                    );
                    tasks.push(Task::DeepCopy(node, depth, namespaces));
                }
                atomic => emit_value(vec![atomic], &mut tasks, depth, namespaces),
            },
            Task::DeepCopy(node, depth, namespaces) => {
                if output.simple_node(node)? {
                    continue;
                }
                if depth > limits.max_depth {
                    return Err(limit());
                }
                if node.is_document() || node.is_element() {
                    if node.is_element() {
                        let mut element = result::source_element(node.raw, true);
                        if !namespaces {
                            element.namespaces.clear();
                        }
                        output.start(element)?;
                        tasks.push(Task::Close);
                    }
                    for child in node.children().rev() {
                        tasks.push(Task::DeepCopy(child, depth + 1, namespaces));
                    }
                } else {
                    copied_leaf(node, &mut output)?;
                }
            }
            Task::FinishDocument(saved, depth) => {
                let xml = std::mem::replace(&mut output, saved).value;
                let value = trees.construct(xml, None, "XTTE0570")?;
                let Item::Node(node) = value.into_iter().next().unwrap() else {
                    unreachable!()
                };
                tasks.push(Task::DeepCopy(node, depth + 1, true));
            }
            Task::Value(item, depth, namespaces) => {
                if raw_active(&captures) {
                    captures.last_mut().unwrap().items.push(item);
                } else {
                    match item {
                        Item::Node(n) => tasks.push(Task::DeepCopy(n, depth + 1, namespaces)),
                        atomic => output.atomic(&atomic.string_compat(sheet.backwards))?,
                    }
                }
            }
            Task::FinishCapture(kind, saved) => {
                let value = std::mem::replace(&mut output, saved).finish_simple()?;
                finish_capture(kind, value, &mut output, budget)?;
            }
            Task::Close => output.end()?,
            Task::Iterate(instruction, items, index, params, frame) => {
                let Instruction::Iterate {
                    params: declared,
                    on_completion,
                    body,
                    ..
                } = instruction
                else {
                    unreachable!()
                };
                let mut env = (*frame.env).clone();
                env.extend(params.iter().map(|(k, v)| (k.clone(), v.clone())));
                if index == items.len() {
                    tasks.push(Task::Body(
                        on_completion,
                        0,
                        Frame {
                            focus: Focus {
                                item: Item::Absent,
                                position: 0,
                                size: 0,
                            },
                            env: Rc::new(env),
                            depth: frame.depth + 1,
                            mode: frame.mode.clone(),
                            tunnel: frame.tunnel.clone(),
                            rule: None,
                        },
                    ));
                    continue;
                }
                budget.tick()?;
                iterations.push(IterationState {
                    parameters: declared,
                    next: None,
                    broken: false,
                });
                tasks.push(Task::IterateNext(
                    instruction,
                    items.clone(),
                    index,
                    params,
                    frame.clone(),
                ));
                tasks.push(Task::Body(
                    body,
                    0,
                    Frame {
                        focus: Focus {
                            item: items[index].clone(),
                            position: index + 1,
                            size: items.len(),
                        },
                        env: Rc::new(env),
                        depth: frame.depth + 1,
                        mode: frame.mode.clone(),
                        tunnel: frame.tunnel.clone(),
                        rule: None,
                    },
                ));
            }
            Task::IterateNext(instruction, items, index, params, frame) => {
                let state = iterations.pop().expect("iteration state");
                if !state.broken {
                    tasks.push(Task::Iterate(
                        instruction,
                        items,
                        index + 1,
                        state.next.unwrap_or(params),
                        frame,
                    ));
                }
            }
        }
    }
    if let Some(result) = raw_result {
        *result = captures.pop().unwrap().items;
    }
    Ok(output.value)
}
