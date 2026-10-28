use crate::{
    Diagnostic, Stylesheet, TransformOptions,
    compile::{Binding, BindingValue, ComputedName, Content, Instruction, Template},
    result::{self, Output},
    xpath::{self, Budget, Focus, Item, Value, Variables, XNode},
};
use std::{collections::BTreeMap, rc::Rc};
fn limit() -> Diagnostic {
    Diagnostic::new("XPDY0130", "execution depth limit exceeded", "run")
}
#[derive(Clone)]
struct Frame<'a> {
    focus: Focus<'a>,
    env: Rc<Variables<'a>>,
    depth: usize,
    mode: String,
}
enum Task<'a> {
    Apply(Focus<'a>, usize, String, Rc<Variables<'a>>),
    ApplyArguments(
        Vec<Focus<'a>>,
        String,
        &'a [Binding],
        usize,
        Variables<'a>,
        Frame<'a>,
    ),
    Body(&'a [Instruction], usize, Frame<'a>),
    Enter(&'a Template, Focus<'a>, Variables<'a>, usize, String),
    DeepCopy(XNode<'a>, usize, bool),
    FinishDocument(Output, usize),
    Atomic(String),
    Close,
    FinishCapture(Capture, Output),
    Parameters(&'a Template, usize, Variables<'a>, Frame<'a>),
    Arguments(&'a Template, &'a [Binding], usize, Variables<'a>, Frame<'a>),
    EvaluateBinding(&'a Binding, Frame<'a>, BindNext<'a>),
    FinishBinding(&'a Binding, Frame<'a>, BindNext<'a>, Output),
}
enum BindNext<'a> {
    Local(&'a [Instruction], usize),
    Parameter(&'a Template, usize, Variables<'a>),
    Argument(&'a Template, &'a [Binding], usize, Variables<'a>),
    ApplyArgument(Vec<Focus<'a>>, String, &'a [Binding], usize, Variables<'a>),
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
        BindNext::ApplyArgument(foci, mode, params, index, mut args) => {
            args.insert(binding.name.clone(), value);
            tasks.push(Task::ApplyArguments(foci, mode, params, index, args, frame));
        }
        BindNext::Argument(t, params, index, mut args) => {
            args.insert(binding.name.clone(), value);
            tasks.push(Task::Arguments(t, params, index, args, frame));
        }
    }
    Ok(())
}
struct Run<'a, 'r> {
    sheet: &'a Stylesheet,
    options: &'r TransformOptions,
    trees: &'r crate::trees::Trees<'a>,
    globals: Rc<Variables<'a>>,
}
enum Capture {
    Attribute(result::Name),
    Comment,
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
        text.push_str(&xpath::simple_string(&v, " ", usize::MAX)?);
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
fn finish_capture(kind: Capture, value: String, output: &mut Output) -> Result<(), Diagnostic> {
    match kind {
        Capture::Attribute(name) => output.attribute(name, value),
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
                xpath::simple_string(&v, &sep, output.max.saturating_sub(output.value.len()))?,
                output,
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
fn execute<'a>(
    sheet: &'a Stylesheet,
    root: Focus<'a>,
    options: &TransformOptions,
    budget: &mut Budget,
    declaration: bool,
    trees: &crate::trees::Trees<'a>,
) -> Result<String, Diagnostic> {
    let parameters = options
        .parameters
        .iter()
        .map(|(name, value)| Ok((crate::qname::external(name)?, value)))
        .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?;
    let limits = options.limits;
    let mut globals = Variables::new();
    let mut pending: Vec<_> = sheet.globals.iter().collect();
    while !pending.is_empty() {
        let before = pending.len();
        let mut remaining = vec![];
        for b in pending {
            budget.tick()?;
            if let Some(s) = parameters.get(&b.name).filter(|_| b.parameter) {
                let mut v = vec![Item::String((*s).clone())];
                if let Some(ty) = &b.ty {
                    v = ty.convert(v, "XTTE0590")?;
                }
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
            let evaluated = match &b.value {
                BindingValue::Select(e) => {
                    xpath::eval(e, &root, &globals, budget, 0, sheet.backwards)
                }
                BindingValue::Body(body) | BindingValue::Document(body) => {
                    let env = Rc::new(globals.clone());
                    let frame = Frame {
                        focus: root.clone(),
                        env: env.clone(),
                        depth: 0,
                        mode: String::new(),
                    };
                    let run = Run {
                        sheet,
                        options,
                        trees,
                        globals: env,
                    };
                    run_tasks(
                        &run,
                        budget,
                        vec![Task::Body(body, 0, frame)],
                        Output::document(limits.max_output_bytes),
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
            };
            match evaluated {
                Ok(mut v) => {
                    if let Some(ty) = &b.ty {
                        v = ty.convert(v, if b.parameter { "XTTE0600" } else { "XTTE0570" })?;
                    }
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
    let mut output = Output::new(limits.max_output_bytes, sheet.text_output);
    if declaration && !sheet.text_output && !sheet.omit_declaration {
        output.push("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    }
    let initial_mode = options
        .initial_mode
        .as_deref()
        .map(crate::qname::external)
        .transpose()?
        .unwrap_or_default();
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
        vec![Task::Enter(t, root, Variables::new(), 0, initial_mode)]
    } else {
        vec![Task::Apply(
            root,
            0,
            initial_mode,
            Rc::new(Variables::new()),
        )]
    };
    let run = Run {
        sheet,
        options,
        trees,
        globals,
    };
    run_tasks(&run, budget, tasks, output)
}
fn run_tasks<'a>(
    run: &Run<'a, '_>,
    budget: &mut Budget,
    mut tasks: Vec<Task<'a>>,
    mut output: Output,
) -> Result<String, Diagnostic> {
    let sheet = run.sheet;
    let limits = run.options.limits;
    let globals = run.globals.clone();
    let trees = run.trees;
    while let Some(task) = tasks.pop() {
        budget.tick()?;
        match task {
            Task::Enter(t, focus, params, depth, mode) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                tasks.push(Task::Parameters(
                    t,
                    0,
                    params,
                    Frame {
                        focus,
                        env: Rc::new((*globals).clone()),
                        depth,
                        mode,
                    },
                ));
            }
            Task::Parameters(t, index, mut supplied, mut frame) => {
                if let Some(p) = t.params.get(index) {
                    if let Some(mut value) = supplied.remove(&p.name) {
                        if let Some(ty) = &p.ty {
                            value = ty.convert(value, "XTTE0590")?;
                        }
                        Rc::make_mut(&mut frame.env).insert(p.name.clone(), value);
                        tasks.push(Task::Parameters(t, index + 1, supplied, frame));
                    } else {
                        if p.required {
                            return Err(Diagnostic::new(
                                "XTDE0700",
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
                } else {
                    tasks.push(Task::Body(&t.body, 0, frame));
                }
            }
            Task::ApplyArguments(foci, mode, params, index, args, frame) => {
                if let Some(p) = params.get(index) {
                    tasks.push(Task::EvaluateBinding(
                        p,
                        frame,
                        BindNext::ApplyArgument(foci, mode, params, index + 1, args),
                    ));
                } else {
                    let args = Rc::new(args);
                    for focus in foci.into_iter().rev() {
                        tasks.push(Task::Apply(
                            focus,
                            frame.depth + 1,
                            mode.clone(),
                            args.clone(),
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
                    ));
                }
            }
            Task::EvaluateBinding(binding, frame, next) => match &binding.value {
                BindingValue::Select(e) => {
                    let value =
                        xpath::eval(e, &frame.focus, &frame.env, budget, 0, sheet.backwards)?;
                    complete_binding(binding, value, frame, next, &mut tasks)?;
                }
                BindingValue::Body(body) | BindingValue::Document(body) => {
                    let saved =
                        std::mem::replace(&mut output, Output::document(limits.max_output_bytes));
                    let mut constructor = frame.clone();
                    constructor.depth += 1;
                    tasks.push(Task::FinishBinding(binding, frame, next, saved));
                    tasks.push(Task::Body(body, 0, constructor));
                }
            },
            Task::FinishBinding(binding, frame, next, saved) => {
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
                complete_binding(binding, value, frame, next, &mut tasks)?;
            }
            Task::Apply(focus, depth, mode, params) => {
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
                let mut best: Option<(&Template, f64)> = None;
                for t in &sheet.templates {
                    budget.tick()?;
                    if !t.modes.iter().any(|m| m == &mode || m == "#all") {
                        continue;
                    }
                    if let Some(branches) = &t.pattern {
                        for b in branches {
                            let priority = t.priority.unwrap_or(b.priority);
                            if best.as_ref().is_none_or(|(_, p)| priority >= *p)
                                && crate::pattern::matches(
                                    b,
                                    node,
                                    &globals,
                                    budget,
                                    sheet.backwards,
                                )?
                            {
                                best = Some((t, priority));
                            }
                        }
                    }
                }
                if let Some((t, _)) = best {
                    tasks.push(Task::Enter(t, focus, (*params).clone(), depth, mode));
                } else {
                    if node.is_document() || node.is_element() {
                        let children = node.children().map(Item::Node).collect();
                        for f in focus_sequence(children).into_iter().rev() {
                            tasks.push(Task::Apply(
                                f,
                                depth + 1,
                                mode.clone(),
                                if sheet.backwards {
                                    Rc::new(Variables::new())
                                } else {
                                    params.clone()
                                },
                            ));
                        }
                    } else if node.is_text() || node.is_attribute() {
                        output.text(&node.string())?;
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
                tasks.push(Task::Body(body, i + 1, frame.clone()));
                let Frame {
                    focus,
                    env,
                    depth,
                    mode,
                } = frame;
                match instruction {
                    Instruction::ComputedElement(n, inherit, body) => {
                        let frame = Frame {
                            focus,
                            env,
                            depth: depth + 1,
                            mode,
                        };
                        let name = computed_name(n, false, &frame, budget, sheet.backwards)?;
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
                        };
                        let name = computed_name(n, true, &frame, budget, sheet.backwards)?;
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
                    Instruction::Comment(content) => capture(
                        content,
                        Capture::Comment,
                        Frame {
                            focus,
                            env,
                            depth: depth + 1,
                            mode,
                        },
                        sheet,
                        budget,
                        &mut output,
                        &mut tasks,
                    )?,
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
                        capture(
                            content,
                            Capture::Pi(name),
                            Frame {
                                focus,
                                env,
                                depth: depth + 1,
                                mode,
                            },
                            sheet,
                            budget,
                            &mut output,
                            &mut tasks,
                        )?;
                    }
                    Instruction::Bind(_) => unreachable!(),
                    Instruction::Text(t) => output.text(t)?,
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
                        output.text(&xpath::simple_string(
                            &v,
                            &separator,
                            output.max.saturating_sub(output.value.len()),
                        )?)?;
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
                            Variables::new(),
                            Frame {
                                focus,
                                env,
                                depth,
                                mode,
                            },
                        ));
                    }
                    Instruction::CopyOf(e, _) | Instruction::Sequence(e) => {
                        let namespaces = match instruction {
                            Instruction::CopyOf(_, value) => *value,
                            _ => true,
                        };
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        // Copy atomics as text with a separator only between adjacent atomics.
                        let mut operations = vec![];
                        for item in value {
                            match item {
                                Item::Node(n) => {
                                    operations.push(Ok(n));
                                }
                                _ => {
                                    operations.push(Err(item.string()));
                                }
                            }
                        }
                        // Text operations need their own tasks to preserve mixed sequence order.
                        for op in operations.into_iter().rev() {
                            match op {
                                Ok(n) => tasks.push(Task::DeepCopy(n, depth + 1, namespaces)),
                                Err(s) => tasks.push(Task::Atomic(s)),
                            }
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
                            atomic => output.atomic(&atomic.string())?,
                        }
                    }
                    Instruction::Document(body) => {
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
                                    },
                                ));
                                break;
                            }
                        }
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
                            Variables::new(),
                            Frame {
                                focus,
                                env,
                                depth,
                                mode,
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
                                value.push_str(&xpath::simple_string(&v, " ", usize::MAX)?);
                            }
                            element.attributes.push((name.clone(), value));
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
                            },
                        ));
                    }
                }
            }
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
            Task::Atomic(s) => output.atomic(&s)?,
            Task::FinishCapture(kind, saved) => {
                let value = std::mem::replace(&mut output, saved).finish_simple()?;
                finish_capture(kind, value, &mut output)?;
            }
            Task::Close => output.end()?,
        }
    }
    Ok(output.value)
}
