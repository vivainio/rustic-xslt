//! Per-transformation arenas own temporary XML strings and parsed documents.
//! Documents are dropped before their source strings; no leaks or unsafe code.
use crate::{
    Diagnostic,
    node::XNode,
    seqtype::SequenceType,
    xpath::{Item, Value},
};
use std::cell::Cell;
use typed_arena::Arena;
pub(crate) struct Trees<'a> {
    pub sources: &'a Arena<String>,
    pub documents: &'a Arena<roxmltree::Document<'a>>,
    max: usize,
    used: Cell<usize>,
}
impl<'a> Trees<'a> {
    pub fn new(
        sources: &'a Arena<String>,
        documents: &'a Arena<roxmltree::Document<'a>>,
        max: usize,
    ) -> Self {
        Self {
            sources,
            documents,
            max,
            used: Cell::new(0),
        }
    }
    pub fn construct(
        &self,
        xml: String,
        ty: Option<&SequenceType>,
    ) -> Result<Value<'a>, Diagnostic> {
        let size = xml
            .len()
            .checked_add("<_rust_xslt_document></_rust_xslt_document>".len())
            .ok_or_else(|| Diagnostic::new("XPDY0130", "temporary-tree limit exceeded", "run"))?;
        if size > self.max.saturating_sub(self.used.get()) {
            return Err(Diagnostic::new(
                "XPDY0130",
                "temporary-tree source-byte limit exceeded",
                "run",
            ));
        }
        self.used.set(self.used.get() + size);
        let text: &'a str = self
            .sources
            .alloc(format!("<_rust_xslt_document>{xml}</_rust_xslt_document>"))
            .as_str();
        let doc: &'a roxmltree::Document<'a> = self.documents.alloc(crate::parse(text, "run")?);
        let root = XNode::fragment(doc.root_element());
        if let Some(ty) = ty {
            ty.convert(root.children().map(Item::Node).collect(), "XTTE0570")
        } else {
            Ok(vec![Item::Node(root)])
        }
    }
}
