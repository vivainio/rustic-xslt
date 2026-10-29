//! Atomized sequence lookup and duplicate removal with bounded comparison work.
use crate::{
    Diagnostic,
    deep_equal::atomic_equal,
    number::Number,
    xpath::{Budget, Item, Value, atomize},
};

pub(crate) fn evaluate<'a>(
    name: &str,
    args: &[Value<'a>],
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    let distinct = name == "distinct-values";
    if let Some(collation) = args.get(if distinct { 1 } else { 2 }) {
        let uri = crate::string_functions::string(collation, false, false)?;
        if uri != "http://www.w3.org/2005/xpath-functions/collation/codepoint" {
            return Err(Diagnostic::new("FOCH0002", "unsupported collation", "run"));
        }
    }
    let search = if distinct {
        None
    } else {
        let [item] = args[1].as_slice() else {
            return Err(Diagnostic::new(
                "XPTY0004",
                "index-of search must be one atomic value",
                "run",
            ));
        };
        Some(atomize(item))
    };
    let mut result = vec![];
    for (index, item) in args[0].iter().enumerate() {
        budget.tick()?;
        let value = atomize(item);
        if let Some(search) = &search {
            if atomic_equal(&value, search, false) {
                result.push(Item::Number(Number::from(index + 1)));
            }
        } else {
            let mut duplicate = false;
            for previous in &result {
                budget.tick()?;
                if atomic_equal(&value, previous, true) {
                    duplicate = true;
                    break;
                }
            }
            if !duplicate {
                result.push(value);
            }
        }
    }
    Ok(result)
}
