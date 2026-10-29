//! Aggregate functions over the supported atomic types.
use crate::{
    Diagnostic,
    collation::Collation,
    number::Number,
    xpath::{Budget, Item, Value, atomize},
};
use std::cmp::Ordering;
fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(code, message, "run")
}
fn rank(number: &Number) -> u8 {
    match number {
        Number::Integer(_) | Number::DerivedInteger(_, _) => 0,
        Number::Decimal(_) => 1,
        Number::Float(_) => 2,
        Number::Double(_) => 3,
    }
}
fn promoted(number: &Number, target: u8) -> Result<Number, Diagnostic> {
    if target == 0 {
        Ok(number.clone())
    } else {
        number.convert(match target {
            1 => "decimal",
            2 => "float",
            _ => "double",
        })
    }
}
pub(crate) fn evaluate<'a>(
    name: &str,
    args: &[Value<'a>],
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    let mut collation = Collation::Codepoint;
    if matches!(name, "min" | "max")
        && let Some(collation_arg) = args.get(1)
    {
        let [item] = collation_arg.as_slice() else {
            return Err(error("XPTY0004", "collation must be a singleton string"));
        };
        let item = atomize(item);
        if !matches!(
            item,
            Item::String(_) | Item::Untyped(_) | Item::AnyUri(_) | Item::DerivedString(_, _)
        ) {
            return Err(error("XPTY0004", "collation must be a string"));
        }
        collation = Collation::parse(&item.string())?;
    }
    let zero = if name == "sum" && args.len() == 2 {
        if args[1].len() > 1 {
            return Err(error(
                "XPTY0004",
                "sum zero must be an optional atomic singleton",
            ));
        }
        args[1].iter().map(atomize).collect()
    } else {
        vec![Item::Number(Number::from(0usize))]
    };
    if args[0].is_empty() {
        return Ok(if name == "sum" { zero } else { vec![] });
    }
    let mut values = vec![];
    for item in &args[0] {
        budget.tick()?;
        values.push(match atomize(item) {
            Item::Untyped(s) => Item::Number(Number::cast("double", &s)?),
            other => other,
        });
    }
    if values.iter().all(|v| matches!(v, Item::Number(_))) {
        let target = values
            .iter()
            .filter_map(|v| {
                if let Item::Number(n) = v {
                    Some(rank(n))
                } else {
                    None
                }
            })
            .max()
            .unwrap();
        let mut numbers = vec![];
        for value in values {
            budget.tick()?;
            let Item::Number(n) = value else {
                unreachable!()
            };
            numbers.push(if matches!(name, "min" | "max") && target <= 1 {
                n
            } else {
                promoted(&n, target)?
            });
        }
        let mut result = numbers[0].clone();
        for number in numbers.iter().skip(1) {
            budget.tick()?;
            if name == "sum" || name == "avg" {
                result = result.arithmetic(number, "+")?;
            } else {
                match result.compare(number) {
                    None => {
                        if number.compare(number).is_none() {
                            result = number.clone();
                        }
                    }
                    Some(order)
                        if name == "min" && order.is_gt() || name == "max" && order.is_lt() =>
                    {
                        result = number.clone()
                    }
                    _ => {}
                }
            }
        }
        if name == "avg" {
            result = result.arithmetic(&Number::from(numbers.len()), "div")?;
        }
        return Ok(vec![Item::Number(result)]);
    }
    if name == "sum" || name == "avg" {
        return Err(error(
            "FORG0006",
            "aggregate requires supported numeric values",
        ));
    }
    if values.iter().any(|v| matches!(v, Item::AnyUri(_)))
        && values
            .iter()
            .any(|v| matches!(v, Item::String(_) | Item::DerivedString(_, _)))
    {
        for value in &mut values {
            budget.tick()?;
            if let Item::AnyUri(text) | Item::DerivedString(_, text) = value {
                *value = Item::String(std::mem::take(text));
            }
        }
    }
    let strings = values.iter().all(|v| {
        matches!(
            v,
            Item::String(_) | Item::AnyUri(_) | Item::DerivedString(_, _)
        )
    });
    let booleans = values.iter().all(|v| matches!(v, Item::Boolean(_)));
    let binaries = matches!(&values[0], Item::Binary(kind, _) if values.iter().all(|v| matches!(v, Item::Binary(other, _) if kind == other)));
    let temporals = matches!(&values[0], Item::Temporal(first) if values.iter().all(|v| {
        matches!(v, Item::Temporal(other)
            if first.kind == other.kind && other.kind.is_ordered())
    }));
    if !strings && !booleans && !binaries && !temporals {
        return Err(error(
            "FORG0006",
            "min/max values must have comparable types",
        ));
    }
    let mut result = values[0].clone();
    for value in values.into_iter().skip(1) {
        budget.tick()?;
        let order = match (&result, &value) {
            (
                Item::String(a) | Item::AnyUri(a) | Item::DerivedString(_, a),
                Item::String(b) | Item::AnyUri(b) | Item::DerivedString(_, b),
            ) => collation.compare(a, b),
            (Item::Boolean(a), Item::Boolean(b)) => a.cmp(b),
            (Item::Binary(_, a), Item::Binary(_, b)) => a.cmp(b),
            (Item::Temporal(a), Item::Temporal(b)) => a.compare(b).expect("comparable temporals"),
            _ => unreachable!(),
        };
        if name == "min" && order.is_gt() || name == "max" && order.is_lt() {
            result = value;
        }
    }
    Ok(vec![result])
}

/// `fn:sort` with an optional collation; keys are the atomized items.
pub(crate) fn sort<'a>(
    args: &[Value<'a>],
    key_values: Option<Vec<Value<'a>>>,
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    let collation = match args.get(1) {
        Some(uri) => {
            let uri = crate::string_functions::string(uri, true, false)?;
            if args[1].is_empty() {
                Collation::Codepoint
            } else {
                Collation::parse(&uri)?
            }
        }
        None => Collation::Codepoint,
    };
    let mut keyed: Vec<(Value<'a>, Item<'a>)> = Vec::with_capacity(args[0].len());
    for (index, item) in args[0].iter().enumerate() {
        budget.tick()?;
        let key = match &key_values {
            Some(keys) => crate::xpath::flatten_arrays(keys[index].clone())?
                .iter()
                .map(atomize)
                .collect(),
            None => crate::xpath::flatten_arrays(vec![item.clone()])?
                .iter()
                .map(atomize)
                .collect(),
        };
        keyed.push((key, item.clone()));
    }
    let order = |a: &Item<'a>, b: &Item<'a>| -> Result<Ordering, Diagnostic> {
        let strings = |item: &Item<'_>| match item {
            Item::String(s) | Item::AnyUri(s) | Item::Untyped(s) | Item::DerivedString(_, s) => {
                Some(s.clone())
            }
            _ => None,
        };
        if let (Some(x), Some(y)) = (strings(a), strings(b)) {
            return Ok(collation.compare(&x, &y));
        }
        let nan = |item: &Item<'_>| matches!(item, Item::Number(n) if n.compare(n).is_none());
        match (nan(a), nan(b)) {
            (true, true) => return Ok(Ordering::Equal),
            (true, false) => return Ok(Ordering::Less),
            (false, true) => return Ok(Ordering::Greater),
            _ => {}
        }
        Ok(if crate::xpath::compare(a, b, "<", false)? {
            Ordering::Less
        } else if crate::xpath::compare(b, a, "<", false)? {
            Ordering::Greater
        } else {
            Ordering::Equal
        })
    };
    let order_keys = |a: &Value<'a>, b: &Value<'a>| -> Result<Ordering, Diagnostic> {
        for (x, y) in a.iter().zip(b) {
            let result = order(x, y)?;
            if result != Ordering::Equal {
                return Ok(result);
            }
        }
        Ok(a.len().cmp(&b.len()))
    };
    // Stable bottom-up merge sort; a fallible comparator cannot violate a total order.
    let mut width = 1;
    while width < keyed.len() {
        let mut merged = Vec::with_capacity(keyed.len());
        let mut start = 0;
        while start < keyed.len() {
            let mid = (start + width).min(keyed.len());
            let end = (start + 2 * width).min(keyed.len());
            let (mut left, mut right) = (start, mid);
            while left < mid && right < end {
                budget.tick()?;
                if order_keys(&keyed[right].0, &keyed[left].0)? == Ordering::Less {
                    merged.push(keyed[right].clone());
                    right += 1;
                } else {
                    merged.push(keyed[left].clone());
                    left += 1;
                }
            }
            merged.extend_from_slice(&keyed[left..mid]);
            merged.extend_from_slice(&keyed[right..end]);
            start = end;
        }
        keyed = merged;
        width *= 2;
    }
    Ok(keyed.into_iter().map(|(_, item)| item).collect())
}
