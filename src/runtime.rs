use crate::{
    Diagnostic, Limits, Stylesheet,
    compile::{Instruction, Select},
};
use roxmltree::Node;

fn evaluate<'a>(s: &Select, node: Node<'a, 'a>) -> Vec<Node<'a, 'a>> {
    match s {
        Select::SelfNode => vec![node],
        Select::Children => node.children().collect(),
        Select::Path { absolute, names } => {
            let mut nodes = vec![if *absolute {
                node.document().root()
            } else {
                node
            }];
            for name in names {
                nodes = nodes
                    .into_iter()
                    .flat_map(|n| n.children())
                    .filter(|n| {
                        n.is_element()
                            && n.tag_name().namespace().is_none()
                            && n.tag_name().name() == name
                    })
                    .collect();
            }
            nodes
        }
    }
}
fn priority(pattern: &str) -> i32 {
    if pattern.contains('/') { 1 } else { 0 }
}
fn matches(pattern: &str, node: Node<'_, '_>) -> bool {
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
}
impl Output {
    fn push(&mut self, s: &str) -> Result<(), Diagnostic> {
        if s.len() > self.max.saturating_sub(self.value.len()) {
            return Err(limit());
        }
        self.value.push_str(s);
        Ok(())
    }
}
enum Task<'a> {
    Apply(Node<'a, 'a>, usize),
    Body(&'a [Instruction], usize, Node<'a, 'a>, usize),
    Close(&'a str),
}
pub(crate) fn transform(
    sheet: &Stylesheet,
    source: &str,
    limits: Limits,
) -> Result<String, Diagnostic> {
    let doc = crate::parse(source, "parse")?;
    let mut output = Output {
        value: String::new(),
        max: limits.max_output_bytes,
    };
    if !sheet.text_output && !sheet.omit_declaration {
        output.push("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    }
    let mut tasks = vec![Task::Apply(doc.root(), 0)];
    let mut steps = 0_u64;
    while let Some(task) = tasks.pop() {
        if steps >= limits.max_steps {
            return Err(limit());
        }
        steps += 1;
        match task {
            Task::Apply(node, depth) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                let template = sheet
                    .templates
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| matches(&t.pattern, node))
                    .max_by_key(|(i, t)| (priority(&t.pattern), *i))
                    .map(|(_, t)| t);
                if let Some(t) = template {
                    tasks.push(Task::Body(&t.body, 0, node, depth));
                } else if node.is_root() || node.is_element() {
                    for child in node.children().rev() {
                        tasks.push(Task::Apply(child, depth + 1));
                    }
                } else if node.is_text() {
                    let text = node.text().unwrap_or_default();
                    output.push(&if sheet.text_output {
                        text.into()
                    } else {
                        escaped(text, false)
                    })?;
                }
            }
            Task::Body(body, i, node, depth) => {
                if depth > limits.max_depth {
                    return Err(limit());
                }
                let Some(instruction) = body.get(i) else {
                    continue;
                };
                tasks.push(Task::Body(body, i + 1, node, depth));
                match instruction {
                    Instruction::Text(t) => output.push(&if sheet.text_output {
                        t.clone()
                    } else {
                        escaped(t, false)
                    })?,
                    Instruction::Value(s) => {
                        for (i, n) in evaluate(s, node).into_iter().enumerate() {
                            if i > 0 {
                                output.push(" ")?;
                            }
                            for text in n.descendants().filter(|n| n.is_text()) {
                                let t = text.text().unwrap_or_default();
                                output.push(&if sheet.text_output {
                                    t.into()
                                } else {
                                    escaped(t, false)
                                })?;
                            }
                        }
                    }
                    Instruction::Apply(s) => {
                        for n in evaluate(s, node).into_iter().rev() {
                            tasks.push(Task::Apply(n, depth + 1));
                        }
                    }
                    Instruction::Element {
                        name,
                        attributes,
                        body,
                    } => {
                        if !sheet.text_output {
                            output.push("<")?;
                            output.push(name)?;
                            for (name, value) in attributes {
                                output.push(" ")?;
                                output.push(name)?;
                                output.push("=\"")?;
                                output.push(&escaped(value, true))?;
                                output.push("\"")?;
                            }
                            output.push(">")?;
                            tasks.push(Task::Close(name));
                        }
                        tasks.push(Task::Body(body, 0, node, depth + 1));
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
