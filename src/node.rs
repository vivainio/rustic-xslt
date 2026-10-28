//! XDM node handles retain document identity and can hide a fragment parser's
//! synthetic wrapper behind a real document-node view.
use roxmltree::Node;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct XNode<'a> {
    pub raw: Node<'a, 'a>,
    pub attribute: Option<(&'a str, Option<&'a str>)>,
    fragment: bool,
    boundary: Option<Node<'a, 'a>>,
}
impl<'a> XNode<'a> {
    pub fn tree(raw: Node<'a, 'a>) -> Self {
        Self {
            raw,
            attribute: None,
            fragment: false,
            boundary: None,
        }
    }
    pub fn fragment(raw: Node<'a, 'a>) -> Self {
        Self {
            raw,
            attribute: None,
            fragment: true,
            boundary: None,
        }
    }
    pub fn relative(self, raw: Node<'a, 'a>) -> Self {
        Self {
            raw,
            attribute: None,
            fragment: self.fragment,
            boundary: self.boundary,
        }
    }
    pub fn detached(mut self) -> Self {
        self.boundary = Some(self.raw);
        self.fragment = false;
        self
    }
    pub fn is_attribute(self) -> bool {
        self.attribute.is_some()
    }
    pub fn is_document(self) -> bool {
        !self.is_attribute()
            && if self.fragment {
                self.raw == self.raw.document().root_element()
            } else {
                self.raw.is_root()
            }
    }
    pub fn is_element(self) -> bool {
        !self.is_attribute() && !self.is_document() && self.raw.is_element()
    }
    pub fn is_text(self) -> bool {
        !self.is_attribute() && self.raw.is_text()
    }
    pub fn is_comment(self) -> bool {
        !self.is_attribute() && self.raw.is_comment()
    }
    pub fn is_pi(self) -> bool {
        !self.is_attribute() && self.raw.is_pi()
    }
    pub fn string(self) -> String {
        if let Some((local, uri)) = self.attribute {
            return self
                .raw
                .attributes()
                .find(|a| a.name() == local && a.namespace() == uri)
                .map_or("", |a| a.value())
                .into();
        }
        if self.is_document() || self.is_element() {
            return self
                .raw
                .descendants()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .collect();
        }
        self.raw
            .pi()
            .and_then(|pi| pi.value)
            .or_else(|| self.raw.text())
            .unwrap_or_default()
            .into()
    }
    pub fn parent(self) -> Option<Self> {
        if self.is_attribute() {
            Some(self.relative(self.raw))
        } else if self.is_document() || self.boundary == Some(self.raw) {
            None
        } else {
            self.raw.parent().map(|n| self.relative(n))
        }
    }
    pub fn root(self) -> Self {
        self.relative(if let Some(boundary) = self.boundary {
            boundary
        } else if self.fragment {
            self.raw.document().root_element()
        } else {
            self.raw.document().root()
        })
    }
    pub fn name(self) -> (Option<&'a str>, &'a str) {
        if let Some((local, uri)) = self.attribute {
            (uri, local)
        } else if self.is_element() {
            (
                self.raw
                    .tag_name()
                    .namespace()
                    .filter(|uri| !uri.is_empty()),
                self.raw.tag_name().name(),
            )
        } else {
            (None, self.raw.pi().map_or("", |pi| pi.target))
        }
    }
    pub fn children(self) -> impl DoubleEndedIterator<Item = Self> {
        self.raw
            .children()
            .filter(move |_| !self.is_attribute())
            .map(move |n| self.relative(n))
    }
    pub fn attributes(self) -> Vec<Self> {
        if !self.is_element() {
            return vec![];
        }
        self.raw
            .attributes()
            .map(|a| Self {
                raw: self.raw,
                attribute: Some((a.name(), a.namespace())),
                fragment: self.fragment,
                boundary: self.boundary,
            })
            .collect()
    }
    pub fn descendants(self) -> Vec<Self> {
        if self.is_attribute() {
            vec![self]
        } else {
            self.raw.descendants().map(|n| self.relative(n)).collect()
        }
    }
    pub fn order(self) -> (usize, u32, bool, &'a str, Option<&'a str>) {
        let (local, uri) = self.attribute.unwrap_or(("", None));
        (
            std::ptr::from_ref(self.raw.document()) as usize,
            self.raw.id().get(),
            self.is_attribute(),
            local,
            uri,
        )
    }
}
