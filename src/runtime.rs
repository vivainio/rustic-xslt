use crate::{
    Diagnostic, Stylesheet, TransformOptions,
    compile::{ComputedName, Content, Instruction, Template},
    result::{self, Output},
    xpath::{self, Budget, Focus, Item, Value, Variables, XNode},
};
use std::rc::Rc;
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
    Apply(Focus<'a>, usize, String),
    Body(&'a [Instruction], usize, Frame<'a>),
    Enter(&'a Template, Focus<'a>, Variables<'a>, usize, String),
    DeepCopy(XNode<'a>, usize),
    Atomic(String),
    Close,
    FinishCapture(Capture, Output),
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
        text.push_str(&xpath::string(&v, " "));
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
            finish_capture(kind, xpath::string(&v, &sep), output)
        }
        Content::Body(body) => {
            let captured = Output::new(output.max.saturating_sub(output.value.len()), true);
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
    let absent = source.is_empty() && options.initial_template.is_some();
    let doc = crate::parse(if absent { "<_absent/>" } else { source }, "parse")?;
    let root = Focus {
        item: if absent {
            Item::Absent
        } else {
            Item::Node(XNode::Tree(doc.root()))
        },
        position: 1,
        size: 1,
    };
    execute(sheet, root, options, &mut Budget::new(options.limits), true)
}
pub(crate) fn transform_record(
    sheet: &Stylesheet,
    source: &str,
    options: &TransformOptions,
    budget: &mut Budget,
    position: usize,
) -> Result<String, Diagnostic> {
    let doc = crate::parse(source, "parse")?;
    let root = Focus {
        item: Item::Node(XNode::Tree(doc.root_element())),
        position,
        size: 1,
    };
    execute(sheet, root, options, budget, false)
}
fn copied_leaf(node: XNode<'_>, output: &mut Output) -> Result<(), Diagnostic> {
    match node {
        XNode::Attribute(n, name, uri) => {
            output.attribute(result::attribute_name(n, name, uri), node.string())
        }
        XNode::Tree(n) if n.is_text() => output.text(n.text().unwrap_or_default()),
        XNode::Tree(n) if n.is_comment() => output.comment(n.text().unwrap_or_default()),
        XNode::Tree(n) if n.is_pi() => {
            let pi = n.pi().unwrap();
            output.pi(pi.target, pi.value.unwrap_or_default())
        }
        _ => Ok(()),
    }
}
fn execute<'a>(
    sheet: &'a Stylesheet,
    root: Focus<'a>,
    options: &TransformOptions,
    budget: &mut Budget,
    declaration: bool,
) -> Result<String, Diagnostic> {
    let limits = options.limits;
    let mut globals = Variables::new();
    let mut pending: Vec<_> = sheet.globals.iter().collect();
    while !pending.is_empty() {
        let before = pending.len();
        let mut remaining = vec![];
        for b in pending {
            budget.tick()?;
            if let Some(s) = options.parameters.get(&b.name).filter(|_| b.parameter) {
                globals.insert(b.name.clone(), vec![Item::String(s.clone())]);
                continue;
            }
            match xpath::eval(&b.value, &root, &globals, budget, 0, sheet.backwards) {
                Ok(v) => {
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
    let initial_mode = options.initial_mode.clone().unwrap_or_default();
    let mut tasks = if let Some(name) = &options.initial_template {
        let t = sheet
            .templates
            .iter()
            .find(|t| t.name.as_ref() == Some(name))
            .ok_or_else(|| {
                Diagnostic::new(
                    "XTDE0040",
                    format!("unknown initial template: {name}"),
                    "run",
                )
            })?;
        vec![Task::Enter(t, root, Variables::new(), 0, initial_mode)]
    } else {
        vec![Task::Apply(root, 0, initial_mode)]
    };
    while let Some(task) = tasks.pop() {
        budget.tick()?;
        match task {
            Task::Enter(t, focus, mut params, depth, mode) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                let mut env = (*globals).clone();
                for p in &t.params {
                    let v = if let Some(v) = params.remove(&p.name) {
                        v
                    } else {
                        xpath::eval(&p.value, &focus, &env, budget, 0, sheet.backwards)?
                    };
                    env.insert(p.name.clone(), v);
                }
                tasks.push(Task::Body(
                    &t.body,
                    0,
                    Frame {
                        focus,
                        env: Rc::new(env),
                        depth,
                        mode,
                    },
                ));
            }
            Task::Apply(focus, depth, mode) => {
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
                    tasks.push(Task::Enter(t, focus, Variables::new(), depth, mode));
                } else {
                    match node {
                        XNode::Tree(n) if n.is_root() || n.is_element() => {
                            let children =
                                n.children().map(|n| Item::Node(XNode::Tree(n))).collect();
                            for f in focus_sequence(children).into_iter().rev() {
                                tasks.push(Task::Apply(f, depth + 1, mode.clone()));
                            }
                        }
                        XNode::Tree(n) if n.is_text() => {
                            output.text(n.text().unwrap_or_default())?
                        }
                        XNode::Attribute(..) => output.text(&node.string())?,
                        _ => {}
                    }
                }
            }
            Task::Body(body, i, mut frame) => {
                if frame.depth > limits.max_depth {
                    return Err(limit());
                }
                let Some(instruction) = body.get(i) else {
                    continue;
                };
                if let Instruction::Bind(b) = instruction {
                    let value = xpath::eval(
                        &b.value,
                        &frame.focus,
                        &frame.env,
                        budget,
                        0,
                        sheet.backwards,
                    )?;
                    Rc::make_mut(&mut frame.env).insert(b.name.clone(), value);
                    tasks.push(Task::Body(body, i + 1, frame));
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
                    Instruction::ComputedElement(n, body) => {
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
                        for (i, item) in v.iter().enumerate() {
                            budget.tick()?;
                            if i > 0 {
                                output.text(&separator)?;
                            }
                            output.text(&item.string())?;
                        }
                    }
                    Instruction::Apply(e, target_mode, sorts) => {
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        let value = crate::sort::sort(value, sorts, &env, budget, sheet.backwards)?;
                        let target_mode = if target_mode == "#current" {
                            mode
                        } else {
                            target_mode.clone()
                        };
                        for f in focus_sequence(value).into_iter().rev() {
                            tasks.push(Task::Apply(f, depth + 1, target_mode.clone()));
                        }
                    }
                    Instruction::CopyOf(e) => {
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        // Copy atomics as text with a separator only between adjacent atomics.
                        let mut operations = vec![];
                        let mut atomic = false;
                        for item in value {
                            match item {
                                Item::Node(n) => {
                                    operations.push(Ok(n));
                                    atomic = false;
                                }
                                _ => {
                                    let mut s = item.string();
                                    if atomic {
                                        s.insert(0, ' ');
                                    }
                                    operations.push(Err(s));
                                    atomic = true;
                                }
                            }
                        }
                        // Text operations need their own tasks to preserve mixed sequence order.
                        for op in operations.into_iter().rev() {
                            match op {
                                Ok(n) => tasks.push(Task::DeepCopy(n, depth + 1)),
                                Err(s) => tasks.push(Task::Atomic(s)),
                            }
                        }
                    }
                    Instruction::Copy(body) => match focus.item.clone() {
                        Item::Node(XNode::Tree(n)) if n.is_element() => {
                            output.start(result::source_element(n, false))?;
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
                        Item::Node(XNode::Tree(n)) if n.is_root() => tasks.push(Task::Body(
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
                        atomic => output.text(&atomic.string())?,
                    },
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
                        let mut bindings = Variables::new();
                        for p in params {
                            bindings.insert(
                                p.name.clone(),
                                xpath::eval(&p.value, &focus, &env, budget, 0, sheet.backwards)?,
                            );
                        }
                        tasks.push(Task::Enter(t, focus, bindings, depth + 1, mode));
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
                                value.push_str(&xpath::string(&v, " "));
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
            Task::DeepCopy(node, depth) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                match node {
                    XNode::Tree(n) if n.is_root() || n.is_element() => {
                        if n.is_element() {
                            output.start(result::source_element(n, true))?;
                            tasks.push(Task::Close);
                        }
                        for child in n.children().rev() {
                            tasks.push(Task::DeepCopy(XNode::Tree(child), depth + 1));
                        }
                    }
                    _ => copied_leaf(node, &mut output)?,
                }
            }
            Task::Atomic(s) => output.text(&s)?,
            Task::FinishCapture(kind, saved) => {
                let value = std::mem::replace(&mut output, saved).value;
                finish_capture(kind, value, &mut output)?;
            }
            Task::Close => output.end()?,
        }
    }
    Ok(output.value)
}
