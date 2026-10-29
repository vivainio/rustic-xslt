//! `indent="yes"`: re-flow serialized XML so that element-only content is placed
//! on separate indented lines. Elements with text children are left untouched.

enum Token<'a> {
    Start {
        text: &'a str,
        empty: bool,
    },
    End(&'a str),
    Text(&'a str),
    /// Comments, processing instructions, CDATA sections and the XML declaration.
    Markup(&'a str),
}

fn tokenize(xml: &str) -> Vec<Token<'_>> {
    let bytes = xml.as_bytes();
    let mut tokens = vec![];
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            let end = xml[i..].find('<').map_or(xml.len(), |p| i + p);
            tokens.push(Token::Text(&xml[i..end]));
            i = end;
            continue;
        }
        let rest = &xml[i..];
        let (len, kind) = if rest.starts_with("<!--") {
            (rest.find("-->").map_or(rest.len(), |p| p + 3), 0)
        } else if rest.starts_with("<![CDATA[") {
            (rest.find("]]>").map_or(rest.len(), |p| p + 3), 2)
        } else if rest.starts_with("<?") {
            (rest.find("?>").map_or(rest.len(), |p| p + 2), 0)
        } else {
            // A tag: find the closing '>' outside quoted attribute values.
            let mut quote = None;
            let mut end = rest.len();
            for (offset, c) in rest.char_indices().skip(1) {
                match (quote, c) {
                    (None, '"' | '\'') => quote = Some(c),
                    (Some(q), c) if c == q => quote = None,
                    (None, '>') => {
                        end = offset + 1;
                        break;
                    }
                    _ => {}
                }
            }
            (end, 1)
        };
        let text = &rest[..len];
        tokens.push(match kind {
            1 if text.starts_with("</") => Token::End(text),
            1 => Token::Start {
                text,
                empty: text.ends_with("/>"),
            },
            2 => Token::Text(text),
            _ => Token::Markup(text),
        });
        i += len;
    }
    tokens
}

pub(crate) fn indent(xml: &str) -> String {
    let tokens = tokenize(xml);
    // First pass: which elements have character content?
    let mut mixed = vec![false; tokens.len()];
    let mut open: Vec<usize> = vec![];
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Start { empty: false, .. } => open.push(index),
            Token::End(_) => {
                open.pop();
            }
            Token::Text(text) if !text.trim_matches([' ', '\t', '\r', '\n']).is_empty() => {
                if let Some(parent) = open.last() {
                    mixed[*parent] = true;
                }
            }
            _ => {}
        }
    }
    let mut out = String::with_capacity(xml.len() + xml.len() / 4);
    // (is mixed, has child markup) per open element
    let mut stack: Vec<(bool, bool)> = vec![];
    let newline = |out: &mut String, depth: usize| {
        out.push('\n');
        for _ in 0..depth {
            out.push_str("  ");
        }
    };
    let mut started = false;
    for (index, token) in tokens.iter().enumerate() {
        let parent_flat = stack.last().is_some_and(|(mixed, _)| *mixed);
        match token {
            Token::Start { text, empty } => {
                if started && !parent_flat {
                    newline(&mut out, stack.len());
                }
                if let Some(parent) = stack.last_mut() {
                    parent.1 = true;
                }
                out.push_str(text);
                if !*empty {
                    stack.push((mixed[index], false));
                }
            }
            Token::End(text) => {
                let (was_mixed, has_children) = stack.pop().unwrap_or((true, false));
                if !was_mixed && has_children {
                    newline(&mut out, stack.len());
                }
                out.push_str(text);
            }
            Token::Text(text) => {
                // Whitespace-only text inside element-only content is replaced by indentation.
                if stack.is_empty()
                    || parent_flat
                    || !text.trim_matches([' ', '\t', '\r', '\n']).is_empty()
                {
                    out.push_str(text);
                }
            }
            Token::Markup(text) => {
                if started && !parent_flat && !stack.is_empty() {
                    newline(&mut out, stack.len());
                } else if started && stack.is_empty() {
                    newline(&mut out, 0);
                }
                if let Some(parent) = stack.last_mut() {
                    parent.1 = true;
                }
                out.push_str(text);
            }
        }
        started = true;
    }
    out
}
