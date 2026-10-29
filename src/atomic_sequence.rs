//! Atomized sequence lookup and duplicate removal with bounded comparison work.
use crate::{
    Diagnostic,
    collation::Collation,
    deep_equal::atomic_equal_with,
    number::Number,
    xpath::{Budget, Item, Value, atomize},
};

pub(crate) fn evaluate<'a>(
    name: &str,
    args: &[Value<'a>],
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    let distinct = name == "distinct-values";
    let collation = match args.get(if distinct { 1 } else { 2 }) {
        Some(uri) => Collation::parse(&crate::string_functions::string(uri, false, false)?)?,
        None => Collation::Codepoint,
    };
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
            if atomic_equal_with(&value, search, false, collation) {
                result.push(Item::Number(Number::from(index + 1)));
            }
        } else {
            let mut duplicate = false;
            for previous in &result {
                budget.tick()?;
                if atomic_equal_with(&value, previous, true, collation) {
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
