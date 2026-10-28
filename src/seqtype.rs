use crate::{
    Diagnostic,
    number::Number,
    xpath::{Item, Value, XNode},
};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    Item,
    Node,
    Document,
    Element,
    Attribute,
    Text,
    Comment,
    Pi,
    String,
    Untyped,
    Boolean,
    Integer,
    Decimal,
    Float,
    Double,
    Atomic,
    Numeric,
    Empty,
}
#[derive(Debug, Clone)]
pub(crate) struct SequenceType {
    pub kind: Kind,
    pub min: usize,
    pub max: usize,
}
impl SequenceType {
    pub fn parse(source: &str, namespaces: &BTreeMap<String, String>) -> Result<Self, Diagnostic> {
        let source = source.trim();
        let mut base = source;
        let (min, max) = match source.chars().last() {
            Some('?') => {
                base = &source[..source.len() - 1];
                (0, 1)
            }
            Some('*') => {
                base = &source[..source.len() - 1];
                (0, usize::MAX)
            }
            Some('+') => {
                base = &source[..source.len() - 1];
                (1, usize::MAX)
            }
            _ => (1, 1),
        };
        let kind = match base.trim() {
            "item()" => Kind::Item,
            "node()" => Kind::Node,
            "document-node()" => Kind::Document,
            "element()" => Kind::Element,
            "attribute()" => Kind::Attribute,
            "text()" => Kind::Text,
            "comment()" => Kind::Comment,
            "processing-instruction()" => Kind::Pi,
            "empty-sequence()" if min == 1 && max == 1 => {
                return Ok(Self {
                    kind: Kind::Empty,
                    min: 0,
                    max: 0,
                });
            }
            name => {
                let Some((p, local)) = name.split_once(':') else {
                    return Err(Diagnostic::new(
                        "XPST0051",
                        "unsupported sequence type",
                        "compile",
                    ));
                };
                if namespaces.get(p).map(String::as_str) != Some("http://www.w3.org/2001/XMLSchema")
                {
                    return Err(Diagnostic::new(
                        "XPST0051",
                        "unknown type namespace",
                        "compile",
                    ));
                }
                match local {
                    "string" => Kind::String,
                    "untypedAtomic" => Kind::Untyped,
                    "boolean" => Kind::Boolean,
                    "integer" => Kind::Integer,
                    "decimal" => Kind::Decimal,
                    "float" => Kind::Float,
                    "double" => Kind::Double,
                    "anyAtomicType" => Kind::Atomic,
                    "numeric" => Kind::Numeric,
                    _ => {
                        return Err(Diagnostic::new(
                            "XPST0051",
                            "unsupported atomic type",
                            "compile",
                        ));
                    }
                }
            }
        };
        Ok(Self { kind, min, max })
    }
    pub fn atomic(&self) -> bool {
        matches!(
            self.kind,
            Kind::String
                | Kind::Untyped
                | Kind::Boolean
                | Kind::Integer
                | Kind::Decimal
                | Kind::Float
                | Kind::Double
                | Kind::Atomic
                | Kind::Numeric
        )
    }
    fn item_matches(&self, item: &Item<'_>) -> bool {
        match (&self.kind, item) {
            (Kind::Item, Item::Absent) => false,
            (Kind::Item, _) => true,
            (Kind::Node, Item::Node(_)) => true,
            (Kind::Document, Item::Node(XNode::Tree(n))) => n.is_root(),
            (Kind::Element, Item::Node(XNode::Tree(n))) => n.is_element(),
            (Kind::Attribute, Item::Node(XNode::Attribute(..))) => true,
            (Kind::Text, Item::Node(XNode::Tree(n))) => n.is_text(),
            (Kind::Comment, Item::Node(XNode::Tree(n))) => n.is_comment(),
            (Kind::Pi, Item::Node(XNode::Tree(n))) => n.is_pi(),
            (Kind::String, Item::String(_))
            | (Kind::Untyped, Item::Untyped(_))
            | (Kind::Boolean, Item::Boolean(_)) => true,
            (Kind::Integer, Item::Number(Number::Integer(_))) => true,
            (Kind::Decimal, Item::Number(Number::Integer(_) | Number::Decimal(_))) => true,
            (Kind::Float, Item::Number(Number::Float(_)))
            | (Kind::Double, Item::Number(Number::Double(_)))
            | (Kind::Numeric, Item::Number(_)) => true,
            (
                Kind::Atomic,
                Item::String(_) | Item::Untyped(_) | Item::Boolean(_) | Item::Number(_),
            ) => true,
            _ => false,
        }
    }
    pub fn matches(&self, value: &[Item<'_>]) -> bool {
        value.len() >= self.min
            && value.len() <= self.max
            && value.iter().all(|i| self.item_matches(i))
    }
    pub fn convert<'a>(&self, value: Value<'a>, code: &str) -> Result<Value<'a>, Diagnostic> {
        let failure = || {
            Diagnostic::new(
                code,
                "value does not match its declared sequence type",
                "run",
            )
        };
        if value.len() < self.min || value.len() > self.max {
            return Err(failure());
        }
        let mut out = vec![];
        for mut item in value {
            if self.atomic()
                && let Item::Node(n) = item
            {
                item = Item::Untyped(n.string());
            }
            item = match (&self.kind, item) {
                (Kind::String, Item::Untyped(s)) => Item::String(s),
                (Kind::Boolean, Item::Untyped(s)) => {
                    Item::Boolean(match s.trim_matches([' ', '\t', '\r', '\n']) {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        _ => return Err(failure()),
                    })
                }
                (Kind::Integer | Kind::Decimal | Kind::Float | Kind::Double, Item::Untyped(s)) => {
                    let kind = match self.kind {
                        Kind::Integer => "integer",
                        Kind::Decimal => "decimal",
                        Kind::Float => "float",
                        _ => "double",
                    };
                    Item::Number(
                        Number::cast(kind, &s)
                            .map_err(|e| if e.code == "XPDY0130" { e } else { failure() })?,
                    )
                }
                (Kind::Double, Item::Number(n)) => {
                    Item::Number(n.convert("double").map_err(|_| failure())?)
                }
                (Kind::Float, Item::Number(n)) if !matches!(n, Number::Double(_)) => {
                    Item::Number(n.convert("float").map_err(|_| failure())?)
                }
                (_, item) => item,
            };
            if !self.item_matches(&item) {
                return Err(failure());
            }
            out.push(item);
        }
        Ok(out)
    }
}
