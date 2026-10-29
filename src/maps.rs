//! XPath 3.1 maps and arrays: construction helpers, lookup and the `map:` and
//! `array:` function libraries (higher-order members live beside `fn:for-each`).
use crate::{
    Diagnostic,
    number::Number,
    xpath::{Budget, Item, MapValue, Value, atomize},
};
use std::rc::Rc;

fn err(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(code, message, "run")
}

pub(crate) fn arity(name: &str) -> Option<(usize, usize)> {
    Some(match name {
        "map:size" | "map:keys" | "array:size" | "array:head" | "array:tail" | "array:reverse"
        | "array:flatten" | "array:join" => (1, 1),
        "map:contains" | "map:get" | "map:remove" | "map:for-each" | "array:get"
        | "array:append" | "array:remove" | "array:for-each" | "array:filter" | "map:entry" => {
            (2, 2)
        }
        "map:put"
        | "array:put"
        | "array:insert-before"
        | "array:fold-left"
        | "array:fold-right"
        | "array:for-each-pair" => (3, 3),
        "map:merge" => (1, 2),
        "map:find" => (2, 2),
        "array:subarray" => (2, 3),
        "array:sort" => (1, 3),
        _ => return None,
    })
}

/// Functions that call back into function items are evaluated by the XPath engine.
pub(crate) fn higher_order(name: &str) -> bool {
    matches!(
        name,
        "map:for-each"
            | "array:for-each"
            | "array:filter"
            | "array:fold-left"
            | "array:fold-right"
            | "array:for-each-pair"
            | "array:sort"
    )
}

/// A map key: exactly one atomic value.
pub(crate) fn key<'a>(value: &[Item<'a>]) -> Result<Item<'a>, Diagnostic> {
    match value {
        [item] => match atomize(item) {
            Item::Function(_) | Item::Map(_) | Item::Array(_) => {
                Err(err("XPTY0004", "a map key must be an atomic value"))
            }
            Item::Untyped(s) => Ok(Item::String(s)),
            other => Ok(other),
        },
        _ => Err(err("XPTY0004", "a map key must be a single atomic value")),
    }
}

fn integer(value: &[Item<'_>]) -> Result<i64, Diagnostic> {
    use num_traits::ToPrimitive;
    match value {
        [Item::Number(Number::Integer(n) | Number::DerivedInteger(_, n))] => n
            .to_i64()
            .ok_or_else(|| err("FOAY0001", "array position out of range")),
        [Item::Untyped(s)] => s
            .trim()
            .parse()
            .map_err(|_| err("FORG0001", "invalid array position")),
        _ => Err(err(
            "XPTY0004",
            "an array position must be a single integer",
        )),
    }
}

pub(crate) fn array_member<'a, 'b>(
    array: &'b [Value<'a>],
    position: &[Item<'a>],
) -> Result<&'b Value<'a>, Diagnostic> {
    let index = integer(position)?;
    usize::try_from(index)
        .ok()
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| array.get(i))
        .ok_or_else(|| err("FOAY0001", "array position out of bounds"))
}

fn flatten_into<'a>(
    value: &[Item<'a>],
    out: &mut Value<'a>,
    budget: &mut Budget,
) -> Result<(), Diagnostic> {
    for item in value {
        budget.tick()?;
        match item {
            Item::Array(members) => {
                for member in members.iter() {
                    flatten_into(member, out, budget)?;
                }
            }
            other => out.push(other.clone()),
        }
    }
    Ok(())
}

/// `base?key`, `base?*`
pub(crate) fn lookup<'a>(
    base: &[Item<'a>],
    keys: Option<&[Item<'a>]>,
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    let mut out = vec![];
    for item in base {
        budget.tick()?;
        match (item, keys) {
            (Item::Map(map), None) => {
                for (_, value) in map.iter() {
                    out.extend(value.iter().cloned());
                }
            }
            (Item::Array(array), None) => {
                for member in array.iter() {
                    out.extend(member.iter().cloned());
                }
            }
            (Item::Map(map), Some(keys)) => {
                for k in keys {
                    if let Some(value) = map.get(&key(std::slice::from_ref(k))?) {
                        out.extend(value.iter().cloned());
                    }
                }
            }
            (Item::Array(array), Some(keys)) => {
                for k in keys {
                    out.extend(
                        array_member(array, std::slice::from_ref(&atomize(k)))?
                            .iter()
                            .cloned(),
                    );
                }
            }
            _ => {
                return Err(err(
                    "XPTY0004",
                    "the lookup operator requires maps or arrays",
                ));
            }
        }
    }
    Ok(out)
}

fn map_arg<'a>(value: &[Item<'a>]) -> Result<Rc<MapValue<'a>>, Diagnostic> {
    match value {
        [Item::Map(m)] => Ok(m.clone()),
        _ => Err(err("XPTY0004", "a single map is required")),
    }
}

fn array_arg<'a>(value: &[Item<'a>]) -> Result<Rc<Vec<Value<'a>>>, Diagnostic> {
    match value {
        [Item::Array(a)] => Ok(a.clone()),
        _ => Err(err("XPTY0004", "a single array is required")),
    }
}

fn wrap_map(map: MapValue<'_>) -> Value<'_> {
    vec![Item::Map(Rc::new(map))]
}

fn wrap_array(array: Vec<Value<'_>>) -> Value<'_> {
    vec![Item::Array(Rc::new(array))]
}

fn integer_value<'a>(n: usize) -> Value<'a> {
    vec![Item::Number(Number::from(n))]
}

pub(crate) fn evaluate<'a>(
    name: &str,
    args: &[Value<'a>],
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    match name {
        "map:size" => Ok(integer_value(map_arg(&args[0])?.len())),
        "map:keys" => Ok(map_arg(&args[0])?.iter().map(|(k, _)| k.clone()).collect()),
        "map:contains" => Ok(vec![Item::Boolean(
            map_arg(&args[0])?.get(&key(&args[1])?).is_some(),
        )]),
        "map:get" => Ok(map_arg(&args[0])?
            .get(&key(&args[1])?)
            .cloned()
            .unwrap_or_default()),
        "map:put" => {
            let mut map = (*map_arg(&args[0])?).clone();
            budget.charge(map.len())?;
            map.insert(key(&args[1])?, args[2].clone());
            Ok(wrap_map(map))
        }
        "map:entry" => {
            let mut map = MapValue::default();
            map.push_new(key(&args[0])?, args[1].clone());
            Ok(wrap_map(map))
        }
        "map:remove" => {
            let mut map = (*map_arg(&args[0])?).clone();
            for k in &args[1] {
                budget.tick()?;
                let k = key(std::slice::from_ref(k))?;
                map.remove(&k);
            }
            Ok(wrap_map(map))
        }
        "map:merge" => {
            let duplicates = match args.get(1) {
                Some(options) => {
                    let options = map_arg(options)?;
                    match options.get(&Item::String("duplicates".into())) {
                        Some(v) => match v.as_slice() {
                            [Item::String(s)] => s.clone(),
                            _ => return Err(err("XPTY0004", "duplicates option must be a string")),
                        },
                        None => "use-first".into(),
                    }
                }
                None => "use-first".into(),
            };
            if !matches!(
                duplicates.as_str(),
                "use-first" | "use-last" | "use-any" | "combine" | "reject"
            ) {
                return Err(err("FOJS0005", "invalid duplicates option"));
            }
            let mut merged = MapValue::default();
            for item in &args[0] {
                budget.tick()?;
                let map = map_arg(std::slice::from_ref(item))?;
                for (k, v) in map.iter() {
                    budget.tick()?;
                    match merged.get_mut(k) {
                        None => merged.push_new(k.clone(), v.clone()),
                        Some(existing) => match duplicates.as_str() {
                            "use-last" => *existing = v.clone(),
                            "combine" => existing.extend(v.iter().cloned()),
                            "reject" => return Err(err("FOJS0003", "duplicate map key")),
                            _ => {}
                        },
                    }
                }
            }
            Ok(wrap_map(merged))
        }
        "map:find" => {
            let wanted = key(&args[1])?;
            let mut found = vec![];
            let mut pending: Vec<Item<'a>> = args[0].iter().rev().cloned().collect();
            while let Some(item) = pending.pop() {
                budget.tick()?;
                match &item {
                    Item::Array(members) => {
                        for member in members.iter().rev() {
                            pending.extend(member.iter().rev().cloned());
                        }
                    }
                    Item::Map(map) => {
                        if let Some(value) = map.get(&wanted) {
                            found.push(value.clone());
                        }
                        for (_, value) in map.iter().rev() {
                            pending.extend(value.iter().rev().cloned());
                        }
                    }
                    _ => {}
                }
            }
            Ok(wrap_array(found))
        }
        "array:size" => Ok(integer_value(array_arg(&args[0])?.len())),
        "array:get" => Ok(array_member(&array_arg(&args[0])?, &args[1])?.clone()),
        "array:put" => {
            let mut array = (*array_arg(&args[0])?).clone();
            let index = integer(&args[1])?;
            let slot = usize::try_from(index)
                .ok()
                .and_then(|i| i.checked_sub(1))
                .and_then(|i| array.get_mut(i))
                .ok_or_else(|| err("FOAY0001", "array position out of bounds"))?;
            *slot = args[2].clone();
            Ok(wrap_array(array))
        }
        "array:append" => {
            let mut array = (*array_arg(&args[0])?).clone();
            array.push(args[1].clone());
            Ok(wrap_array(array))
        }
        "array:subarray" => {
            let array = array_arg(&args[0])?;
            let start = integer(&args[1])?;
            let length = match args.get(2) {
                Some(length) => integer(length)?,
                None => array.len() as i64 - start + 1,
            };
            if start < 1 || length < 0 || start - 1 + length > array.len() as i64 {
                return Err(err("FOAY0001", "subarray out of bounds"));
            }
            let from = (start - 1) as usize;
            Ok(wrap_array(array[from..from + length as usize].to_vec()))
        }
        "array:remove" => {
            let array = array_arg(&args[0])?;
            let mut positions = vec![];
            for p in &args[1] {
                let p = integer(std::slice::from_ref(&atomize(p)))?;
                if p < 1 || p > array.len() as i64 {
                    return Err(err("FOAY0001", "array position out of bounds"));
                }
                positions.push(p as usize - 1);
            }
            Ok(wrap_array(
                array
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !positions.contains(i))
                    .map(|(_, m)| m.clone())
                    .collect(),
            ))
        }
        "array:insert-before" => {
            let mut array = (*array_arg(&args[0])?).clone();
            let position = integer(&args[1])?;
            if position < 1 || position > array.len() as i64 + 1 {
                return Err(err("FOAY0001", "array position out of bounds"));
            }
            array.insert(position as usize - 1, args[2].clone());
            Ok(wrap_array(array))
        }
        "array:head" => array_arg(&args[0])?
            .first()
            .cloned()
            .ok_or_else(|| err("FOAY0001", "the array is empty")),
        "array:tail" => {
            let array = array_arg(&args[0])?;
            if array.is_empty() {
                return Err(err("FOAY0001", "the array is empty"));
            }
            Ok(wrap_array(array[1..].to_vec()))
        }
        "array:reverse" => {
            let mut array = (*array_arg(&args[0])?).clone();
            array.reverse();
            Ok(wrap_array(array))
        }
        "array:join" => {
            let mut joined = vec![];
            for item in &args[0] {
                budget.tick()?;
                joined.extend(array_arg(std::slice::from_ref(item))?.iter().cloned());
            }
            Ok(wrap_array(joined))
        }
        "array:flatten" => {
            let mut out = vec![];
            flatten_into(&args[0], &mut out, budget)?;
            Ok(out)
        }
        _ => unreachable!("not a map or array function: {name}"),
    }
}

/// Members of an array argument for the higher-order array functions.
pub(crate) fn members<'a>(value: &[Item<'a>]) -> Result<Rc<Vec<Value<'a>>>, Diagnostic> {
    array_arg(value)
}

pub(crate) fn entries<'a>(value: &[Item<'a>]) -> Result<Rc<MapValue<'a>>, Diagnostic> {
    map_arg(value)
}

pub(crate) fn new_array(members: Vec<Value<'_>>) -> Value<'_> {
    wrap_array(members)
}
