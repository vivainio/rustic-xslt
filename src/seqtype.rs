use crate::{
    Diagnostic,
    number::Number,
    xpath::{Item, Value},
};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    Item,
    Node,
    Document,
    Element,
    NamedElement(crate::xpath::Name),
    Attribute,
    NamedAttribute(crate::xpath::Name),
    DocumentElement(Option<crate::xpath::Name>),
    Text,
    Comment,
    Pi,
    String,
    DerivedString(crate::string_type::StringType),
    Untyped,
    AnyUri,
    Boolean,
    QName,
    Binary(crate::binary::BinaryType),
    Function(Option<usize>),
    Temporal(crate::temporal::TemporalType),
    Integer,
    DerivedInteger(crate::integer_type::IntegerType),
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
fn node_name(
    source: &str,
    namespaces: &BTreeMap<String, String>,
) -> Result<Option<crate::xpath::Name>, Diagnostic> {
    let source = source.trim();
    if source.is_empty() || source == "*" {
        return Ok(None);
    }
    let local = if source.starts_with("Q{") {
        source.split_once('}').map_or(source, |(_, local)| local)
    } else {
        source
    };
    if local.contains(',') {
        return Err(Diagnostic::new(
            "RUST_XSLT_UNSUPPORTED",
            "node type annotations are not implemented",
            "compile",
        ));
    }
    let canonical = crate::qname::resolve(source, namespaces, "XPST0003")?;
    let (uri, local) = crate::qname::parts(&canonical);
    Ok(Some(crate::xpath::Name {
        uri: uri.map(str::to_owned),
        local: local.into(),
    }))
}
fn named_node(node: crate::node::XNode<'_>, name: &crate::xpath::Name) -> bool {
    let (uri, local) = node.name();
    uri == name.uri.as_deref() && local == name.local
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
            name if name.starts_with("function(") && name.ends_with(')') => {
                let inner = name[9..name.len() - 1].trim();
                Kind::Function(if inner == "*" {
                    None
                } else if inner.is_empty() {
                    Some(0)
                } else {
                    let mut depth = 0usize;
                    let mut count = 1;
                    for c in inner.chars() {
                        match c {
                            '(' => depth += 1,
                            ')' => depth = depth.saturating_sub(1),
                            ',' if depth == 0 => count += 1,
                            _ => {}
                        }
                    }
                    Some(count)
                })
            }
            name if name.starts_with("element(") && name.ends_with(')') => {
                match node_name(&name[8..name.len() - 1], namespaces)? {
                    Some(name) => Kind::NamedElement(name),
                    None => Kind::Element,
                }
            }
            name if name.starts_with("attribute(") && name.ends_with(')') => {
                match node_name(&name[10..name.len() - 1], namespaces)? {
                    Some(name) => Kind::NamedAttribute(name),
                    None => Kind::Attribute,
                }
            }
            name if name.starts_with("document-node(") && name.ends_with(')') => {
                let inner = name[14..name.len() - 1].trim();
                if !inner.starts_with("element(") || !inner.ends_with(')') {
                    return Err(Diagnostic::new(
                        "XPST0003",
                        "document type requires an element test",
                        "compile",
                    ));
                }
                Kind::DocumentElement(node_name(&inner[8..inner.len() - 1], namespaces)?)
            }
            name => {
                let canonical = crate::qname::resolve(name, namespaces, "XPST0003")?;
                let (uri, local) = crate::qname::parts(&canonical);
                if uri != Some("http://www.w3.org/2001/XMLSchema") {
                    return Err(Diagnostic::new(
                        "XPST0051",
                        "unknown type namespace",
                        "compile",
                    ));
                }
                match local {
                    "string" => Kind::String,
                    "untypedAtomic" => Kind::Untyped,
                    "anyURI" => Kind::AnyUri,
                    "boolean" => Kind::Boolean,
                    "QName" => Kind::QName,
                    name if crate::binary::BinaryType::parse(name).is_some() => {
                        Kind::Binary(crate::binary::BinaryType::parse(name).unwrap())
                    }
                    name if crate::temporal::TemporalType::parse(name).is_some() => {
                        Kind::Temporal(crate::temporal::TemporalType::parse(name).unwrap())
                    }
                    "integer" => Kind::Integer,
                    "decimal" => Kind::Decimal,
                    "float" => Kind::Float,
                    "double" => Kind::Double,
                    "anyAtomicType" => Kind::Atomic,
                    "numeric" => Kind::Numeric,
                    name if crate::string_type::StringType::parse(name).is_some() => {
                        Kind::DerivedString(crate::string_type::StringType::parse(name).unwrap())
                    }
                    name if crate::integer_type::IntegerType::parse(name).is_some() => {
                        Kind::DerivedInteger(crate::integer_type::IntegerType::parse(name).unwrap())
                    }
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
                | Kind::DerivedString(_)
                | Kind::Untyped
                | Kind::AnyUri
                | Kind::Boolean
                | Kind::QName
                | Kind::Binary(_)
                | Kind::Temporal(_)
                | Kind::Integer
                | Kind::DerivedInteger(_)
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
            (Kind::Document, Item::Node(n)) => n.is_document(),
            (Kind::Element, Item::Node(n)) => n.is_element(),
            (Kind::NamedElement(name), Item::Node(n)) => n.is_element() && named_node(*n, name),
            (Kind::Attribute, Item::Node(n)) => n.is_attribute(),
            (Kind::NamedAttribute(name), Item::Node(n)) => n.is_attribute() && named_node(*n, name),
            (Kind::DocumentElement(name), Item::Node(n)) => {
                if !n.is_document() {
                    return false;
                }
                let mut element = None;
                for child in n.children() {
                    if child.is_element() {
                        if element.replace(child).is_some() {
                            return false;
                        }
                    } else if !child.is_comment() && !child.is_pi() {
                        return false;
                    }
                }
                element.is_some_and(|n| name.as_ref().is_none_or(|name| named_node(n, name)))
            }
            (Kind::Text, Item::Node(n)) => n.is_text(),
            (Kind::Comment, Item::Node(n)) => n.is_comment(),
            (Kind::Pi, Item::Node(n)) => n.is_pi(),
            (Kind::DerivedString(target), Item::DerivedString(actual, _)) => {
                actual.subtype_of(*target)
            }
            (Kind::String, Item::DerivedString(_, _)) => true,
            (Kind::String, Item::String(_))
            | (Kind::Untyped, Item::Untyped(_))
            | (Kind::AnyUri, Item::AnyUri(_))
            | (Kind::Boolean, Item::Boolean(_))
            | (Kind::QName, Item::QName(_)) => true,
            (Kind::Binary(target), Item::Binary(actual, _)) => target == actual,
            (Kind::Function(arity), Item::Function(f)) => arity.is_none_or(|a| a == f.arity()),
            (Kind::Temporal(target), Item::Temporal(actual)) => target.accepts(actual.kind),
            (Kind::Integer, Item::Number(Number::Integer(_) | Number::DerivedInteger(_, _))) => {
                true
            }
            (Kind::DerivedInteger(target), Item::Number(Number::DerivedInteger(actual, _))) => {
                actual.subtype_of(*target)
            }
            (
                Kind::Decimal,
                Item::Number(
                    Number::Integer(_) | Number::DerivedInteger(_, _) | Number::Decimal(_),
                ),
            ) => true,
            (Kind::Float, Item::Number(Number::Float(_)))
            | (Kind::Double, Item::Number(Number::Double(_)))
            | (Kind::Numeric, Item::Number(_)) => true,
            (
                Kind::Atomic,
                Item::String(_)
                | Item::DerivedString(_, _)
                | Item::AnyUri(_)
                | Item::Untyped(_)
                | Item::Boolean(_)
                | Item::Number(_)
                | Item::QName(_)
                | Item::Binary(_, _)
                | Item::Temporal(_),
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
            if self.atomic() {
                item = crate::xpath::atomize(&item);
            }
            item = match (&self.kind, item) {
                (Kind::String, Item::Untyped(s) | Item::AnyUri(s)) => Item::String(s),
                (Kind::DerivedString(kind), Item::Untyped(s)) => {
                    Item::DerivedString(*kind, kind.cast(&s).map_err(|_| failure())?)
                }
                (Kind::AnyUri, Item::Untyped(s)) => {
                    Item::AnyUri(crate::string_functions::collapse_xml_whitespace(&s))
                }
                (Kind::Binary(kind), Item::Untyped(s)) => {
                    Item::Binary(*kind, kind.decode(&s).map_err(|_| failure())?)
                }
                (Kind::Temporal(kind), Item::Untyped(s)) => Item::Temporal(
                    crate::temporal::Temporal::parse(*kind, &s).map_err(|_| failure())?,
                ),
                (Kind::Boolean, Item::Untyped(s)) => {
                    Item::Boolean(match s.trim_matches([' ', '\t', '\r', '\n']) {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        _ => return Err(failure()),
                    })
                }
                (
                    Kind::Integer
                    | Kind::DerivedInteger(_)
                    | Kind::Decimal
                    | Kind::Float
                    | Kind::Double,
                    Item::Untyped(s),
                ) => {
                    let kind = match self.kind {
                        Kind::Integer => "integer",
                        Kind::DerivedInteger(kind) => kind.name(),
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
