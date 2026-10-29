//! XPath regular expressions translated to a bounded, linear-time regex engine.
use crate::{
    Diagnostic,
    string_functions::string,
    xpath::{Budget, Item, Value},
};
use regex::{Regex, RegexBuilder};
fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(code, message, "run")
}
fn invalid() -> Diagnostic {
    error("FORX0002", "invalid XPath regular expression")
}
const NAME_START: &str = r":A-Z_a-z\x{C0}-\x{D6}\x{D8}-\x{F6}\x{F8}-\x{2FF}\x{370}-\x{37D}\x{37F}-\x{1FFF}\x{200C}-\x{200D}\x{2070}-\x{218F}\x{2C00}-\x{2FEF}\x{3001}-\x{D7FF}\x{F900}-\x{FDCF}\x{FDF0}-\x{FFFD}\x{10000}-\x{EFFFF}";
fn compile(pattern: &str, flags: &str, budget: &mut Budget) -> Result<Regex, Diagnostic> {
    for flag in flags.chars() {
        budget.tick()?;
        if !"imsxq".contains(flag) {
            return Err(error("FORX0001", "invalid regular-expression flag"));
        }
    }
    for _ in pattern.bytes() {
        budget.tick()?;
    }
    let translated = if flags.contains('q') {
        regex::escape(pattern)
    } else {
        let chars = pattern.chars().collect::<Vec<_>>();
        let mut result = String::new();
        let mut i = 0;
        let mut class = 0usize;
        let mut groups = 0usize;
        while i < chars.len() {
            let c = chars[i];
            i += 1;
            if flags.contains('x') && class == 0 && matches!(c, ' ' | '\t' | '\r' | '\n') {
                continue;
            }
            match c {
                '\\' => {
                    let escaped = *chars.get(i).ok_or_else(invalid)?;
                    i += 1;
                    match escaped {
                        'i' | 'I' | 'c' | 'C' => {
                            result.push('[');
                            if escaped.is_ascii_uppercase() {
                                result.push('^');
                            }
                            result.push_str(NAME_START);
                            if matches!(escaped, 'c' | 'C') {
                                result.push_str(r"\-\.0-9\x{B7}\x{300}-\x{36F}\x{203F}-\x{2040}");
                            }
                            result.push(']');
                        }
                        's' => result.push_str(r"[ \t\r\n]"),
                        'S' => result.push_str(r"[^ \t\r\n]"),
                        'w' => result.push_str(r"[^\p{P}\p{Z}\p{C}]"),
                        'W' => result.push_str(r"[\p{P}\p{Z}\p{C}]"),
                        '1'..='9' => {
                            return Err(error(
                                "RUST_XSLT_UNSUPPORTED",
                                "regular-expression backreferences are not implemented",
                            ));
                        }
                        'p' | 'P' => {
                            if chars.get(i) != Some(&'{') {
                                return Err(invalid());
                            }
                            let start = i;
                            i += 1;
                            while i < chars.len() && chars[i] != '}' {
                                i += 1;
                            }
                            if i == chars.len() {
                                return Err(invalid());
                            }
                            let property = chars[start + 1..i].iter().collect::<String>();
                            if let Some(block) = property.strip_prefix("Is") {
                                let class = crate::unicode_blocks::class(block, escaped == 'P')
                                    .ok_or_else(invalid)?;
                                result.push_str(&class);
                                i += 1;
                                continue;
                            }
                            // XPath accepts Unicode general categories, not engine-specific scripts/properties.
                            if ![
                                "L", "Lu", "Ll", "Lt", "Lm", "Lo", "M", "Mn", "Mc", "Me", "N",
                                "Nd", "Nl", "No", "P", "Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po",
                                "Z", "Zs", "Zl", "Zp", "S", "Sm", "Sc", "Sk", "So", "C", "Cc",
                                "Cf", "Cs", "Co", "Cn",
                            ]
                            .contains(&property.as_str())
                            {
                                return Err(invalid());
                            }
                            result.push('\\');
                            result.push(escaped);
                            result.extend(chars[start..=i].iter());
                            i += 1;
                        }
                        'n' | 'r' | 't' | 'd' | 'D' | '\\' | '|' | '.' | '-' | '^' | '$' | '?'
                        | '*' | '+' | '{' | '}' | '(' | ')' | '[' | ']' => {
                            result.push('\\');
                            result.push(escaped);
                        }
                        _ => return Err(invalid()),
                    }
                }
                '[' => {
                    class += 1;
                    result.push(c);
                }
                ']' => {
                    if class == 0 {
                        return Err(invalid());
                    }
                    class -= 1;
                    result.push(c);
                }
                '-' if class > 0 && chars.get(i) == Some(&'[') => result.push_str("--"),
                '&' | '~' if class > 0 => {
                    result.push('\\');
                    result.push(c);
                }
                '.' if class == 0 && !flags.contains('s') => result.push_str(r"[^\n\r]"),
                '(' if class == 0 => {
                    groups += 1;
                    if groups > 128 {
                        return Err(error("XPDY0130", "regex nesting limit exceeded"));
                    }
                    if chars.get(i) == Some(&'?') && (chars.get(i + 1) != Some(&':')) {
                        return Err(invalid());
                    }
                    result.push(c);
                }
                ')' if class == 0 => {
                    groups = groups.saturating_sub(1);
                    result.push(c);
                }
                _ => result.push(c),
            }
        }
        result
    };
    RegexBuilder::new(&translated)
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .size_limit(1 << 20)
        .dfa_size_limit(1 << 20)
        .nest_limit(256)
        .build()
        .map_err(|e| match e {
            regex::Error::CompiledTooBig(_) => {
                error("XPDY0130", "compiled regex size limit exceeded")
            }
            _ => invalid(),
        })
}
#[derive(Debug)]
enum Replacement {
    Text(String),
    Group(usize),
}
fn replacement(source: &str, groups: usize, literal: bool) -> Result<Vec<Replacement>, Diagnostic> {
    if literal {
        return Ok(vec![Replacement::Text(source.into())]);
    }
    let chars = source.chars().collect::<Vec<_>>();
    let mut i = 0;
    let mut result = vec![];
    let mut text = String::new();
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        if c == '\\' {
            let escaped = *chars
                .get(i)
                .ok_or_else(|| error("FORX0004", "invalid replacement escape"))?;
            i += 1;
            if !matches!(escaped, '\\' | '$') {
                return Err(error("FORX0004", "invalid replacement escape"));
            }
            text.push(escaped);
        } else if c == '$' {
            let start = i;
            while chars.get(i).is_some_and(char::is_ascii_digit) {
                i += 1;
            }
            if start == i {
                return Err(error(
                    "FORX0004",
                    "replacement dollar must be followed by a digit",
                ));
            }
            if !text.is_empty() {
                result.push(Replacement::Text(std::mem::take(&mut text)));
            }
            let mut end = i;
            let mut number = chars[start..end]
                .iter()
                .collect::<String>()
                .parse::<usize>()
                .unwrap_or(usize::MAX);
            while number > groups && end - start > 1 {
                end -= 1;
                number = chars[start..end]
                    .iter()
                    .collect::<String>()
                    .parse::<usize>()
                    .unwrap_or(usize::MAX);
            }
            result.push(Replacement::Group(number));
            text.extend(chars[end..i].iter());
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        result.push(Replacement::Text(text));
    }
    Ok(result)
}
fn append(result: &mut String, text: &str, budget: &mut Budget) -> Result<(), Diagnostic> {
    budget.charge(text.len())?;
    result.push_str(text);
    Ok(())
}
pub(crate) fn evaluate<'a>(
    name: &str,
    args: &[Value<'a>],
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    let input = string(&args[0], true, false)?;
    if name == "tokenize" && args.len() == 1 {
        budget.charge(input.len())?;
        return Ok(input
            .split([' ', '\t', '\r', '\n'])
            .filter(|s| !s.is_empty())
            .map(|s| Item::String(s.into()))
            .collect());
    }
    let pattern = string(&args[1], false, false)?;
    let flags = args
        .get(if name == "replace" { 3 } else { 2 })
        .map(|a| string(a, false, false))
        .transpose()?
        .unwrap_or_default();
    let regex = compile(&pattern, &flags, budget)?;
    let work = input
        .len()
        .checked_add(1)
        .and_then(|n| n.checked_mul(pattern.len().saturating_add(1)))
        .ok_or_else(|| error("XPDY0130", "regex work limit exceeded"))?;
    budget.charge(work)?;
    if name == "matches" {
        return Ok(vec![Item::Boolean(regex.is_match(&input))]);
    }
    if regex.is_match("") {
        return Err(error("FORX0003", "pattern matches an empty string"));
    }
    if name == "tokenize" {
        if input.is_empty() {
            return Ok(vec![]);
        }
        return regex
            .split(&input)
            .map(|part| {
                budget.tick()?;
                Ok(Item::String(part.into()))
            })
            .collect();
    }
    let replace = string(&args[2], false, false)?;
    budget.charge(replace.len())?;
    let parts = replacement(&replace, regex.captures_len() - 1, flags.contains('q'))?;
    let mut result = String::new();
    let mut previous = 0;
    for captures in regex.captures_iter(&input) {
        budget.tick()?;
        let found = captures.get(0).unwrap();
        append(&mut result, &input[previous..found.start()], budget)?;
        for part in &parts {
            let text = match part {
                Replacement::Text(s) => s.as_str(),
                Replacement::Group(n) => captures.get(*n).map_or("", |m| m.as_str()),
            };
            append(&mut result, text, budget)?;
        }
        previous = found.end();
    }
    append(&mut result, &input[previous..], budget)?;
    Ok(vec![Item::String(result)])
}
