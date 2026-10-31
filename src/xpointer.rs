//! XML fragment selection using XPointer shorthand and element() pointers.
use crate::{Diagnostic, node::XNode, qname::xml_ncname, xpath::Budget};

fn invalid() -> Diagnostic {
    Diagnostic::new(
        "XTDE1160",
        "invalid or unresolved XML fragment identifier",
        "run",
    )
}

fn decode(fragment: &str) -> Result<String, Diagnostic> {
    let input = fragment.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            let digit = |b: u8| (b as char).to_digit(16).map(|n| n as u8);
            let (Some(a), Some(b)) = (
                input.get(i + 1).and_then(|b| digit(*b)),
                input.get(i + 2).and_then(|b| digit(*b)),
            ) else {
                return Err(invalid());
            };
            bytes.push(a * 16 + b);
            i += 3;
        } else {
            bytes.push(input[i]);
            i += 1;
        }
    }
    String::from_utf8(bytes).map_err(|_| invalid())
}

fn id<'a>(
    document: XNode<'a>,
    name: &str,
    budget: &mut Budget,
) -> Result<Option<XNode<'a>>, Diagnostic> {
    for raw in document.raw.descendants() {
        budget.tick()?;
        if raw.is_element()
            && raw
                .attribute(("http://www.w3.org/XML/1998/namespace", "id"))
                .is_some_and(|value| value.trim_matches([' ', '\t', '\r', '\n']) == name)
        {
            return Ok(Some(document.relative(raw)));
        }
    }
    Ok(None)
}

fn element<'a>(
    document: XNode<'a>,
    data: &str,
    budget: &mut Budget,
) -> Result<Option<XNode<'a>>, Diagnostic> {
    let (anchor, steps) = data
        .split_once('/')
        .map_or((data, None), |(anchor, steps)| (anchor, Some(steps)));
    let mut node = if anchor.is_empty() {
        if steps.is_none() {
            return Ok(None);
        }
        Some(document)
    } else if xml_ncname(anchor) {
        id(document, anchor, budget)?
    } else {
        return Ok(None);
    };
    if let Some(steps) = steps {
        for step in steps.split('/') {
            if step.is_empty() || step.starts_with('0') || !step.bytes().all(|b| b.is_ascii_digit())
            {
                return Ok(None);
            }
            let Ok(index) = step.parse::<usize>() else {
                return Ok(None);
            };
            let Some(parent) = node else { return Ok(None) };
            let mut count = 0usize;
            node = None;
            for child in parent.children() {
                budget.tick()?;
                if child.is_element() {
                    count += 1;
                    if count == index {
                        node = Some(child);
                        break;
                    }
                }
            }
        }
    }
    Ok(node)
}

pub(crate) fn select<'a>(
    document: XNode<'a>,
    fragment: &str,
    budget: &mut Budget,
) -> Result<XNode<'a>, Diagnostic> {
    budget.charge(fragment.len())?;
    let decoded = decode(fragment)?;
    if xml_ncname(&decoded) {
        return id(document, &decoded, budget)?.ok_or_else(invalid);
    }
    let mut rest = decoded.as_str();
    while !rest.is_empty() {
        rest = rest.trim_start_matches([' ', '\t', '\r', '\n']);
        if rest.is_empty() {
            break;
        }
        let open = rest.find('(').ok_or_else(invalid)?;
        let scheme = &rest[..open];
        if !xml_ncname(scheme)
            && !scheme
                .split_once(':')
                .is_some_and(|(p, l)| xml_ncname(p) && xml_ncname(l))
        {
            return Err(invalid());
        }
        let mut data = String::new();
        let mut depth = 1usize;
        let mut close = None;
        let mut chars = rest[open + 1..].char_indices().peekable();
        while let Some((index, c)) = chars.next() {
            if c == '^' {
                let Some((_, escaped @ ('^' | '(' | ')'))) = chars.next() else {
                    return Err(invalid());
                };
                data.push(escaped);
            } else if c == '(' {
                depth += 1;
                data.push(c);
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + 1 + index + c.len_utf8());
                    break;
                }
                data.push(c);
            } else {
                data.push(c);
            }
        }
        let end = close.ok_or_else(invalid)?;
        if scheme == "element"
            && let Some(node) = element(document, &data, budget)?
        {
            return Ok(node);
        }
        rest = &rest[end..];
    }
    Err(invalid())
}
