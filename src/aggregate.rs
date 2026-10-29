//! Aggregate functions over the supported atomic types.
use crate::{
    Diagnostic,
    number::Number,
    xpath::{Budget, Item, Value, atomize},
};
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
    if matches!(name, "min" | "max")
        && let Some(collation) = args.get(1)
    {
        let [item] = collation.as_slice() else {
            return Err(error("XPTY0004", "collation must be a singleton string"));
        };
        let item = atomize(item);
        if !matches!(item, Item::String(_) | Item::Untyped(_) | Item::AnyUri(_)) {
            return Err(error("XPTY0004", "collation must be a string"));
        }
        if item.string() != "http://www.w3.org/2005/xpath-functions/collation/codepoint" {
            return Err(error("FOCH0002", "unsupported collation"));
        }
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
            Item::AnyUri(s) => Item::String(s),
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
            numbers.push(promoted(&n, target)?);
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
    let strings = values.iter().all(|v| matches!(v, Item::String(_)));
    let booleans = values.iter().all(|v| matches!(v, Item::Boolean(_)));
    if !strings && !booleans {
        return Err(error(
            "FORG0006",
            "min/max values must have comparable types",
        ));
    }
    let mut result = values[0].clone();
    for value in values.into_iter().skip(1) {
        budget.tick()?;
        let order = match (&result, &value) {
            (Item::String(a), Item::String(b)) => a.cmp(b),
            (Item::Boolean(a), Item::Boolean(b)) => a.cmp(b),
            _ => unreachable!(),
        };
        if name == "min" && order.is_gt() || name == "max" && order.is_lt() {
            result = value;
        }
    }
    Ok(vec![result])
}
