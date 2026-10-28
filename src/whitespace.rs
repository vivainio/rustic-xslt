use crate::{Diagnostic, Stylesheet};
#[derive(Debug)]
pub(crate) struct Rule {
    pub uri: Option<String>,
    pub local: Option<String>,
    pub specificity: u8,
    pub strip: bool,
}
pub(crate) fn xml_space(s: &str) -> bool {
    s.chars().all(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
}
fn strip(sheet: &Stylesheet, parent: roxmltree::Node<'_, '_>, inherited_preserve: bool) -> bool {
    let preserve = parent
        .ancestors()
        .filter_map(|n| n.attribute(("http://www.w3.org/XML/1998/namespace", "space")))
        .next()
        .map_or(inherited_preserve, |v| v == "preserve");
    if preserve {
        return false;
    }
    sheet
        .space_rules
        .iter()
        .filter(|r| {
            r.uri
                .as_deref()
                .is_none_or(|u| parent.tag_name().namespace().unwrap_or("") == u)
                && r.local
                    .as_deref()
                    .is_none_or(|l| parent.tag_name().name() == l)
        })
        .max_by_key(|r| r.specificity)
        .is_some_and(|r| r.strip)
}
/// roxmltree is immutable, so remove only the lexical ranges corresponding to
/// stripped text nodes and reparse. No markup/attribute reserialization occurs.
pub(crate) fn prepare(
    sheet: &Stylesheet,
    source: &str,
    inherited_preserve: bool,
) -> Result<Option<String>, Diagnostic> {
    if sheet.space_rules.is_empty() || source.is_empty() {
        return Ok(None);
    }
    let doc = crate::parse(source, "parse")?;
    let mut ranges = vec![];
    for n in doc.descendants().filter(|n| n.is_text()) {
        if xml_space(n.text().unwrap_or_default())
            && n.parent()
                .filter(|n| n.is_element())
                .is_some_and(|p| strip(sheet, p, inherited_preserve))
        {
            // Adjacent character data and CDATA are coalesced, but roxmltree's
            // reported range covers only the first lexical text segment.
            let parent = n.parent().unwrap();
            let end = n
                .next_sibling()
                .map(|next| next.range().start)
                .unwrap_or_else(|| {
                    let range = parent.range();
                    range.start
                        + source[range]
                            .rfind("</")
                            .expect("text parent has a closing tag")
                });
            ranges.push(n.range().start..end);
        }
    }
    if ranges.is_empty() {
        return Ok(None);
    }
    ranges.sort_by_key(|r| r.start);
    let mut value = String::with_capacity(source.len());
    let mut last = 0;
    for r in ranges {
        if r.start >= last {
            value.push_str(&source[last..r.start]);
            last = r.end;
        }
    }
    value.push_str(&source[last..]);
    Ok(Some(value))
}
