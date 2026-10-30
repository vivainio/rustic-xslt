use crate::{
    Diagnostic,
    collation::Collation,
    compile::Sort,
    xpath::{self, Budget, Focus, Value, Variables},
};
use std::cmp::Ordering;
enum Key {
    Number(f64),
    Text(String),
}
/// A sort key's settings after attribute value templates have been evaluated.
struct Settings {
    numeric: bool,
    descending: bool,
    case_order: String,
    collation: Option<Collation>,
}
fn resolve<'a>(
    sort: &Sort,
    focus: &Focus<'a>,
    env: &Variables<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<Settings, Diagnostic> {
    let mut settings = Settings {
        numeric: sort.numeric,
        descending: sort.descending,
        case_order: sort.case_order.clone(),
        collation: sort.collation,
    };
    let text = |parts: &Option<Vec<crate::xpath::Expr>>,
                budget: &mut Budget|
     -> Result<Option<String>, Diagnostic> {
        let Some(parts) = parts else { return Ok(None) };
        let mut out = String::new();
        for part in parts {
            let value = xpath::eval(part, focus, env, budget, 0, backwards)?;
            out.push_str(&xpath::simple_string(&value, " ", usize::MAX, backwards)?);
        }
        Ok(Some(out.trim().to_owned()))
    };
    let invalid = |what: &str| Diagnostic::new("XTDE0030", format!("invalid sort {what}"), "run");
    if let Some(v) = text(&sort.dynamic.stable, budget)?
        && !["yes", "no", "true", "false", "1", "0"].contains(&v.as_str())
    {
        return Err(invalid("stable"));
    }
    if let Some(v) = text(&sort.dynamic.data_type, budget)? {
        settings.numeric = match v.as_str() {
            "text" | "" => false,
            "number" => true,
            _ => return Err(invalid("data-type")),
        };
    }
    if let Some(v) = text(&sort.dynamic.order, budget)? {
        settings.descending = match v.as_str() {
            "ascending" | "" => false,
            "descending" => true,
            _ => return Err(invalid("order")),
        };
    }
    if let Some(v) = text(&sort.dynamic.case_order, budget)? {
        if !["", "upper-first", "lower-first"].contains(&v.as_str()) {
            return Err(invalid("case-order"));
        }
        settings.case_order = v;
    }
    if let Some(v) = text(&sort.dynamic.collation, budget)? {
        settings.collation = Some(Collation::parse(&v).map_err(|mut e| {
            e.code = "XTDE1035".into();
            e
        })?);
    }
    Ok(settings)
}
fn compare(a: &Key, b: &Key, case: &str, collation: Option<Collation>) -> Ordering {
    match (a, b) {
        (Key::Number(a), Key::Number(b)) => match (a.is_nan(), b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        },
        (Key::Text(a), Key::Text(b)) => {
            if let Some(collation) = collation {
                return collation.compare(a, b);
            }
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
    focus: &Focus<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    if sorts.is_empty() {
        return Ok(value);
    }
    let order = order(&value, sorts, env, focus, None, budget, backwards)?;
    let mut value: Vec<_> = value.into_iter().map(Some).collect();
    Ok(order
        .into_iter()
        .map(|i| value[i].take().unwrap())
        .collect())
}
/// Sorted positions of `value`; `per_item` supplies a different variable scope
/// for each item (used to expose each group's `current-group()` to its sort keys).
pub(crate) fn order<'a>(
    value: &[crate::xpath::Item<'a>],
    sorts: &[Sort],
    env: &Variables<'a>,
    outer: &Focus<'a>,
    per_item: Option<&[Variables<'a>]>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<Vec<usize>, Diagnostic> {
    let size = value.len();
    let mut settings = Vec::with_capacity(sorts.len());
    for sort in sorts {
        settings.push(resolve(sort, outer, env, budget, backwards)?);
    }
    let mut keys = Vec::with_capacity(size);
    for (i, item) in value.iter().enumerate() {
        let focus = Focus {
            item: item.clone(),
            position: i + 1,
            size,
        };
        let mut row = vec![];
        for (key, setting) in sorts.iter().zip(&settings) {
            let scope = per_item.map_or(env, |scopes| &scopes[i]);
            let mut v = xpath::eval(&key.select, &focus, scope, budget, 0, backwards)?;
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
            let s = v
                .first()
                .map_or_else(String::new, |item| item.string_compat(backwards));
            row.push(if setting.numeric {
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
                    for (k, spec) in settings.iter().enumerate() {
                        budget.tick()?;
                        result = compare(
                            &keys[order[a]][k],
                            &keys[order[b]][k],
                            &spec.case_order,
                            spec.collation,
                        );
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
    Ok(order)
}
