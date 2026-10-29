//! XDM node handles retain document identity and can hide a fragment parser's
//! synthetic wrapper behind a real document-node view.
use roxmltree::Node;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct XNode<'a> {
    pub raw: Node<'a, 'a>,
    pub attribute: Option<(&'a str, Option<&'a str>)>,
    fragment: bool,
    detached_attribute: bool,
    empty_text: bool,
    boundary: Option<Node<'a, 'a>>,
}
impl<'a> XNode<'a> {
    pub fn tree(raw: Node<'a, 'a>) -> Self {
        Self {
            raw,
            attribute: None,
            fragment: false,
            detached_attribute: false,
            empty_text: false,
            boundary: None,
        }
    }
    pub fn fragment(raw: Node<'a, 'a>) -> Self {
        Self {
            raw,
            attribute: None,
            fragment: true,
            detached_attribute: false,
            empty_text: false,
            boundary: None,
        }
    }
    pub fn relative(self, raw: Node<'a, 'a>) -> Self {
        Self {
            raw,
            attribute: None,
            fragment: self.fragment,
            detached_attribute: false,
            empty_text: self.empty_text && raw == self.raw,
            boundary: self.boundary,
        }
    }
    pub fn detached(mut self) -> Self {
        self.detached_attribute = self.is_attribute();
        self.boundary = Some(self.raw);
        self.fragment = false;
        self
    }
    // The parser cannot retain empty text nodes. A detached backing text node
    // supplies identity while this flag supplies its actual XDM string value.
    pub fn empty_text(mut self) -> Self {
        debug_assert!(self.is_text() && self.parent().is_none());
        self.empty_text = true;
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
        if self.empty_text {
            return String::new();
        }
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
            if self.detached_attribute {
                None
            } else {
                Some(self.relative(self.raw))
            }
        } else if self.is_document() || self.boundary == Some(self.raw) {
            None
        } else {
            self.raw.parent().map(|n| self.relative(n))
        }
    }
    pub fn root(self) -> Self {
        if self.detached_attribute {
            return self;
        }
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
                detached_attribute: false,
                empty_text: false,
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
