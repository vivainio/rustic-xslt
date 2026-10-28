use crate::{
    Diagnostic,
    compile::Sort,
    xpath::{self, Budget, Focus, Item, Value, Variables},
};
use std::cmp::Ordering;
enum Key {
    Number(f64),
    Text(String),
}
fn compare(a: &Key, b: &Key, case: &str) -> Ordering {
    match (a, b) {
        (Key::Number(a), Key::Number(b)) => match (a.is_nan(), b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        },
        (Key::Text(a), Key::Text(b)) => {
            if case.is_empty() {
                return a.cmp(b);
            }
            let order = a.to_lowercase().cmp(&b.to_lowercase());
            if order != Ordering::Equal {
                order
            } else if case == "lower-first" {
                b.cmp(a)
            } else {
                a.cmp(b)
            }
        }
        _ => unreachable!(),
    }
}
/// Stable bottom-up merge sort; comparison work consumes the shared budget and
/// can stop immediately, rather than hiding long sorts inside an infallible closure.
pub(crate) fn sort<'a>(
    value: Value<'a>,
    sorts: &[Sort],
    env: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    if sorts.is_empty() {
        return Ok(value);
    }
    let size = value.len();
    let mut keys = Vec::with_capacity(size);
    for (i, item) in value.iter().enumerate() {
        let focus = Focus {
            item: item.clone(),
            position: i + 1,
            size,
        };
        let mut row = vec![];
        for key in sorts {
            let mut v = xpath::eval(&key.select, &focus, env, budget, 0, backwards)?;
            if backwards {
                v.truncate(1);
            }
            if v.len() > 1 {
                return Err(Diagnostic::new(
                    "XTTE1020",
                    "sort key must have at most one value",
                    "run",
                ));
            }
            let s = v.first().map_or(String::new(), Item::string);
            row.push(if key.numeric {
                Key::Number(s.trim().parse().unwrap_or(f64::NAN))
            } else {
                Key::Text(s)
            });
        }
        keys.push(row);
    }
    let mut order: Vec<_> = (0..size).collect();
    let mut spare = order.clone();
    let mut width = 1;
    while width < size {
        for start in (0..size).step_by(width * 2) {
            let middle = (start + width).min(size);
            let end = (middle + width).min(size);
            let (mut a, mut b) = (start, middle);
            for slot in &mut spare[start..end] {
                budget.tick()?;
                let take_left = if b == end {
                    true
                } else if a == middle {
                    false
                } else {
                    let mut result = Ordering::Equal;
                    for (k, spec) in sorts.iter().enumerate() {
                        budget.tick()?;
                        result = compare(&keys[order[a]][k], &keys[order[b]][k], &spec.case_order);
                        if spec.descending {
                            result = result.reverse();
                        }
                        if result != Ordering::Equal {
                            break;
                        }
                    }
                    result != Ordering::Greater
                };
                if take_left {
                    *slot = order[a];
                    a += 1;
                } else {
                    *slot = order[b];
                    b += 1;
                }
            }
        }
        std::mem::swap(&mut order, &mut spare);
        width *= 2;
    }
    let mut value: Vec<_> = value.into_iter().map(Some).collect();
    Ok(order
        .into_iter()
        .map(|i| value[i].take().unwrap())
        .collect())
}
