use crate::{
    Diagnostic, Stylesheet, TransformOptions,
    compile::{Instruction, Template},
    xpath::{self, Budget, Focus, Item, Value, Variables, XNode},
};
use std::rc::Rc;

fn priority(pattern: &str) -> i32 {
    if pattern.contains('/') { 1 } else { 0 }
}
fn matches(pattern: &str, node: XNode<'_>) -> bool {
    let XNode::Tree(node) = node else {
        return false;
    };
    if pattern == "/" {
        return node.is_root();
    }
    let mut current = Some(node);
    for part in pattern.trim_start_matches('/').rsplit('/') {
        let Some(n) = current else {
            return false;
        };
        if !n.is_element() || n.tag_name().namespace().is_some() || n.tag_name().name() != part {
            return false;
        }
        current = n.parent();
    }
    !pattern.starts_with('/') || current.is_some_and(|n| n.is_root())
}
fn escaped(value: &str, attr: bool) -> String {
    let mut s = String::new();
    for c in value.chars() {
        s.push_str(match c {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' if attr => "&quot;",
            '\n' if attr => "&#10;",
            '\r' => "&#13;",
            '\t' if attr => "&#9;",
            _ => {
                s.push(c);
                continue;
            }
        });
    }
    s
}
fn limit() -> Diagnostic {
    Diagnostic::new("XPDY0130", "execution or output limit exceeded", "run")
}
struct Output {
    value: String,
    max: usize,
    text: bool,
}
impl Output {
    fn push(&mut self, s: &str) -> Result<(), Diagnostic> {
        if s.len() > self.max.saturating_sub(self.value.len()) {
            return Err(limit());
        }
        self.value.push_str(s);
        Ok(())
    }
    fn text(&mut self, s: &str) -> Result<(), Diagnostic> {
        if self.text {
            self.push(s)
        } else {
            self.push(&escaped(s, false))
        }
    }
}
enum Task<'a> {
    Apply(Focus<'a>, usize),
    Body(
        &'a [Instruction],
        usize,
        Focus<'a>,
        Rc<Variables<'a>>,
        usize,
    ),
    Enter(&'a Template, Focus<'a>, Variables<'a>, usize),
    Close(&'a str),
}
fn node_focus(n: XNode<'_>) -> Focus<'_> {
    Focus {
        item: Item::Node(n),
        position: 1,
        size: 1,
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
    let doc = crate::parse(source, "parse")?;
    let root = node_focus(XNode::Tree(doc.root()));
    let mut budget = Budget::new(options.limits);
    execute(sheet, root, options, &mut budget, true)
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
                "unresolved or circular global variable dependency",
                "run",
            ));
        }
        pending = remaining;
    }
    let globals = Rc::new(globals);
    let mut output = Output {
        value: String::new(),
        max: limits.max_output_bytes,
        text: sheet.text_output,
    };
    if declaration && !sheet.text_output && !sheet.omit_declaration {
        output.push("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    }
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
        vec![Task::Enter(t, root, Variables::new(), 0)]
    } else {
        vec![Task::Apply(root, 0)]
    };
    while let Some(task) = tasks.pop() {
        budget.tick()?;
        match task {
            Task::Enter(t, focus, mut params, depth) => {
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
                tasks.push(Task::Body(&t.body, 0, focus, Rc::new(env), depth));
            }
            Task::Apply(focus, depth) => {
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
                let template = sheet
                    .templates
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.pattern.as_ref().is_some_and(|p| matches(p, node)))
                    .max_by_key(|(i, t)| (priority(t.pattern.as_ref().unwrap()), *i))
                    .map(|(_, t)| t);
                if let Some(t) = template {
                    tasks.push(Task::Enter(t, focus, Variables::new(), depth));
                } else {
                    match node {
                        XNode::Tree(n) if n.is_root() || n.is_element() => {
                            let children =
                                n.children().map(|n| Item::Node(XNode::Tree(n))).collect();
                            for f in focus_sequence(children).into_iter().rev() {
                                tasks.push(Task::Apply(f, depth + 1));
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
            Task::Body(body, i, focus, mut env, depth) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                let Some(instruction) = body.get(i) else {
                    continue;
                };
                if let Instruction::Bind(b) = instruction {
                    let value = xpath::eval(&b.value, &focus, &env, budget, 0, sheet.backwards)?;
                    Rc::make_mut(&mut env).insert(b.name.clone(), value);
                    tasks.push(Task::Body(body, i + 1, focus, env, depth));
                    continue;
                }
                tasks.push(Task::Body(body, i + 1, focus.clone(), env.clone(), depth));
                match instruction {
                    Instruction::Bind(_) => unreachable!(),
                    Instruction::Text(t) => output.text(t)?,
                    Instruction::Value(e) => {
                        let mut v = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        if sheet.backwards {
                            v.truncate(1);
                        }
                        for (i, item) in v.iter().enumerate() {
                            budget.tick()?;
                            if i > 0 {
                                output.text(" ")?;
                            }
                            output.text(&item.string())?;
                        }
                    }
                    Instruction::Apply(e) => {
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        for f in focus_sequence(value).into_iter().rev() {
                            tasks.push(Task::Apply(f, depth + 1));
                        }
                    }
                    Instruction::ForEach(e, body) => {
                        let value = xpath::eval(e, &focus, &env, budget, 0, sheet.backwards)?;
                        for f in focus_sequence(value).into_iter().rev() {
                            tasks.push(Task::Body(body, 0, f, env.clone(), depth + 1));
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
                            tasks.push(Task::Body(body, 0, focus, env, depth + 1));
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
                                tasks.push(Task::Body(body, 0, focus, env, depth + 1));
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
                        tasks.push(Task::Enter(t, focus, bindings, depth + 1));
                    }
                    Instruction::Element {
                        name,
                        attributes,
                        body,
                    } => {
                        if !sheet.text_output {
                            output.push("<")?;
                            output.push(name)?;
                            for (name, parts) in attributes {
                                output.push(" ")?;
                                output.push(name)?;
                                output.push("=\"")?;
                                for part in parts {
                                    let mut v = xpath::eval(
                                        part,
                                        &focus,
                                        &env,
                                        budget,
                                        0,
                                        sheet.backwards,
                                    )?;
                                    if sheet.backwards {
                                        v.truncate(1);
                                    }
                                    output.push(&escaped(&xpath::string(&v, " "), true))?;
                                }
                                output.push("\"")?;
                            }
                            output.push(">")?;
                            tasks.push(Task::Close(name));
                        } else {
                            // Attribute expressions are still evaluated for their dynamic errors.
                            for (_, parts) in attributes {
                                for part in parts {
                                    xpath::eval(part, &focus, &env, budget, 0, sheet.backwards)?;
                                }
                            }
                        }
                        tasks.push(Task::Body(body, 0, focus, env, depth + 1));
                    }
                }
            }
            Task::Close(name) => {
                output.push("</")?;
                output.push(name)?;
                output.push(">")?;
            }
        }
    }
    Ok(output.value)
}
