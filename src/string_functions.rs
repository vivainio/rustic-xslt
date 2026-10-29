//! Unicode string operations with XPath function conversion and shared budgets.
use crate::{
    Diagnostic,
    number::Number,
    xpath::{self, Budget, Item, Value},
};
use num_traits::ToPrimitive;
use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;
pub(crate) fn collapse_xml_whitespace(value: &str) -> String {
    value
        .split([' ', '\t', '\n', '\r'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
fn type_error() -> Diagnostic {
    Diagnostic::new(
        "XPTY0004",
        "string function argument has an invalid type or cardinality",
        "run",
    )
}
pub(crate) fn string(
    value: &[Item<'_>],
    optional: bool,
    backwards: bool,
) -> Result<String, Diagnostic> {
    if value.is_empty() {
        return if optional || backwards {
            Ok(String::new())
        } else {
            Err(type_error())
        };
    }
    if value.len() > 1 && !backwards {
        return Err(type_error());
    }
    match xpath::atomize(&value[0]) {
        Item::String(s) | Item::Untyped(s) | Item::AnyUri(s) | Item::DerivedString(_, s) => Ok(s),
        item if backwards => Ok(item.string()),
        _ => Err(type_error()),
    }
}
fn number(value: &[Item<'_>], backwards: bool) -> Result<f64, Diagnostic> {
    if backwards {
        return match value.first().map(xpath::atomize) {
            Some(Item::Number(n)) => Ok(n.double()),
            Some(Item::Boolean(b)) => Ok(if b { 1.0 } else { 0.0 }),
            Some(item) => {
                Ok(Number::cast("double", &item.string()).map_or(f64::NAN, |n| n.double()))
            }
            None => Ok(f64::NAN),
        };
    }
    let [item] = value else {
        return Err(type_error());
    };
    match xpath::atomize(item) {
        Item::Number(n) => Ok(n.double()),
        Item::Untyped(s) => Ok(Number::cast("double", &s)?.double()),
        _ => Err(type_error()),
    }
}
fn round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor < 0.5 {
        floor
    } else {
        value.ceil()
    }
}
pub(crate) fn evaluate<'a>(
    name: &str,
    args: &[Value<'a>],
    budget: &mut Budget,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    if name == "codepoints-to-string" {
        let mut result = String::new();
        for item in &args[0] {
            budget.tick()?;
            let integer = match xpath::atomize(item) {
                Item::Number(Number::Integer(n) | Number::DerivedInteger(_, n)) => n,
                Item::Untyped(s) => {
                    let Number::Integer(n) = Number::cast("integer", &s)? else {
                        unreachable!()
                    };
                    n
                }
                _ => return Err(type_error()),
            };
            let point = integer.to_u32().filter(|c| matches!(c, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF));
            let c = point.and_then(char::from_u32).ok_or_else(|| {
                Diagnostic::new(
                    "FOCH0001",
                    "codepoint is not a valid XML 1.0 character",
                    "run",
                )
            })?;
            result.push(c);
        }
        return Ok(vec![Item::String(result)]);
    }
    if matches!(name, "compare" | "codepoint-equal") {
        let left = string(&args[0], true, backwards)?;
        let right = string(&args[1], true, backwards)?;
        if let Some(collation) = args.get(2)
            && string(collation, false, backwards)?
                != "http://www.w3.org/2005/xpath-functions/collation/codepoint"
        {
            return Err(Diagnostic::new("FOCH0002", "unsupported collation", "run"));
        }
        if args[0].is_empty() || args[1].is_empty() {
            return Ok(vec![]);
        }
        for _ in left.bytes().chain(right.bytes()) {
            budget.tick()?;
        }
        return Ok(vec![if name == "codepoint-equal" {
            Item::Boolean(left == right)
        } else {
            let result: i32 = match left.cmp(&right) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            Item::Number(Number::Integer(result.into()))
        }]);
    }
    let input = string(&args[0], true, backwards)?;
    if name == "string-to-codepoints" {
        let mut result = vec![];
        for c in input.chars() {
            budget.tick()?;
            result.push(Item::Number(Number::from(c as usize)));
        }
        return Ok(result);
    }
    let mut result = String::new();
    match name {
        "normalize-unicode" => {
            let form = args
                .get(1)
                .map(|v| string(v, false, backwards))
                .transpose()?
                .unwrap_or_else(|| "NFC".into());
            let form = form
                .trim_matches([' ', '\t', '\n', '\r'])
                .to_ascii_uppercase();
            for _ in input.bytes() {
                budget.tick()?;
            }
            let normalized: Box<dyn Iterator<Item = char> + '_> = match form.as_str() {
                "NFC" => Box::new(input.nfc()),
                "NFD" => Box::new(input.nfd()),
                "NFKC" => Box::new(input.nfkc()),
                "NFKD" => Box::new(input.nfkd()),
                "" => Box::new(input.chars()),
                _ => {
                    return Err(Diagnostic::new(
                        "FOCH0003",
                        "unsupported Unicode normalization form",
                        "run",
                    ));
                }
            };
            for c in normalized {
                budget.tick()?;
                result.push(c);
            }
        }
        "upper-case" | "lower-case" => {
            for _ in input.bytes() {
                budget.tick()?;
            }
            result = if name == "upper-case" {
                input.to_uppercase()
            } else {
                input.to_lowercase()
            };
            for _ in result.bytes() {
                budget.tick()?;
            }
        }
        "translate" => {
            let map = string(&args[1], false, backwards)?;
            let replacement = string(&args[2], false, backwards)?;
            let mut replacement = replacement.chars();
            let mut table = BTreeMap::new();
            for c in map.chars() {
                budget.tick()?;
                let value = replacement.next();
                table.entry(c).or_insert(value);
            }
            for c in input.chars() {
                budget.tick()?;
                if let Some(c) = table.get(&c).copied().unwrap_or(Some(c)) {
                    result.push(c);
                }
            }
        }
        "substring" => {
            let start = round(number(&args[1], backwards)?);
            let end = if args.len() == 3 {
                start + round(number(&args[2], backwards)?)
            } else {
                f64::INFINITY
            };
            for (index, c) in input.chars().enumerate() {
                budget.tick()?;
                let position = (index + 1) as f64;
                if position >= start && position < end {
                    result.push(c);
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(vec![Item::String(result)])
}
