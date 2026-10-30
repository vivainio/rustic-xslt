//! XPath expression machinery. The AST mirrors the reference engine's separation
//! of literals, binary expressions, function calls and axis steps.
pub(crate) use crate::node::XNode;
use crate::{Diagnostic, Limits, number::Number};
use num_traits::ToPrimitive;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Name {
    pub uri: Option<String>,
    pub local: String,
}
#[derive(Debug, Clone)]
pub(crate) enum Test {
    Name(Name),
    Any,
    Node,
    Text,
    Comment,
    Pi(Option<String>),
    Namespace(String),
    Local(String),
    Document,
    Element(Option<Name>),
    AttributeKind(Option<Name>),
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Axis {
    Child,
    Attribute,
    SelfNode,
    Parent,
    Descendant,
    DescendantOrSelf,
    Ancestor,
    AncestorOrSelf,
    FollowingSibling,
    PrecedingSibling,
    Following,
    Preceding,
}
#[derive(Debug, Clone)]
pub(crate) struct Step {
    pub axis: Axis,
    pub test: Test,
    pub predicates: Vec<Expr>,
}
#[derive(Debug, Clone)]
pub(crate) enum Expr {
    String(String),
    Number(Number),
    Variable(String),
    For(String, Box<Expr>, Box<Expr>),
    Let(String, Box<Expr>, Box<Expr>),
    Quantified(bool, String, Box<Expr>, Box<Expr>),
    Context,
    Root,
    Path(Box<Expr>, Vec<Step>),
    PathMap(Box<Expr>, Box<Expr>),
    Filter(Box<Expr>, Vec<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Unary(bool, Box<Expr>),
    Call(String, Vec<Expr>),
    StaticName(Box<Expr>, BTreeMap<String, String>, &'static str),
    LookupContext(Box<Expr>, BTreeMap<String, String>),
    Sequence(Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    TypeOp(String, Box<Expr>, crate::seqtype::SequenceType),
    QNameCast(Box<Expr>, BTreeMap<String, String>, bool, bool),
    InlineFunction(std::sync::Arc<InlineFunction>),
    FunctionRef(String, usize, Option<BTreeMap<String, String>>),
    DynamicCall(Box<Expr>, Vec<Expr>),
    Partial(Box<Expr>, Vec<Option<Expr>>),
    /// `xsl:perform-sort`: the value of the first expression ordered by the sort keys.
    Sorted(Box<Expr>, Vec<crate::compile::Sort>),
    /// `xsl:evaluate`: parse and run an XPath string with the given context and variables.
    Evaluate {
        xpath: Box<Expr>,
        context: Option<Box<Expr>>,
        params: Vec<(String, Expr)>,
        namespaces: BTreeMap<String, String>,
    },
    MapConstructor(Vec<(Expr, Expr)>),
    ArraySquare(Vec<Expr>),
    ArrayCurly(Box<Expr>),
    /// `base?key`; `None` is the wildcard `?*`.
    Lookup(Box<Expr>, Option<Box<Expr>>),
}
#[derive(Debug)]
pub(crate) struct InlineFunction {
    pub params: Vec<(String, Option<crate::seqtype::SequenceType>)>,
    pub ret: Option<crate::seqtype::SequenceType>,
    pub body: Expr,
}
/// A map keeps insertion order; keys are atomic values compared with `op:same-key`.
/// A hash index over normalized keys keeps lookups close to constant time.
#[derive(Debug, Clone, Default)]
pub(crate) struct MapValue<'a> {
    entries: Vec<(Item<'a>, Value<'a>)>,
    index: std::collections::HashMap<u64, Vec<usize>>,
}
fn key_hash(key: &Item<'_>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match key {
        Item::String(s) | Item::Untyped(s) | Item::AnyUri(s) | Item::DerivedString(_, s) => {
            (0u8, s).hash(&mut hasher)
        }
        Item::Number(n) => {
            let d = n.double();
            if d.is_nan() {
                1u8.hash(&mut hasher);
            } else {
                (2u8, if d == 0.0 { 0.0f64 } else { d }.to_bits()).hash(&mut hasher);
            }
        }
        Item::Boolean(b) => (3u8, b).hash(&mut hasher),
        Item::QName(q) => (4u8, &q.uri, &q.local).hash(&mut hasher),
        Item::Binary(_, bytes) => (5u8, bytes).hash(&mut hasher),
        _ => 6u8.hash(&mut hasher),
    }
    hasher.finish()
}
impl<'a> MapValue<'a> {
    fn position(&self, key: &Item<'_>) -> Option<usize> {
        self.index
            .get(&key_hash(key))?
            .iter()
            .copied()
            .find(|i| crate::deep_equal::atomic_equal(&self.entries[*i].0, key, true))
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, (Item<'a>, Value<'a>)> {
        self.entries.iter()
    }
    pub fn get(&self, key: &Item<'_>) -> Option<&Value<'a>> {
        self.position(key).map(|i| &self.entries[i].1)
    }
    pub fn get_mut(&mut self, key: &Item<'_>) -> Option<&mut Value<'a>> {
        self.position(key).map(|i| &mut self.entries[i].1)
    }
    /// Append an entry whose key is known to be absent.
    pub fn push_new(&mut self, key: Item<'a>, value: Value<'a>) {
        self.index
            .entry(key_hash(&key))
            .or_default()
            .push(self.entries.len());
        self.entries.push((key, value));
    }
    pub fn insert(&mut self, key: Item<'a>, value: Value<'a>) {
        match self.get_mut(&key) {
            Some(existing) => *existing = value,
            None => self.push_new(key, value),
        }
    }
    pub fn remove(&mut self, key: &Item<'_>) {
        if let Some(position) = self.position(key) {
            self.entries.remove(position);
            self.index.clear();
            for (i, (k, _)) in self.entries.iter().enumerate() {
                self.index.entry(key_hash(k)).or_default().push(i);
            }
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) enum FunctionValue<'a> {
    Inline(std::sync::Arc<InlineFunction>, Variables<'a>),
    Named(String, usize, Option<BTreeMap<String, String>>),
    /// current#0 retains the current item at the point the reference is created.
    Current(Item<'a>),
    /// A map or array used where a function item is expected.
    Map(std::rc::Rc<MapValue<'a>>),
    Array(std::rc::Rc<Vec<Value<'a>>>),
    /// A function with some arguments fixed; `None` slots are still open.
    Partial(std::rc::Rc<FunctionValue<'a>>, Vec<Option<Value<'a>>>),
}
impl FunctionValue<'_> {
    pub fn arity(&self) -> usize {
        match self {
            Self::Inline(def, _) => def.params.len(),
            Self::Named(_, arity, _) => *arity,
            Self::Current(_) => 0,
            Self::Map(_) | Self::Array(_) => 1,
            Self::Partial(_, slots) => slots.iter().filter(|s| s.is_none()).count(),
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) enum Item<'a> {
    Absent,
    Node(XNode<'a>),
    String(String),
    Untyped(String),
    AnyUri(String),
    DerivedString(crate::string_type::StringType, String),
    Number(Number),
    Boolean(bool),
    QName(crate::qname::AtomicQName),
    Binary(crate::binary::BinaryType, Vec<u8>),
    Temporal(crate::temporal::Temporal),
    Function(std::rc::Rc<FunctionValue<'a>>),
    Map(std::rc::Rc<MapValue<'a>>),
    Array(std::rc::Rc<Vec<Value<'a>>>),
}
pub(crate) type Value<'a> = Vec<Item<'a>>;
pub(crate) type Variables<'a> = BTreeMap<String, Value<'a>>;
pub(crate) fn atomize<'a>(item: &Item<'a>) -> Item<'a> {
    match item {
        Item::Node(n) if n.is_comment() || n.is_pi() => Item::String(n.string()),
        Item::Node(n) => Item::Untyped(n.string()),
        _ => item.clone(),
    }
}
impl Item<'_> {
    pub fn string(&self) -> String {
        match self {
            Self::Absent => String::new(),
            Self::Node(n) => n.string(),
            Self::String(s) | Self::Untyped(s) | Self::AnyUri(s) | Self::DerivedString(_, s) => {
                s.clone()
            }
            Self::Number(n) => n.string(),
            Self::Boolean(b) => b.to_string(),
            Self::QName(name) => name.lexical(),
            Self::Binary(kind, bytes) => kind.encode(bytes),
            Self::Temporal(t) => t.string(),
            Self::Function(_) | Self::Map(_) | Self::Array(_) => String::new(),
        }
    }
    pub fn string_compat(&self, backwards: bool) -> String {
        if backwards && let Self::Number(n) = self {
            n.legacy_string()
        } else {
            self.string()
        }
    }
    fn number(&self) -> f64 {
        match self {
            Self::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Number(n) => n.double(),
            Self::AnyUri(_)
            | Self::QName(_)
            | Self::Binary(_, _)
            | Self::Temporal(_)
            | Self::Function(_)
            | Self::Map(_)
            | Self::Array(_) => f64::NAN,
            _ => self.string().trim().parse().unwrap_or(f64::NAN),
        }
    }
}
#[derive(Clone)]
pub(crate) struct Focus<'a> {
    pub item: Item<'a>,
    pub position: usize,
    pub size: usize,
}
/// A stylesheet function lowered to an XPath expression.
#[derive(Debug)]
pub(crate) struct UserFunction {
    pub params: Vec<(String, Option<crate::seqtype::SequenceType>)>,
    pub body: Expr,
    pub result: Option<crate::seqtype::SequenceType>,
}
/// Global variables are also bound under this prefix so function bodies can reach
/// them even when a caller's local variable shadows the plain name.
pub(crate) const GLOBAL_PREFIX: &str = "\u{0}global:";
/// One `xsl:key` declaration: nodes matching `branches` are indexed by the `use` values.
#[derive(Debug)]
pub(crate) struct KeyDefinition {
    pub branches: Vec<crate::pattern::Branch>,
    pub value: Expr,
    pub backwards: bool,
    pub composite: bool,
    pub collation: crate::collation::Collation,
    pub collation_uri: String,
}
/// Variable names under which `xsl:for-each-group` publishes the current group.
pub(crate) const CURRENT_ITEM: &str = "\u{0}current-item";
pub(crate) const CURRENT_GROUP: &str = "\u{0}current-group";
pub(crate) const REGEX_GROUPS: &str = "\u{0}regex-groups";
pub(crate) const CURRENT_KEY: &str = "\u{0}current-grouping-key";
pub(crate) type DecimalFormats = std::sync::Arc<BTreeMap<String, crate::format_number::Format>>;
thread_local! {
    static DOCUMENT_URIS: std::cell::RefCell<std::collections::BTreeSet<String>> =
        const { std::cell::RefCell::new(std::collections::BTreeSet::new()) };
}
/// Literal URIs passed to doc(), document() and doc-available() since the last call.
pub(crate) fn take_document_uris() -> Vec<String> {
    DOCUMENT_URIS.with(|uris| {
        std::mem::take(&mut *uris.borrow_mut())
            .into_iter()
            .collect()
    })
}
/// Global-variable key under which a preloaded document is bound.
pub(crate) fn document_key(uri: &str) -> String {
    format!("\u{0}doc:{uri}")
}
/// Cached key data owns its atomic values and stores node IDs, never borrowed nodes.
#[derive(Debug)]
struct KeyEntry {
    node: roxmltree::NodeId,
    attribute: Option<(String, Option<String>)>,
    values: Value<'static>,
}
type KeyIdentity = (String, usize, u32);
type KeyIndex = std::rc::Rc<Vec<KeyEntry>>;
pub(crate) type KeyTable = std::sync::Arc<BTreeMap<String, Vec<KeyDefinition>>>;
pub(crate) type FunctionTable = std::sync::Arc<BTreeMap<(String, usize), UserFunction>>;
pub(crate) struct Budget {
    pub functions: FunctionTable,
    pub keys: KeyTable,
    pub xpath_only: bool,
    /// Keys currently being evaluated, identified by expanded name and document root.
    active_keys: Vec<KeyIdentity>,
    key_indexes: BTreeMap<KeyIdentity, KeyIndex>,
    pub decimal_formats: DecimalFormats,
    /// Secondary results (href, serialized content) created by `xsl:result-document`.
    pub secondary: Vec<(String, String)>,
    pub result_destinations: std::collections::BTreeSet<String>,
    /// Serialized principal result emitted by an explicit href-less result document.
    pub principal_output: String,
    pub principal_output_claimed: bool,
    /// Text of the non-terminating `xsl:message` instructions, in order.
    pub messages: Vec<String>,
    pub resolver: Option<crate::ResourceResolver>,
    remaining: u64,
    pub max_depth: usize,
    pub now: crate::temporal::Temporal,
}
impl Budget {
    pub fn new(limits: Limits) -> Self {
        Self {
            functions: FunctionTable::default(),
            keys: KeyTable::default(),
            xpath_only: false,
            active_keys: vec![],
            key_indexes: BTreeMap::new(),
            decimal_formats: DecimalFormats::default(),
            secondary: vec![],
            result_destinations: Default::default(),
            principal_output: String::new(),
            principal_output_claimed: false,
            messages: vec![],
            resolver: None,
            remaining: limits.max_steps,
            max_depth: limits.max_depth,
            now: crate::temporal::Temporal::now(),
        }
    }
    pub fn charge(&mut self, steps: usize) -> Result<(), Diagnostic> {
        let steps =
            u64::try_from(steps).map_err(|_| err("XPDY0130", "execution limit exceeded"))?;
        self.remaining = self
            .remaining
            .checked_sub(steps)
            .ok_or_else(|| err("XPDY0130", "execution limit exceeded"))?;
        Ok(())
    }
    pub fn tick(&mut self) -> Result<(), Diagnostic> {
        if self.remaining == 0 {
            return Err(err("XPDY0130", "execution limit exceeded"));
        }
        self.remaining -= 1;
        Ok(())
    }
}
fn err(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(code, message, "run")
}
pub(crate) fn boolean(value: &[Item<'_>]) -> Result<bool, Diagnostic> {
    match value {
        [] => Ok(false),
        [Item::Node(_), ..] => Ok(true),
        [Item::Boolean(b)] => Ok(*b),
        [Item::String(s) | Item::Untyped(s) | Item::AnyUri(s) | Item::DerivedString(_, s)] => {
            Ok(!s.is_empty())
        }
        [Item::Number(n)] => Ok(n.boolean()),
        _ => Err(err(
            "FORG0006",
            "effective boolean value requires a node sequence or a single atomic value",
        )),
    }
}
/// XSLT simple-content construction merges adjacent text nodes before
/// atomization; ordinary XPath string-join intentionally does not.
pub(crate) fn simple_string(
    value: &[Item<'_>],
    separator: &str,
    max: usize,
    backwards: bool,
) -> Result<String, Diagnostic> {
    let mut parts: Vec<String> = vec![];
    let mut previous_text = false;
    let mut bytes = 0usize;
    for item in value {
        let text = matches!(item,Item::Node(n) if n.is_text());
        let value = item.string_compat(backwards);
        if value.len() > max.saturating_sub(bytes) {
            return Err(err("XPDY0130", "simple-content byte limit exceeded"));
        }
        bytes += value.len();
        if text && value.is_empty() {
            continue;
        }
        if text && previous_text {
            parts.last_mut().unwrap().push_str(&value);
        } else {
            parts.push(value);
        }
        previous_text = text;
    }
    if parts
        .len()
        .saturating_sub(1)
        .saturating_mul(separator.len())
        > max.saturating_sub(bytes)
    {
        return Err(err("XPDY0130", "simple-content byte limit exceeded"));
    }
    Ok(parts.join(separator))
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Name(String),
    String(String),
    Number(Number),
    Symbol(String),
    End,
}
struct Parser {
    tokens: Vec<Token>,
    index: usize,
    namespaces: BTreeMap<String, String>,
    depth: usize,
}
fn syntax(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("XPST0003", message, "compile")
}
fn tokenize(s: &str) -> Result<Vec<Token>, Diagnostic> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = vec![];
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, ' ' | '\t' | '\r' | '\n') {
            i += 1;
            continue;
        }
        if c == '(' && chars.get(i + 1) == Some(&':') {
            let mut nesting = 1usize;
            i += 2;
            while i < chars.len() && nesting > 0 {
                if chars[i] == '(' && chars.get(i + 1) == Some(&':') {
                    nesting += 1;
                    if nesting > 128 {
                        return Err(syntax("XPath comment nesting limit exceeded"));
                    }
                    i += 2;
                } else if chars[i] == ':' && chars.get(i + 1) == Some(&')') {
                    nesting -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if nesting != 0 {
                return Err(syntax("unterminated XPath comment"));
            }
            continue;
        }
        if c == '\'' || c == '"' {
            i += 1;
            let mut text = String::new();
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == c {
                    i += 1;
                    if chars.get(i) == Some(&c) {
                        text.push(c);
                        i += 1;
                    } else {
                        closed = true;
                        break;
                    }
                } else {
                    text.push(chars[i]);
                    i += 1;
                }
            }
            if !closed {
                return Err(syntax("unterminated string literal"));
            }
            out.push(Token::String(text));
        } else if c.is_ascii_digit()
            || c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)
        {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if chars.get(i).is_some_and(|c| *c == 'e' || *c == 'E') {
                i += 1;
                if chars.get(i).is_some_and(|c| *c == '+' || *c == '-') {
                    i += 1;
                }
                while chars.get(i).is_some_and(char::is_ascii_digit) {
                    i += 1;
                }
            }
            if chars.get(i).is_some_and(|c| c.is_alphabetic() || *c == '_') {
                return Err(syntax("numeric literal must be separated from a name"));
            }
            let n: String = chars[start..i].iter().collect();
            out.push(Token::Number(Number::literal(&n).map_err(|mut e| {
                e.phase = "compile";
                e
            })?));
        } else if c == 'Q' && chars.get(i + 1) == Some(&'{') {
            let start = i;
            i += 2;
            while i < chars.len() && chars[i] != '}' {
                if chars[i] == '{' {
                    return Err(syntax("nested brace in URIQualifiedName"));
                }
                i += 1;
            }
            if i == chars.len() {
                return Err(syntax("unterminated URIQualifiedName"));
            }
            i += 1;
            while i < chars.len() && crate::qname::xml_ncname_char(chars[i]) {
                i += 1;
            }
            let wildcard = chars.get(i) == Some(&'*');
            if wildcard {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let validation = if wildcard {
                format!("{}name", &text[..text.len() - 1])
            } else {
                text.clone()
            };
            crate::qname::resolve(&validation, &BTreeMap::new(), "XPST0003")?;
            out.push(Token::Name(text));
        } else if crate::qname::xml_ncname_start(c) {
            let start = i;
            i += 1;
            while i < chars.len() && crate::qname::xml_ncname_char(chars[i]) {
                i += 1;
            }
            if chars.get(i) == Some(&':') && !matches!(chars.get(i + 1), Some(':' | '=')) {
                i += 1;
                if chars.get(i) == Some(&'*') {
                    i += 1;
                    out.push(Token::Name(chars[start..i].iter().collect()));
                    continue;
                }
                if !chars
                    .get(i)
                    .is_some_and(|c| crate::qname::xml_ncname_start(*c))
                {
                    // A lone colon (as in `map{$k:1}`) is a separate token.
                    i -= 1;
                    out.push(Token::Name(chars[start..i].iter().collect()));
                    continue;
                }
                i += 1;
                while i < chars.len() && crate::qname::xml_ncname_char(chars[i]) {
                    i += 1;
                }
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else if c == '*' && chars.get(i + 1) == Some(&':') {
            let start = i;
            i += 2;
            if !chars
                .get(i)
                .is_some_and(|c| crate::qname::xml_ncname_start(*c))
            {
                return Err(syntax("expected wildcard local name"));
            }
            i += 1;
            while i < chars.len() && crate::qname::xml_ncname_char(chars[i]) {
                i += 1;
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else {
            let pair: String = chars[i..chars.len().min(i + 2)].iter().collect();
            if matches!(
                pair.as_str(),
                "//" | ".." | "!=" | "<=" | ">=" | "::" | "<<" | ">>" | ":=" | "||" | "=>"
            ) {
                out.push(Token::Symbol(pair));
                i += 2;
            } else if "/.@[](),$+-*=<>|?!{}#:".contains(c) {
                out.push(Token::Symbol(c.to_string()));
                i += 1;
            } else {
                return Err(syntax(format!("unsupported XPath character: {c}")));
            }
        }
    }
    if out.len() > 4096 {
        return Err(syntax("XPath token limit exceeded"));
    }
    out.push(Token::End);
    Ok(out)
}
pub(crate) fn parse(s: &str, namespaces: BTreeMap<String, String>) -> Result<Expr, Diagnostic> {
    parse_inner(s, namespaces, false)
}
pub(crate) fn parse_optional(
    s: &str,
    namespaces: BTreeMap<String, String>,
) -> Result<Expr, Diagnostic> {
    parse_inner(s, namespaces, true)
}
fn parse_inner(
    s: &str,
    namespaces: BTreeMap<String, String>,
    optional: bool,
) -> Result<Expr, Diagnostic> {
    let mut p = Parser {
        tokens: tokenize(s)?,
        index: 0,
        namespaces,
        depth: 0,
    };
    if optional && p.peek() == &Token::End {
        return Ok(Expr::String(String::new()));
    }
    let e = p.expression(0)?;
    if p.peek() != &Token::End {
        return Err(syntax("unexpected token after expression"));
    }
    Ok(e)
}
impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.index]
    }
    fn next(&mut self) -> Token {
        let t = self.tokens[self.index].clone();
        if t != Token::End {
            self.index += 1;
        }
        t
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.peek() == &Token::Symbol(s.into()) {
            self.next();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, s: &str) -> Result<(), Diagnostic> {
        if self.eat(s) {
            Ok(())
        } else {
            Err(syntax(format!("expected {s}")))
        }
    }
    fn qname(&self, text: String) -> Result<Name, Diagnostic> {
        if text.starts_with("Q{") {
            let canonical = crate::qname::resolve(&text, &self.namespaces, "XPST0003")?;
            let (uri, local) = crate::qname::parts(&canonical);
            return Ok(Name {
                uri: uri.map(str::to_owned),
                local: local.into(),
            });
        }
        if let Some((prefix, local)) = text.split_once(':') {
            Ok(Name {
                uri: Some(
                    self.namespaces
                        .get(prefix)
                        .ok_or_else(|| {
                            Diagnostic::new(
                                "XPST0081",
                                format!("unbound prefix: {prefix}"),
                                "compile",
                            )
                        })?
                        .clone(),
                ),
                local: local.into(),
            })
        } else {
            Ok(Name {
                uri: None,
                local: text,
            })
        }
    }
    fn expression(&mut self, minimum: u8) -> Result<Expr, Diagnostic> {
        self.depth += 1;
        if self.depth > 128 {
            return Err(syntax("XPath nesting limit exceeded"));
        }
        let result = self.expression_inner(minimum);
        self.depth -= 1;
        result
    }
    fn expression_inner(&mut self, minimum: u8) -> Result<Expr, Diagnostic> {
        let binding = matches!(self.peek(), Token::Name(n) if ["for", "let", "some", "every"].contains(&n.as_str()))
            && self.tokens.get(self.index + 1) == Some(&Token::Symbol("$".into()));
        let conditional = matches!(self.peek(), Token::Name(n) if n == "if")
            && self.tokens.get(self.index + 1) == Some(&Token::Symbol("(".into()));
        if minimum > 2 && (binding || conditional) {
            return Err(syntax(
                "conditional and binding expressions require parentheses in this operand",
            ));
        }
        let mut left = self.primary()?;
        let mut previous_type = None;
        let mut range_seen = false;
        loop {
            if let Token::Name(op) = self.peek().clone()
                && ["instance", "treat", "cast", "castable"].contains(&op.as_str())
            {
                let precedence = match op.as_str() {
                    "instance" => 11,
                    "treat" => 12,
                    "castable" => 13,
                    _ => 14,
                };
                if minimum > precedence {
                    break;
                }
                if previous_type.is_some_and(|p| precedence >= p) {
                    return Err(syntax("invalid ordering or repetition of type operators"));
                }
                previous_type = Some(precedence);
                self.next();
                let expected = if op == "instance" { "of" } else { "as" };
                if self.next() != Token::Name(expected.into()) {
                    return Err(syntax(format!("expected {expected}")));
                }
                let ty = self.sequence_type(matches!(op.as_str(), "cast" | "castable"))?;
                if matches!(op.as_str(), "cast" | "castable")
                    && (!ty.atomic()
                        || matches!(
                            ty.kind,
                            crate::seqtype::Kind::Atomic | crate::seqtype::Kind::Numeric
                        )
                        || ty.max != 1)
                {
                    return Err(Diagnostic::new(
                        "XPST0080",
                        "cast target must be a supported concrete atomic type",
                        "compile",
                    ));
                }
                if matches!(op.as_str(), "cast" | "castable")
                    && matches!(ty.kind, crate::seqtype::Kind::QName)
                {
                    left = Expr::QNameCast(
                        Box::new(left),
                        self.namespaces.clone(),
                        ty.min == 0,
                        op == "castable",
                    );
                    continue;
                }
                left = Expr::TypeOp(op, Box::new(left), ty);
                continue;
            }
            if self.peek() == &Token::Symbol("=>".into()) {
                if minimum > 14 {
                    break;
                }
                self.next();
                left = self.arrow_call(left)?;
                if matches!(self.peek(), Token::Symbol(s) if ["!", "/", "//"].contains(&s.as_str()))
                {
                    return Err(syntax(
                        "an arrow call cannot be followed by a path or map operator",
                    ));
                }
                continue;
            }
            let (op, prec) = match self.peek() {
                Token::Symbol(s) if s == "," => (s.clone(), 1),
                Token::Name(s) if s == "or" => (s.clone(), 2),
                Token::Name(s) if s == "and" => (s.clone(), 3),
                Token::Name(s) if s == "is" => (s.clone(), 4),
                Token::Name(s) if ["eq", "ne", "lt", "gt", "le", "ge"].contains(&s.as_str()) => {
                    (s.clone(), 4)
                }
                Token::Symbol(s)
                    if ["=", "!=", "<", ">", "<=", ">=", "<<", ">>"].contains(&s.as_str()) =>
                {
                    (s.clone(), 4)
                }
                Token::Symbol(s) if s == "||" => (s.clone(), 5),
                Token::Name(s) if s == "to" => (s.clone(), 6),
                Token::Symbol(s) if s == "!" => (s.clone(), 16),
                Token::Symbol(s) if s == "|" => (s.clone(), 9),
                Token::Name(s) if s == "union" => (s.clone(), 9),
                Token::Name(s) if s == "intersect" || s == "except" => (s.clone(), 10),
                Token::Symbol(s) if s == "+" || s == "-" => (s.clone(), 7),
                Token::Symbol(s) if s == "*" => (s.clone(), 8),
                Token::Name(s) if ["div", "idiv", "mod"].contains(&s.as_str()) => (s.clone(), 8),
                _ => break,
            };
            if prec < minimum {
                break;
            }
            self.next();
            if op == "to" {
                if range_seen {
                    return Err(syntax("range expressions cannot be chained"));
                }
                range_seen = true;
            }
            if op == "!"
                && (matches!(self.peek(), Token::Symbol(s) if s == "+" || s == "-")
                    || matches!(self.peek(), Token::Name(n) if n == "if")
                        && self.tokens.get(self.index + 1) == Some(&Token::Symbol("(".into())))
            {
                return Err(syntax(
                    "simple-map right operand requires a path expression or parentheses",
                ));
            }
            let right = self.expression(prec + 1)?;
            left = if op == "," {
                match left {
                    Expr::Sequence(mut items) => {
                        items.push(right);
                        Expr::Sequence(items)
                    }
                    _ => Expr::Sequence(vec![left, right]),
                }
            } else {
                Expr::Binary(op, Box::new(left), Box::new(right))
            };
        }
        Ok(left)
    }
    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        self.operand(true)
    }
    fn call(&mut self, n: String, mut args: Vec<Expr>) -> Result<Expr, Diagnostic> {
        let name = self.qname(n.clone())?;
        let local = match name.uri.as_deref() {
            None if !n.starts_with("Q{") => name.local,
            Some("http://www.w3.org/2005/xpath-functions") => name.local,
            Some("http://www.w3.org/2001/XMLSchema") => format!("xs:{}", name.local),
            Some("http://www.w3.org/2005/xpath-functions/math") => format!("math:{}", name.local),
            Some("http://www.w3.org/2005/xpath-functions/map") => format!("map:{}", name.local),
            Some("http://www.w3.org/2005/xpath-functions/array") => {
                format!("array:{}", name.local)
            }
            Some(uri) if !uri.is_empty() => {
                return Ok(Expr::Call(format!("Q{{{uri}}}{}", name.local), args));
            }
            _ => {
                return Err(Diagnostic::new(
                    "XPST0017",
                    format!("unknown function namespace: {n}"),
                    "compile",
                ));
            }
        };
        let arity = function_arity(&local).ok_or_else(|| {
            Diagnostic::new(
                "XPST0017",
                format!("unknown or unsupported function: {n}"),
                "compile",
            )
        })?;
        if args.len() < arity.0 || args.len() > arity.1 {
            return Err(Diagnostic::new(
                "XPST0017",
                format!("wrong number of arguments to {n}"),
                "compile",
            ));
        }
        if local == "xs:QName" {
            Ok(Expr::QNameCast(
                Box::new(args.remove(0)),
                self.namespaces.clone(),
                true,
                false,
            ))
        } else {
            if matches!(local.as_str(), "doc" | "document" | "doc-available")
                && let Some(Expr::String(uri)) = args.first()
            {
                DOCUMENT_URIS.with(|uris| uris.borrow_mut().insert(uri.trim().to_owned()));
            }
            if local == "format-number"
                && let Some(Expr::String(format_name)) = args.get_mut(2)
                && format_name.contains(':')
            {
                *format_name =
                    crate::qname::resolve(format_name.trim(), &self.namespaces, "FODF1280")?;
            }
            if local == "key"
                && let Some(Expr::String(key_name)) = args.first_mut()
                && key_name.contains(':')
            {
                *key_name = crate::qname::resolve(key_name.trim(), &self.namespaces, "FOCA0002")?;
            }
            if local == "element-available" {
                args[0] = Expr::StaticName(
                    Box::new(args[0].clone()),
                    self.namespaces.clone(),
                    "XTDE1440",
                );
            }
            if local == "system-property" {
                args[0] = Expr::StaticName(
                    Box::new(args[0].clone()),
                    self.namespaces.clone(),
                    "XTDE1390",
                );
            }
            if local == "type-available" {
                args[0] = Expr::StaticName(
                    Box::new(args[0].clone()),
                    self.namespaces.clone(),
                    "XTDE1428",
                );
            }
            if local == "function-available" {
                args[0] = Expr::StaticName(
                    Box::new(args[0].clone()),
                    self.namespaces.clone(),
                    "XTDE1400",
                );
            }
            let lookup = local == "function-lookup";
            let call = Expr::Call(local, args);
            Ok(if lookup {
                Expr::LookupContext(Box::new(call), self.namespaces.clone())
            } else {
                call
            })
        }
    }
    fn node_qname(&self, text: String, element: bool) -> Result<Name, Diagnostic> {
        if !element {
            return self.qname(text);
        }
        let canonical = crate::qname::resolve_element(&text, &self.namespaces, "XPST0003")?;
        let (uri, local) = crate::qname::parts(&canonical);
        Ok(Name {
            uri: uri.map(str::to_owned),
            local: local.into(),
        })
    }
    fn sequence_type(
        &mut self,
        cast_like: bool,
    ) -> Result<crate::seqtype::SequenceType, Diagnostic> {
        let Token::Name(mut name) = self.next() else {
            return Err(syntax("expected a sequence type"));
        };
        if self.eat("(") {
            name.push('(');
            let mut nesting = 1usize;
            let mut previous_name = false;
            while nesting > 0 {
                let token = self.next();
                let is_name = matches!(token, Token::Name(_));
                if previous_name && is_name {
                    name.push(' ');
                }
                previous_name = is_name;
                match token {
                    Token::Name(s) => name.push_str(&s),
                    Token::Symbol(s) if s == "(" => {
                        nesting += 1;
                        if nesting > 128 {
                            return Err(syntax("sequence type nesting limit exceeded"));
                        }
                        name.push('(');
                    }
                    Token::Symbol(s) if s == ")" => {
                        nesting -= 1;
                        name.push(')');
                    }
                    Token::Symbol(s) if ["*", ",", "?"].contains(&s.as_str()) => name.push_str(&s),
                    _ => return Err(syntax("invalid sequence type")),
                }
            }
        }
        if name.starts_with("function(") && self.peek() == &Token::Name("as".into()) {
            // The declared return type is parsed for syntax but not enforced.
            self.next();
            self.sequence_type(false)?;
        }
        for suffix in if cast_like {
            &["?"][..]
        } else {
            &["?", "*", "+"][..]
        } {
            if self.eat(suffix) {
                name.push_str(suffix);
                break;
            }
        }
        let ty = crate::seqtype::SequenceType::parse(&name, &self.namespaces)?;
        Ok(ty)
    }
    /// `left => f(args)` is `f(left, args)`; the target may also be `$var(...)` or `(expr)(...)`.
    fn arrow_call(&mut self, left: Expr) -> Result<Expr, Diagnostic> {
        let target = match self.peek().clone() {
            Token::Name(n)
                if self.tokens.get(self.index + 1) == Some(&Token::Symbol("(".into())) =>
            {
                self.next();
                Err(n)
            }
            Token::Symbol(s) if s == "$" => {
                self.next();
                let Token::Name(n) = self.next() else {
                    return Err(syntax("expected variable QName"));
                };
                Ok(Expr::Variable(crate::qname::resolve(
                    &n,
                    &self.namespaces,
                    "XPST0003",
                )?))
            }
            Token::Symbol(s) if s == "(" => {
                self.next();
                let inner = self.expression(0)?;
                self.expect(")")?;
                Ok(inner)
            }
            _ => return Err(syntax("invalid arrow function specifier")),
        };
        self.expect("(")?;
        let mut args = vec![left];
        if !self.eat(")") {
            loop {
                args.push(self.expression(2)?);
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        match target {
            Err(name) => self.call(name, args),
            Ok(callee) => Ok(Expr::DynamicCall(Box::new(callee), args)),
        }
    }
    /// Call arguments after `(`; a bare `?` is a placeholder for partial application.
    fn arguments(&mut self) -> Result<Vec<Option<Expr>>, Diagnostic> {
        let mut args = vec![];
        if self.eat(")") {
            return Ok(args);
        }
        loop {
            let placeholder = self.peek() == &Token::Symbol("?".into())
                && matches!(self.tokens.get(self.index + 1), Some(Token::Symbol(s)) if s == "," || s == ")");
            if placeholder {
                self.next();
                args.push(None);
            } else {
                args.push(Some(self.expression(2)?));
            }
            if self.eat(")") {
                break;
            }
            self.expect(",")?;
        }
        Ok(args)
    }
    /// The key specifier after `?`: a name, integer, wildcard or parenthesized expression.
    fn lookup_key(&mut self) -> Result<Option<Box<Expr>>, Diagnostic> {
        Ok(match self.next() {
            Token::Name(n) if !n.contains(':') && !n.starts_with("Q{") => {
                Some(Box::new(Expr::String(n)))
            }
            Token::Number(n @ Number::Integer(_)) => Some(Box::new(Expr::Number(n))),
            Token::Symbol(s) if s == "*" => None,
            Token::Symbol(s) if s == "(" => {
                let inner = self.expression(0)?;
                self.expect(")")?;
                Some(Box::new(inner))
            }
            _ => return Err(syntax("invalid lookup key")),
        })
    }
    fn path_rhs(&mut self, base: Expr) -> Result<Expr, Diagnostic> {
        let binding = matches!(self.peek(), Token::Name(n) if ["for", "let", "some", "every"].contains(&n.as_str()))
            && self.tokens.get(self.index + 1) == Some(&Token::Symbol("$".into()));
        let conditional = matches!(self.peek(), Token::Name(n) if n == "if")
            && self.tokens.get(self.index + 1) == Some(&Token::Symbol("(".into()));
        if binding
            || conditional
            || matches!(self.peek(), Token::Symbol(s) if ["+", "-", "/", "//"].contains(&s.as_str()))
        {
            return Err(syntax("path step requires a postfix or axis expression"));
        }
        let rhs = self.operand(false)?;
        Ok(match rhs {
            Expr::Path(context, steps) if matches!(*context, Expr::Context) => match base {
                Expr::Path(root, mut previous) => {
                    previous.extend(steps);
                    Expr::Path(root, previous)
                }
                other => Expr::Path(Box::new(other), steps),
            },
            other => Expr::PathMap(Box::new(base), Box::new(other)),
        })
    }
    fn descendant_path(base: Expr) -> Expr {
        Expr::Path(
            Box::new(base),
            vec![Step {
                axis: Axis::DescendantOrSelf,
                test: Test::Node,
                predicates: vec![],
            }],
        )
    }
    fn operand(&mut self, paths: bool) -> Result<Expr, Diagnostic> {
        let mut callable = false;
        let mut e = if self.eat("-") {
            Expr::Unary(false, Box::new(self.expression(15)?))
        } else if self.eat("+") {
            Expr::Unary(true, Box::new(self.expression(15)?))
        } else if self.eat("(") {
            callable = true;
            if self.eat(")") {
                Expr::Sequence(vec![])
            } else {
                let e = self.expression(0)?;
                self.expect(")")?;
                e
            }
        } else if self.eat("$") {
            callable = true;
            match self.next() {
                Token::Name(n) => {
                    Expr::Variable(crate::qname::resolve(&n, &self.namespaces, "XPST0003")?)
                }
                _ => return Err(syntax("expected variable QName")),
            }
        } else if self.eat(".") {
            Expr::Context
        } else if self.eat("/") {
            let type_operator = matches!(self.peek(),Token::Name(n) if ["instance","treat","cast","castable"].contains(&n.as_str()))
                && matches!(self.tokens.get(self.index+1),Some(Token::Name(n)) if n == "of" || n == "as");
            let postfix = matches!(self.peek(), Token::String(_) | Token::Number(_))
                || matches!(self.peek(), Token::Symbol(s) if s == "$" || s == "(");
            if (self.step_start() || postfix) && !type_operator {
                self.path_rhs(Expr::Root)?
            } else {
                Expr::Root
            }
        } else if self.eat("//") {
            self.path_rhs(Self::descendant_path(Expr::Root))?
        } else {
            match self.peek().clone() {
                Token::String(s) => {
                    self.next();
                    Expr::String(s)
                }
                Token::Number(n) => {
                    self.next();
                    Expr::Number(n)
                }
                Token::Name(n)
                    if (["for", "let", "some", "every"].contains(&n.as_str()))
                        && self.tokens.get(self.index + 1) == Some(&Token::Symbol("$".into())) =>
                {
                    self.next();
                    self.binding_expression(&n)?
                }
                Token::Name(n)
                    if n == "map"
                        && self.tokens.get(self.index + 1) == Some(&Token::Symbol("{".into())) =>
                {
                    self.next();
                    self.next();
                    let mut entries = vec![];
                    if !self.eat("}") {
                        loop {
                            let key = self.expression(2)?;
                            self.expect(":")?;
                            let value = self.expression(2)?;
                            entries.push((key, value));
                            if self.eat("}") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    Expr::MapConstructor(entries)
                }
                Token::Name(n)
                    if n == "array"
                        && self.tokens.get(self.index + 1) == Some(&Token::Symbol("{".into())) =>
                {
                    self.next();
                    self.next();
                    if self.eat("}") {
                        Expr::ArrayCurly(Box::new(Expr::Sequence(vec![])))
                    } else {
                        let inner = self.expression(0)?;
                        self.expect("}")?;
                        Expr::ArrayCurly(Box::new(inner))
                    }
                }
                Token::Symbol(s) if s == "[" => {
                    self.next();
                    let mut members = vec![];
                    if !self.eat("]") {
                        loop {
                            members.push(self.expression(2)?);
                            if self.eat("]") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    Expr::ArraySquare(members)
                }
                Token::Symbol(s) if s == "?" => {
                    self.next();
                    let key = self.lookup_key()?;
                    Expr::Lookup(Box::new(Expr::Context), key)
                }
                Token::Name(n)
                    if n == "function"
                        && self.tokens.get(self.index + 1) == Some(&Token::Symbol("(".into())) =>
                {
                    self.next();
                    self.expect("(")?;
                    let mut params = vec![];
                    if !self.eat(")") {
                        loop {
                            self.expect("$")?;
                            let Token::Name(name) = self.next() else {
                                return Err(syntax("expected parameter name"));
                            };
                            let name = crate::qname::resolve(&name, &self.namespaces, "XPST0003")?;
                            if params.iter().any(|(p, _)| *p == name) {
                                return Err(Diagnostic::new(
                                    "XQST0039",
                                    "duplicate inline function parameter",
                                    "compile",
                                ));
                            }
                            let ty = if self.peek() == &Token::Name("as".into()) {
                                self.next();
                                Some(self.sequence_type(false)?)
                            } else {
                                None
                            };
                            params.push((name, ty));
                            if self.eat(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    let ret = if self.peek() == &Token::Name("as".into()) {
                        self.next();
                        Some(self.sequence_type(false)?)
                    } else {
                        None
                    };
                    self.expect("{")?;
                    let body = if self.eat("}") {
                        Expr::Sequence(vec![])
                    } else {
                        let body = self.expression(0)?;
                        self.expect("}")?;
                        body
                    };
                    Expr::InlineFunction(std::sync::Arc::new(InlineFunction { params, ret, body }))
                }
                Token::Name(n)
                    if self.tokens.get(self.index + 1) == Some(&Token::Symbol("#".into()))
                        && matches!(self.tokens.get(self.index + 2), Some(Token::Number(_))) =>
                {
                    self.next();
                    self.next();
                    let Token::Number(arity) = self.next() else {
                        unreachable!()
                    };
                    let arity = arity
                        .double()
                        .to_usize()
                        .filter(|a| *a as f64 == arity.double())
                        .ok_or_else(|| syntax("invalid function arity"))?;
                    match unwrap_lookup_call(self.call(n.clone(), vec![Expr::Context; arity])?) {
                        Expr::Call(name, _) => {
                            let namespaces = matches!(
                                name.as_str(),
                                "element-available"
                                    | "system-property"
                                    | "type-available"
                                    | "function-available"
                                    | "function-lookup"
                            )
                            .then(|| self.namespaces.clone());
                            Expr::FunctionRef(name, arity, namespaces)
                        }
                        _ => return Err(syntax("unsupported function reference")),
                    }
                }
                Token::Name(n)
                    if n == "if"
                        && self.tokens.get(self.index + 1) == Some(&Token::Symbol("(".into())) =>
                {
                    self.next();
                    self.expect("(")?;
                    let condition = self.expression(0)?;
                    self.expect(")")?;
                    if self.next() != Token::Name("then".into()) {
                        return Err(syntax("expected then"));
                    }
                    let yes = self.expression(2)?;
                    if self.next() != Token::Name("else".into()) {
                        return Err(syntax("expected else"));
                    }
                    let no = self.expression(2)?;
                    Expr::If(Box::new(condition), Box::new(yes), Box::new(no))
                }
                Token::Name(n)
                    if self.tokens.get(self.index + 1) == Some(&Token::Symbol("(".into()))
                        && ![
                            "node",
                            "text",
                            "comment",
                            "processing-instruction",
                            "document-node",
                            "element",
                            "attribute",
                        ]
                        .contains(&n.as_str()) =>
                {
                    self.next();
                    self.expect("(")?;
                    let arguments = self.arguments()?;
                    if arguments.iter().any(Option::is_none) {
                        let arity = arguments.len();
                        let Expr::Call(name, _) =
                            unwrap_lookup_call(self.call(n, vec![Expr::Context; arity])?)
                        else {
                            return Err(syntax("unsupported partial function application"));
                        };
                        {
                            let namespaces = matches!(
                                name.as_str(),
                                "element-available"
                                    | "system-property"
                                    | "type-available"
                                    | "function-available"
                                    | "function-lookup"
                            )
                            .then(|| self.namespaces.clone());
                            Expr::Partial(
                                Box::new(Expr::FunctionRef(name, arity, namespaces)),
                                arguments,
                            )
                        }
                    } else {
                        self.call(n, arguments.into_iter().flatten().collect())?
                    }
                }
                _ if self.step_start() => Expr::Path(Box::new(Expr::Context), vec![self.step()?]),
                _ => return Err(syntax("expected XPath expression")),
            }
        };
        if matches!(
            e,
            Expr::InlineFunction(_)
                | Expr::FunctionRef(..)
                | Expr::Call(..)
                | Expr::LookupContext(..)
                | Expr::MapConstructor(_)
                | Expr::ArraySquare(_)
                | Expr::ArrayCurly(_)
                | Expr::Lookup(..)
                | Expr::Partial(..)
        ) {
            callable = true;
        }
        let mut path_steps = 0;
        loop {
            if path_steps > 128 {
                return Err(syntax("XPath path nesting limit exceeded"));
            }
            if callable
                && matches!(
                    e,
                    Expr::Variable(_)
                        | Expr::Sequence(_)
                        | Expr::InlineFunction(_)
                        | Expr::FunctionRef(..)
                        | Expr::DynamicCall(..)
                        | Expr::Filter(..)
                        | Expr::Call(..)
                        | Expr::LookupContext(..)
                        | Expr::If(..)
                        | Expr::Path(..)
                        | Expr::Binary(..)
                        | Expr::MapConstructor(_)
                        | Expr::ArraySquare(_)
                        | Expr::ArrayCurly(_)
                        | Expr::Lookup(..)
                        | Expr::Partial(..)
                )
                && self.peek() == &Token::Symbol("(".into())
            {
                self.next();
                let arguments = self.arguments()?;
                e = if arguments.iter().any(Option::is_none) {
                    Expr::Partial(Box::new(e), arguments)
                } else {
                    Expr::DynamicCall(Box::new(e), arguments.into_iter().flatten().collect())
                };
            } else if self.eat("?") {
                let key = self.lookup_key()?;
                e = Expr::Lookup(Box::new(e), key);
            } else if self.eat("[") {
                let p = self.expression(0)?;
                self.expect("]")?;
                e = match e {
                    Expr::Filter(base, mut ps) => {
                        ps.push(p);
                        Expr::Filter(base, ps)
                    }
                    _ => Expr::Filter(Box::new(e), vec![p]),
                };
            } else if paths && self.eat("/") {
                e = self.path_rhs(e)?;
                path_steps += 1;
            } else if paths && self.eat("//") {
                e = self.path_rhs(Self::descendant_path(e))?;
                path_steps += 1;
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn binding_expression(&mut self, keyword: &str) -> Result<Expr, Diagnostic> {
        let mut bindings = vec![];
        loop {
            if bindings.len() >= 128 {
                return Err(syntax("XPath binding nesting limit exceeded"));
            }
            self.expect("$")?;
            let Token::Name(name) = self.next() else {
                return Err(syntax("expected binding variable name"));
            };
            let name = crate::qname::resolve(&name, &self.namespaces, "XPST0003")?;
            if keyword != "let" {
                if self.next() != Token::Name("in".into()) {
                    return Err(syntax("expected in"));
                }
            } else {
                self.expect(":=")?;
            }
            let extra = bindings.len();
            self.depth += extra;
            let selected = self.expression(2);
            self.depth -= extra;
            let select = selected?;
            bindings.push((name, select));
            if !self.eat(",") {
                break;
            }
        }
        let terminal = if matches!(keyword, "some" | "every") {
            "satisfies"
        } else {
            "return"
        };
        if self.next() != Token::Name(terminal.into()) {
            return Err(syntax(format!("expected {terminal}")));
        }
        let extra = bindings.len().saturating_sub(1);
        self.depth += extra;
        let returned = self.expression(2);
        self.depth -= extra;
        let mut body = returned?;
        for (name, select) in bindings.into_iter().rev() {
            body = match keyword {
                "for" => Expr::For(name, Box::new(select), Box::new(body)),
                "let" => Expr::Let(name, Box::new(select), Box::new(body)),
                _ => Expr::Quantified(keyword == "every", name, Box::new(select), Box::new(body)),
            };
        }
        Ok(body)
    }
    fn step_start(&self) -> bool {
        matches!(self.peek(), Token::Name(_))
            || matches!(self.peek(), Token::Symbol(s) if [".", "..", "@", "*"].contains(&s.as_str()))
    }
    fn step(&mut self) -> Result<Step, Diagnostic> {
        let (axis, test) = if self.eat(".") {
            (Axis::SelfNode, Test::Node)
        } else if self.eat("..") {
            (Axis::Parent, Test::Node)
        } else {
            let mut axis = if self.eat("@") {
                Axis::Attribute
            } else {
                Axis::Child
            };
            if let Token::Name(n) = self.peek().clone()
                && self.tokens.get(self.index + 1) == Some(&Token::Symbol("::".into()))
            {
                self.next();
                self.next();
                axis = match n.as_str() {
                    "child" => Axis::Child,
                    "attribute" => Axis::Attribute,
                    "self" => Axis::SelfNode,
                    "parent" => Axis::Parent,
                    "descendant" => Axis::Descendant,
                    "descendant-or-self" => Axis::DescendantOrSelf,
                    "ancestor" => Axis::Ancestor,
                    "ancestor-or-self" => Axis::AncestorOrSelf,
                    "following-sibling" => Axis::FollowingSibling,
                    "preceding-sibling" => Axis::PrecedingSibling,
                    "following" => Axis::Following,
                    "preceding" => Axis::Preceding,
                    _ => return Err(syntax(format!("unsupported axis: {n}"))),
                };
            }
            let test = if self.eat("*") {
                Test::Any
            } else {
                match self.next() {
                    Token::Name(n) => {
                        if self.eat("(") {
                            let test = match n.as_str() {
                                "node" => Test::Node,
                                "text" => Test::Text,
                                "comment" => Test::Comment,
                                "document-node" => Test::Document,
                                "processing-instruction" => {
                                    let target = match self.peek().clone() {
                                        Token::String(s) => {
                                            self.next();
                                            Some(s)
                                        }
                                        Token::Name(s) if !s.contains(':') => {
                                            self.next();
                                            Some(s)
                                        }
                                        _ => None,
                                    };
                                    Test::Pi(target)
                                }
                                "element" | "attribute" => {
                                    let n_is_element = n == "element";
                                    let name = match self.peek().clone() {
                                        Token::Name(n) => {
                                            self.next();
                                            Some(self.node_qname(n, n_is_element)?)
                                        }
                                        _ => {
                                            self.eat("*");
                                            None
                                        }
                                    };
                                    if n == "element" {
                                        Test::Element(name)
                                    } else {
                                        Test::AttributeKind(name)
                                    }
                                }
                                _ => return Err(syntax("unsupported node test")),
                            };
                            self.expect(")")?;
                            test
                        } else if n.starts_with("Q{") && n.ends_with("}*") {
                            Test::Namespace(crate::qname::normalize_uri(&n[2..n.len() - 2]))
                        } else if let Some(prefix) = n.strip_suffix(":*") {
                            Test::Namespace(self.namespaces.get(prefix).cloned().ok_or_else(
                                || {
                                    Diagnostic::new(
                                        "XPST0081",
                                        "unbound wildcard prefix",
                                        "compile",
                                    )
                                },
                            )?)
                        } else if let Some(local) = n.strip_prefix("*:") {
                            Test::Local(local.into())
                        } else {
                            Test::Name(self.node_qname(n, !matches!(axis, Axis::Attribute))?)
                        }
                    }
                    _ => return Err(syntax("expected node test")),
                }
            };
            (axis, test)
        };
        let mut predicates = vec![];
        while self.eat("[") {
            predicates.push(self.expression(0)?);
            self.expect("]")?;
        }
        Ok(Step {
            axis,
            test,
            predicates,
        })
    }
}
/// Splits names such as `year-from-date` into the accepted type and component.
fn temporal_accessor(name: &str) -> Option<(crate::temporal::TemporalType, &str)> {
    let (component, kind) = name.split_once("-from-")?;
    let kind = crate::temporal::TemporalType::parse(kind)?;
    let valid = if kind.is_duration() {
        matches!(
            component,
            "years" | "months" | "days" | "hours" | "minutes" | "seconds"
        )
    } else {
        match component {
            "year" | "month" | "day" => kind != crate::temporal::TemporalType::Time,
            "hours" | "minutes" | "seconds" => kind != crate::temporal::TemporalType::Date,
            "timezone" => true,
            _ => false,
        }
    };
    valid.then_some((kind, component))
}
thread_local! {
    static XPATH_ONLY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
pub(crate) fn xpath_only_profile() -> bool {
    XPATH_ONLY.with(std::cell::Cell::get)
}
pub(crate) fn with_xpath_profile<T>(operation: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            XPATH_ONLY.with(|profile| profile.set(self.0));
        }
    }
    let _restore = Restore(XPATH_ONLY.with(|profile| profile.replace(true)));
    operation()
}
fn xslt_function(name: &str) -> bool {
    matches!(
        name,
        "current"
            | "current-group"
            | "current-grouping-key"
            | "key"
            | "document"
            | "system-property"
            | "element-available"
            | "function-available"
            | "type-available"
            | "regex-group"
            | "unparsed-entity-uri"
            | "unparsed-entity-public-id"
    )
}
fn function_arity(name: &str) -> Option<(usize, usize)> {
    if xpath_only_profile() && xslt_function(name) {
        return None;
    }
    Some(match name {
        "xs:string" | "xs:untypedAtomic" | "xs:integer" | "xs:decimal" | "xs:double"
        | "xs:float" | "xs:boolean" | "xs:QName" | "xs:anyURI" | "xs:hexBinary"
        | "xs:base64Binary" => (1, 1),
        "true" | "false" | "position" | "last" | "default-collation" | "current-dateTime"
        | "current-date" | "current-time" | "implicit-timezone" => (0, 0),
        name if temporal_accessor(name).is_some() => (1, 1),
        "dateTime" => (2, 2),
        "adjust-dateTime-to-timezone" | "adjust-date-to-timezone" | "adjust-time-to-timezone" => {
            (1, 2)
        }
        name if name
            .strip_prefix("xs:")
            .and_then(crate::temporal::TemporalType::parse)
            .is_some() =>
        {
            (1, 1)
        }
        "trace" => (1, 2),
        "error" => (0, 3),
        "QName" => (2, 2),
        "namespace-uri-from-QName" | "prefix-from-QName" | "local-name-from-QName" => (1, 1),
        "namespace-uri-for-prefix" | "resolve-QName" => (2, 2),
        "lang" => (1, 2),
        "in-scope-prefixes" | "innermost" | "outermost" | "unordered" => (1, 1),
        "string" | "number" | "normalize-space" | "string-length" | "name" | "local-name"
        | "namespace-uri" | "node-name" | "has-children" | "generate-id" | "root" | "data"
        | "nilled" => (0, 1),
        "count" | "boolean" | "not" | "exists" | "empty" | "reverse" | "head" | "tail" => (1, 1),
        "sum" | "min" | "max" | "distinct-values" => (1, 2),
        "avg" | "exactly-one" | "zero-or-one" | "one-or-more" | "upper-case" | "lower-case" => {
            (1, 1)
        }
        "substring" | "compare" => (2, 3),
        "codepoint-equal" => (2, 2),
        "normalize-unicode" => (1, 2),
        "abs" | "floor" | "ceiling" => (1, 1),
        "round" | "round-half-to-even" => (1, 2),
        "codepoints-to-string"
        | "string-to-codepoints"
        | "encode-for-uri"
        | "iri-to-uri"
        | "escape-html-uri" => (1, 1),
        "translate" => (3, 3),
        "remove" => (2, 2),
        "insert-before" => (3, 3),
        "subsequence" | "deep-equal" | "index-of" => (2, 3),
        "contains" | "starts-with" | "ends-with" | "substring-before" | "substring-after" => (2, 3),
        "string-join" => (1, 2),
        "contains-token" => (2, 3),
        "resolve-uri" => (1, 2),
        "sort" => (1, 3),
        "for-each" | "filter" => (2, 2),
        "fold-left" | "fold-right" | "for-each-pair" => (3, 3),
        "format-number" => (2, 3),
        "parse-json" => (1, 2),
        "format-integer" => (2, 3),
        "id" | "element-with-id" => (1, 2),
        "doc" | "doc-available" => (1, 1),
        "document" => (1, 2),
        "system-property" | "element-available" | "type-available" => (1, 1),
        "function-available" => (1, 2),
        "default-language" | "static-base-uri" | "available-environment-variables" => (0, 0),
        "environment-variable" => (1, 1),
        "base-uri" | "document-uri" => (0, 1),
        "collation-key" => (1, 2),
        "unparsed-text" | "unparsed-text-available" => (1, 2),
        "unparsed-text-lines" => (1, 2),
        "json-doc" => (1, 2),
        "format-dateTime" | "format-date" | "format-time" => (2, 5),
        "math:pi" => (0, 0),
        "math:exp" | "math:exp10" | "math:log" | "math:log10" | "math:sqrt" | "math:sin"
        | "math:cos" | "math:tan" | "math:asin" | "math:acos" | "math:atan" => (1, 1),
        "math:pow" | "math:atan2" => (2, 2),
        "function-arity" | "function-name" => (1, 1),
        "function-lookup" | "apply" => (2, 2),
        name if crate::maps::arity(name).is_some() => crate::maps::arity(name).unwrap(),
        "key" => (2, 3),
        "current" | "current-group" | "current-grouping-key" => (0, 0),
        "regex-group" => (1, 1),
        "path" => (0, 1),
        "matches" => (2, 3),
        "replace" => (3, 4),
        "tokenize" => (1, 3),
        "concat" => (2, usize::MAX),
        name if name
            .strip_prefix("xs:")
            .and_then(crate::integer_type::IntegerType::parse)
            .is_some() =>
        {
            (1, 1)
        }
        name if name
            .strip_prefix("xs:")
            .and_then(crate::string_type::StringType::parse)
            .is_some() =>
        {
            (1, 1)
        }
        _ => return None,
    })
}
pub(crate) fn test_node(test: &Test, n: XNode<'_>, axis: Axis) -> bool {
    let principal = if matches!(axis, Axis::Attribute) {
        n.is_attribute()
    } else {
        n.is_element()
    };
    match test {
        Test::Node => true,
        Test::Any => principal,
        Test::Name(name) => principal && n.name() == (name.uri.as_deref(), name.local.as_str()),
        Test::Text => n.is_text(),
        Test::Comment => n.is_comment(),
        Test::Pi(target) => {
            n.is_pi()
                && n.raw
                    .pi()
                    .is_some_and(|pi| target.as_ref().is_none_or(|s| s == pi.target))
        }
        Test::Document => n.is_document(),
        Test::Element(name) => {
            n.is_element()
                && name
                    .as_ref()
                    .is_none_or(|name| n.name() == (name.uri.as_deref(), name.local.as_str()))
        }
        Test::AttributeKind(name) => {
            n.is_attribute()
                && name
                    .as_ref()
                    .is_none_or(|name| n.name() == (name.uri.as_deref(), name.local.as_str()))
        }
        Test::Namespace(uri) => principal && n.name().0.unwrap_or_default() == uri,
        Test::Local(local) => principal && n.name().1 == local,
    }
}
fn ordered<'a>(mut nodes: Vec<XNode<'a>>) -> Value<'a> {
    nodes.sort_by_key(|n| n.order());
    nodes.dedup();
    nodes.into_iter().map(Item::Node).collect()
}
pub(crate) fn filter<'a>(
    mut value: Value<'a>,
    predicates: &[Expr],
    vars: &Variables<'a>,
    budget: &mut Budget,
    depth: usize,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    for predicate in predicates {
        let size = value.len();
        let mut selected = vec![];
        for (i, item) in value.into_iter().enumerate() {
            budget.tick()?;
            let focus = Focus {
                item: item.clone(),
                position: i + 1,
                size,
            };
            let v = eval(predicate, &focus, vars, budget, depth + 1, backwards)?;
            let keep = match v.as_slice() {
                [Item::Number(n)] => {
                    n.compare(&Number::from(i + 1)) == Some(std::cmp::Ordering::Equal)
                }
                _ => boolean(&v)?,
            };
            if keep {
                selected.push(item);
            }
        }
        value = selected;
    }
    Ok(value)
}
pub(crate) fn eval<'a>(
    e: &Expr,
    focus: &Focus<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    depth: usize,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    budget.tick()?;
    if depth > budget.max_depth.min(128) {
        return Err(err("XPDY0130", "XPath depth limit exceeded"));
    }
    let current_scope;
    let vars = if depth == 0 {
        current_scope = {
            let mut scope = vars.clone();
            scope.insert(CURRENT_ITEM.into(), vec![focus.item.clone()]);
            scope
        };
        &current_scope
    } else {
        vars
    };
    let result = match e {
        Expr::String(s) => vec![Item::String(s.clone())],
        Expr::Number(n) => vec![Item::Number(n.clone())],
        Expr::Variable(n) => vars
            .get(n)
            .cloned()
            .ok_or_else(|| err("XPST0008", format!("unknown variable: {n}")))?,
        Expr::For(name, select, body) => {
            let selected = eval(select, focus, vars, budget, depth + 1, backwards)?;
            let mut result = vec![];
            for item in selected {
                budget.tick()?;
                let mut scope = vars.clone();
                scope.insert(name.clone(), vec![item]);
                for item in eval(body, focus, &scope, budget, depth + 1, backwards)? {
                    budget.tick()?;
                    result.push(item);
                }
            }
            result
        }
        Expr::Quantified(every, name, select, body) => {
            let selected = eval(select, focus, vars, budget, depth + 1, backwards)?;
            let mut result = *every;
            for item in selected {
                budget.tick()?;
                let mut scope = vars.clone();
                scope.insert(name.clone(), vec![item]);
                let value = eval(body, focus, &scope, budget, depth + 1, backwards)?;
                let matches = boolean(&value)?;
                if matches != *every {
                    result = matches;
                    break;
                }
            }
            vec![Item::Boolean(result)]
        }
        Expr::Let(name, select, body) => {
            let selected = eval(select, focus, vars, budget, depth + 1, backwards)?;
            let mut scope = vars.clone();
            scope.insert(name.clone(), selected);
            eval(body, focus, &scope, budget, depth + 1, backwards)?
        }
        Expr::Context => {
            if matches!(focus.item, Item::Absent) {
                return Err(err("XPDY0002", "context item is absent"));
            } else {
                vec![focus.item.clone()]
            }
        }
        Expr::Root => match focus.item {
            Item::Node(n) => {
                let root = n.root();
                if !root.is_document() {
                    return Err(err(
                        "XPDY0050",
                        "absolute paths require a document-rooted tree",
                    ));
                }
                vec![Item::Node(root)]
            }
            Item::Absent => return Err(err("XPDY0002", "context item is absent")),
            _ => return Err(err("XPTY0020", "root requires a node context")),
        },
        Expr::LookupContext(value, namespaces) => attach_lookup_context(
            eval(value, focus, vars, budget, depth + 1, backwards)?,
            namespaces,
        ),
        Expr::StaticName(value, namespaces, code) => {
            let value = eval(value, focus, vars, budget, depth + 1, backwards)?;
            static_name(&value, namespaces, backwards, *code != "XTDE1390", code)?
        }
        Expr::QNameCast(value, namespaces, optional, castable) => {
            let value = eval(value, focus, vars, budget, depth + 1, backwards)?;
            let cast = || -> Result<Value<'a>, Diagnostic> {
                match value.as_slice() {
                    [] if *optional => Ok(vec![]),
                    [item] => match atomize(item) {
                        Item::QName(name) => Ok(vec![Item::QName(name)]),
                        Item::String(s) | Item::Untyped(s) | Item::DerivedString(_, s) => {
                            for _ in s.bytes() {
                                budget.tick()?;
                            }
                            Ok(vec![Item::QName(crate::qname::AtomicQName::cast(
                                &s, namespaces,
                            )?)])
                        }
                        _ => Err(err(
                            "XPTY0004",
                            "QName casting requires a string, untyped atomic value or QName",
                        )),
                    },
                    _ => Err(err(
                        "XPTY0004",
                        "QName cast requires the declared singleton cardinality",
                    )),
                }
            };
            let mut cast = cast;
            match cast() {
                Ok(_) if *castable => vec![Item::Boolean(true)],
                Err(error)
                    if *castable
                        && ["XPTY0004", "FORG0001", "FONS0004"].contains(&error.code.as_str()) =>
                {
                    vec![Item::Boolean(false)]
                }
                other => other?,
            }
        }
        Expr::TypeOp(op, e, ty) => {
            let value = eval(e, focus, vars, budget, depth + 1, backwards)?;
            match op.as_str() {
                "variable" => ty.convert(value, "XTTE0570")?,
                "instance" => vec![Item::Boolean(ty.matches(&value))],
                "treat" => {
                    if !ty.matches(&value) {
                        return Err(err(
                            "XPDY0050",
                            "treat-as value does not match the required type",
                        ));
                    }
                    value
                }
                _ => {
                    let mut cast = || -> Result<Value<'a>, Diagnostic> {
                        if value.len() < ty.min || value.len() > ty.max {
                            return Err(err(
                                "XPTY0004",
                                "cast requires the declared singleton cardinality",
                            ));
                        }
                        let kind = match ty.kind {
                            crate::seqtype::Kind::String => "string",
                            crate::seqtype::Kind::DerivedString(kind) => kind.name(),
                            crate::seqtype::Kind::Untyped => "untypedAtomic",
                            crate::seqtype::Kind::AnyUri => "anyURI",
                            crate::seqtype::Kind::Boolean => "boolean",
                            crate::seqtype::Kind::Binary(kind) => kind.name(),
                            crate::seqtype::Kind::Temporal(kind) => kind.name(),
                            crate::seqtype::Kind::Integer => "integer",
                            crate::seqtype::Kind::DerivedInteger(kind) => kind.name(),
                            crate::seqtype::Kind::Decimal => "decimal",
                            crate::seqtype::Kind::Float => "float",
                            crate::seqtype::Kind::Double => "double",
                            _ => unreachable!(),
                        };
                        function(
                            &format!("xs:{kind}"),
                            std::slice::from_ref(&value),
                            focus,
                            budget,
                            backwards,
                        )
                    };
                    let result = cast();
                    if op == "castable" {
                        match result {
                            Ok(_) => vec![Item::Boolean(true)],
                            Err(e)
                                if ["XPTY0004", "FORG0001", "FOCA0002"]
                                    .contains(&e.code.as_str()) =>
                            {
                                vec![Item::Boolean(false)]
                            }
                            Err(e) => return Err(e),
                        }
                    } else {
                        result?
                    }
                }
            }
        }
        Expr::If(condition, yes, no) => {
            let condition = eval(condition, focus, vars, budget, depth + 1, backwards)?;
            eval(
                if boolean(&condition)? { yes } else { no },
                focus,
                vars,
                budget,
                depth + 1,
                backwards,
            )?
        }
        Expr::Sequence(es) => {
            let mut value = vec![];
            for e in es {
                value.extend(eval(e, focus, vars, budget, depth + 1, backwards)?);
            }
            value
        }
        Expr::Filter(base, predicates) => filter(
            eval(base, focus, vars, budget, depth + 1, backwards)?,
            predicates,
            vars,
            budget,
            depth,
            backwards,
        )?,
        Expr::PathMap(base, step) => {
            let input = eval(base, focus, vars, budget, depth + 1, backwards)?;
            let size = input.len();
            let mut result = vec![];
            let mut has_nodes = false;
            let mut has_atomics = false;
            for (index, item) in input.into_iter().enumerate() {
                budget.tick()?;
                if !matches!(item, Item::Node(_)) {
                    return Err(err("XPTY0019", "path operand must contain nodes"));
                }
                let focus = Focus {
                    item,
                    position: index + 1,
                    size,
                };
                for item in eval(step, &focus, vars, budget, depth + 1, backwards)? {
                    budget.tick()?;
                    has_nodes |= matches!(item, Item::Node(_));
                    has_atomics |= !matches!(item, Item::Node(_));
                    if has_nodes && has_atomics {
                        return Err(err("XPTY0018", "path result mixes nodes and atomic values"));
                    }
                    result.push(item);
                }
            }
            if has_nodes {
                ordered(
                    result
                        .into_iter()
                        .map(|item| {
                            let Item::Node(node) = item else {
                                unreachable!()
                            };
                            node
                        })
                        .collect(),
                )
            } else {
                result
            }
        }
        Expr::Path(base, steps) => {
            let mut value = eval(base, focus, vars, budget, depth + 1, backwards)?;
            for step in steps {
                let mut nodes = vec![];
                for item in value {
                    budget.tick()?;
                    let Item::Node(n) = item else {
                        return Err(err("XPTY0019", "path operand must contain nodes"));
                    };
                    let candidates: Vec<_> = match step.axis {
                        Axis::SelfNode => vec![n],
                        Axis::Parent => n.parent().into_iter().collect(),
                        Axis::Child => n.children().collect(),
                        Axis::Attribute => n.attributes(),
                        Axis::Ancestor | Axis::AncestorOrSelf => {
                            let mut out = vec![];
                            let mut current = if matches!(step.axis, Axis::AncestorOrSelf) {
                                Some(n)
                            } else {
                                n.parent()
                            };
                            while let Some(node) = current {
                                budget.tick()?;
                                out.push(node);
                                current = node.parent();
                            }
                            out
                        }
                        Axis::FollowingSibling | Axis::PrecedingSibling => {
                            if n.is_attribute() || n.parent().is_none() {
                                vec![]
                            } else if matches!(step.axis, Axis::FollowingSibling) {
                                n.raw
                                    .next_siblings()
                                    .skip(1)
                                    .map(|raw| n.relative(raw))
                                    .collect()
                            } else {
                                n.raw
                                    .prev_siblings()
                                    .skip(1)
                                    .map(|raw| n.relative(raw))
                                    .collect()
                            }
                        }
                        Axis::Following | Axis::Preceding => {
                            let mut out = vec![];
                            for candidate in n.root().descendants() {
                                budget.tick()?;
                                let keep = if matches!(step.axis, Axis::Following) {
                                    candidate.raw.id().get() > n.raw.id().get()
                                        && (n.is_attribute()
                                            || !candidate.raw.ancestors().any(|a| a == n.raw))
                                } else {
                                    candidate.raw.id().get() < n.raw.id().get()
                                        && !n.raw.ancestors().any(|a| a == candidate.raw)
                                };
                                if keep {
                                    out.push(candidate);
                                }
                            }
                            if matches!(step.axis, Axis::Preceding) {
                                out.reverse();
                            }
                            out
                        }
                        Axis::Descendant | Axis::DescendantOrSelf => n
                            .descendants()
                            .into_iter()
                            .skip(usize::from(matches!(step.axis, Axis::Descendant)))
                            .collect(),
                    };
                    let mut selected = vec![];
                    for n in candidates {
                        budget.tick()?;
                        if test_node(&step.test, n, step.axis) {
                            selected.push(Item::Node(n));
                        }
                    }
                    let selected =
                        filter(selected, &step.predicates, vars, budget, depth, backwards)?;
                    nodes.extend(
                        selected
                            .into_iter()
                            .filter_map(|i| if let Item::Node(n) = i { Some(n) } else { None }),
                    );
                }
                value = ordered(nodes);
            }
            value
        }
        Expr::Unary(plus, e) => {
            let value = eval(e, focus, vars, budget, depth + 1, backwards)?;
            if value.is_empty() && !backwards {
                return Ok(vec![]);
            }
            let n = numeric(&value, backwards)?;
            vec![Item::Number(if *plus { n } else { n.negate() })]
        }
        Expr::Binary(op, a, b) => {
            let a = eval(a, focus, vars, budget, depth + 1, backwards)?;
            if op == "!" || op == "instruction-map" {
                let size = a.len();
                let mut result = vec![];
                for (index, item) in a.into_iter().enumerate() {
                    budget.tick()?;
                    let scope;
                    let vars = if op == "instruction-map" {
                        scope = {
                            let mut scope = vars.clone();
                            scope.insert(CURRENT_ITEM.into(), vec![item.clone()]);
                            scope
                        };
                        &scope
                    } else {
                        vars
                    };
                    let mapped = eval(
                        b,
                        &Focus {
                            item,
                            position: index + 1,
                            size,
                        },
                        vars,
                        budget,
                        depth + 1,
                        backwards,
                    )?;
                    for item in mapped {
                        budget.tick()?;
                        result.push(item);
                    }
                }
                return Ok(result);
            }
            if op == "and" && !boolean(&a)? {
                return Ok(vec![Item::Boolean(false)]);
            }
            if op == "or" && boolean(&a)? {
                return Ok(vec![Item::Boolean(true)]);
            }
            let b = eval(b, focus, vars, budget, depth + 1, backwards)?;
            let (a, b) = if matches!(
                op.as_str(),
                "=" | "!="
                    | "<"
                    | ">"
                    | "<="
                    | ">="
                    | "eq"
                    | "ne"
                    | "lt"
                    | "le"
                    | "gt"
                    | "ge"
                    | "+"
                    | "-"
                    | "*"
                    | "div"
                    | "idiv"
                    | "mod"
            ) {
                (flatten_arrays(a)?, flatten_arrays(b)?)
            } else {
                (a, b)
            };
            match op.as_str() {
                "and" | "or" => vec![Item::Boolean(boolean(&b)?)],
                "||" => {
                    if a.len() > 1 || b.len() > 1 {
                        return Err(err(
                            "XPTY0004",
                            "string concatenation requires optional singletons",
                        ));
                    }
                    let mut left = a.first().map_or_else(String::new, Item::string);
                    let right = b.first().map_or_else(String::new, Item::string);
                    for _ in left.bytes().chain(right.bytes()) {
                        budget.tick()?;
                    }
                    left.push_str(&right);
                    vec![Item::String(left)]
                }
                "to" => {
                    fn bound(value: &[Item<'_>]) -> Result<Option<num_bigint::BigInt>, Diagnostic> {
                        match value {
                            [] => Ok(None),
                            [item] => match atomize(item) {
                                Item::Number(Number::Integer(n) | Number::DerivedInteger(_, n)) => {
                                    Ok(Some(n))
                                }
                                Item::Untyped(s) => {
                                    let Number::Integer(n) = Number::cast("integer", &s)? else {
                                        unreachable!()
                                    };
                                    Ok(Some(n))
                                }
                                _ => Err(err("XPTY0004", "range bounds must be optional integers")),
                            },
                            _ => Err(err("XPTY0004", "range bounds must be optional integers")),
                        }
                    }
                    let (Some(mut start), Some(end)) = (bound(&a)?, bound(&b)?) else {
                        return Ok(vec![]);
                    };
                    if start > end {
                        return Ok(vec![]);
                    }
                    let length = &end - &start + num_bigint::BigInt::from(1u8);
                    if length > num_bigint::BigInt::from(budget.remaining) {
                        return Err(err(
                            "XPDY0130",
                            "range exceeds the remaining execution budget",
                        ));
                    }
                    let mut items = vec![];
                    while start <= end {
                        budget.tick()?;
                        items.push(Item::Number(Number::Integer(start.clone())));
                        start += 1;
                    }
                    items
                }
                "|" | "union" | "intersect" | "except" => {
                    let mut nodes = vec![];
                    let mut right = std::collections::HashSet::new();
                    for item in b {
                        budget.tick()?;
                        let Item::Node(node) = item else {
                            return Err(err("XPTY0004", "set operators require node operands"));
                        };
                        if matches!(op.as_str(), "|" | "union") {
                            nodes.push(node);
                        } else {
                            right.insert(node);
                        }
                    }
                    for item in a {
                        budget.tick()?;
                        let Item::Node(node) = item else {
                            return Err(err("XPTY0004", "set operators require node operands"));
                        };
                        if match op.as_str() {
                            "intersect" => right.contains(&node),
                            "except" => !right.contains(&node),
                            _ => true,
                        } {
                            nodes.push(node);
                        }
                    }
                    ordered(nodes)
                }
                "is" | "<<" | ">>" => {
                    if a.is_empty() || b.is_empty() {
                        return Ok(vec![]);
                    }
                    let ([Item::Node(a)], [Item::Node(b)]) = (a.as_slice(), b.as_slice()) else {
                        return Err(err("XPTY0004", "node comparison requires singleton nodes"));
                    };
                    vec![Item::Boolean(match op.as_str() {
                        "is" => a == b,
                        "<<" => a.order() < b.order(),
                        _ => a.order() > b.order(),
                    })]
                }
                "eq" | "ne" | "lt" | "gt" | "le" | "ge" => {
                    if a.is_empty() || b.is_empty() {
                        return Ok(vec![]);
                    }
                    if a.len() != 1 || b.len() != 1 {
                        return Err(err(
                            "XPTY0004",
                            "value comparison requires singleton operands",
                        ));
                    }
                    let op = match op.as_str() {
                        "eq" => "=",
                        "ne" => "!=",
                        "lt" => "<",
                        "gt" => ">",
                        "le" => "<=",
                        _ => ">=",
                    };
                    let atom = |i: &Item<'a>| match i {
                        Item::Node(n) => Item::String(n.string()),
                        Item::Untyped(s) => Item::String(s.clone()),
                        _ => i.clone(),
                    };
                    vec![Item::Boolean(compare(
                        &atom(&a[0]),
                        &atom(&b[0]),
                        op,
                        false,
                    )?)]
                }
                "=" | "!=" | "<" | ">" | "<=" | ">=" => {
                    if backwards
                        && ["=", "!="].contains(&op.as_str())
                        && (a.iter().any(|i| matches!(i, Item::Boolean(_)))
                            || b.iter().any(|i| matches!(i, Item::Boolean(_))))
                    {
                        let equal = boolean(&a)? == boolean(&b)?;
                        return Ok(vec![Item::Boolean(if op == "=" { equal } else { !equal })]);
                    }
                    let mut yes = false;
                    for x in &a {
                        for y in &b {
                            budget.tick()?;
                            if compare(x, y, op, backwards)? {
                                yes = true;
                            }
                        }
                    }
                    vec![Item::Boolean(yes)]
                }
                _ => {
                    if !backwards && (a.is_empty() || b.is_empty()) {
                        return Ok(vec![]);
                    }
                    if !backwards && let ([left], [right]) = (a.as_slice(), b.as_slice()) {
                        let (left, right) = (atomize(left), atomize(right));
                        if matches!(left, Item::Temporal(_)) || matches!(right, Item::Temporal(_)) {
                            return Ok(vec![temporal_arithmetic(op, &left, &right)?]);
                        }
                    }
                    let x = numeric(&a, backwards)?;
                    let y = numeric(&b, backwards)?;
                    vec![Item::Number(x.arithmetic(&y, op)?)]
                }
            }
        }
        Expr::Call(name, args) => {
            let args = args
                .iter()
                .map(|e| eval(e, focus, vars, budget, depth + 1, backwards))
                .collect::<Result<Vec<_>, _>>()?;
            call_named(name, args, focus, vars, budget, depth, backwards)?
        }
        Expr::MapConstructor(entries) => {
            let mut map = MapValue::default();
            for (key, value) in entries {
                let key = eval(key, focus, vars, budget, depth + 1, backwards)?;
                let key = crate::maps::key(&key)?;
                if map.get(&key).is_some() {
                    return Err(err("XQDY0137", "duplicate key in map constructor"));
                }
                let value = eval(value, focus, vars, budget, depth + 1, backwards)?;
                map.push_new(key, value);
            }
            vec![Item::Map(std::rc::Rc::new(map))]
        }
        Expr::ArraySquare(members) => {
            let mut array = Vec::with_capacity(members.len());
            for member in members {
                array.push(eval(member, focus, vars, budget, depth + 1, backwards)?);
            }
            vec![Item::Array(std::rc::Rc::new(array))]
        }
        Expr::ArrayCurly(inner) => {
            let items = eval(inner, focus, vars, budget, depth + 1, backwards)?;
            vec![Item::Array(std::rc::Rc::new(
                items.into_iter().map(|i| vec![i]).collect(),
            ))]
        }
        Expr::Lookup(base, key) => {
            let base = eval(base, focus, vars, budget, depth + 1, backwards)?;
            let keys = match key {
                Some(key) => Some(eval(key, focus, vars, budget, depth + 1, backwards)?),
                None => None,
            };
            crate::maps::lookup(&base, keys.as_deref(), budget)?
        }
        Expr::InlineFunction(def) => {
            let captured = without_grouping_context(vars);
            vec![Item::Function(std::rc::Rc::new(FunctionValue::Inline(
                def.clone(),
                captured,
            )))]
        }
        Expr::FunctionRef(name, arity, namespaces) => {
            vec![Item::Function(named_function(
                name.clone(),
                *arity,
                namespaces.clone(),
                vars,
            ))]
        }
        Expr::Evaluate {
            xpath,
            context,
            params,
            namespaces,
        } => {
            let source = eval(xpath, focus, vars, budget, depth + 1, backwards)?;
            let source = match source.as_slice() {
                [item] => atomize(item).string(),
                _ => return Err(err("XTDE3160", "xsl:evaluate needs a single XPath string")),
            };
            for _ in source.bytes() {
                budget.tick()?;
            }
            let parsed =
                with_xpath_profile(|| parse(&source, namespaces.clone())).map_err(|mut e| {
                    e.code = "XTDE3160".into();
                    e.phase = "run";
                    e
                })?;
            let mut scope: Variables<'a> = vars
                .iter()
                .filter(|(k, _)| k.starts_with(GLOBAL_PREFIX))
                .map(|(k, v)| (k[GLOBAL_PREFIX.len()..].to_owned(), v.clone()))
                .collect();
            for (name, value) in params {
                scope.insert(
                    name.clone(),
                    eval(value, focus, vars, budget, depth + 1, backwards)?,
                );
            }
            let inner = match context {
                Some(c) => {
                    let items = eval(c, focus, vars, budget, depth + 1, backwards)?;
                    match items.as_slice() {
                        [item] => Focus {
                            item: item.clone(),
                            position: 1,
                            size: 1,
                        },
                        _ => return Err(err("XTTE3210", "xsl:evaluate context must be one item")),
                    }
                }
                None => Focus {
                    item: Item::Absent,
                    position: 0,
                    size: 0,
                },
            };
            scope.insert(CURRENT_ITEM.into(), vec![inner.item.clone()]);
            eval(&parsed, &inner, &scope, budget, depth + 1, backwards)?
        }
        Expr::Sorted(inner, keys) => {
            let value = eval(inner, focus, vars, budget, depth + 1, backwards)?;
            crate::sort::sort(value, keys, vars, focus, budget, backwards)?
        }
        Expr::Partial(callee, slots) => {
            let callee = eval(callee, focus, vars, budget, depth + 1, backwards)?;
            let base = function_arg(&callee)?;
            if base.arity() != slots.len() {
                return Err(err(
                    "XPTY0004",
                    "partial application has the wrong number of arguments",
                ));
            }
            let mut fixed = Vec::with_capacity(slots.len());
            for slot in slots {
                fixed.push(match slot {
                    Some(e) => Some(eval(e, focus, vars, budget, depth + 1, backwards)?),
                    None => None,
                });
            }
            vec![Item::Function(std::rc::Rc::new(FunctionValue::Partial(
                base, fixed,
            )))]
        }
        Expr::DynamicCall(callee, args) => {
            let callee = eval(callee, focus, vars, budget, depth + 1, backwards)?;
            let args = args
                .iter()
                .map(|e| eval(e, focus, vars, budget, depth + 1, backwards))
                .collect::<Result<Vec<_>, _>>()?;
            let f = function_arg(&callee)?;
            call_function_item(&f, args, focus, vars, budget, depth, backwards)?
        }
    };
    Ok(result)
}
fn call_named<'a>(
    name: &str,
    args: Vec<Value<'a>>,
    focus: &Focus<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    depth: usize,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    Ok(if name == "current" {
        match vars.get(CURRENT_ITEM).and_then(|value| value.first()) {
            Some(item) if !matches!(item, Item::Absent) => vec![item.clone()],
            _ => return Err(err("XTDE1360", "there is no current item")),
        }
    } else if name == "current-group" {
        vars.get(CURRENT_GROUP).cloned().ok_or_else(|| {
            err(
                "XTDE1061",
                "current-group() is used outside xsl:for-each-group",
            )
        })?
    } else if name == "current-grouping-key" {
        vars.get(CURRENT_KEY)
            .cloned()
            .ok_or_else(|| err("XTDE1071", "there is no current grouping key"))?
    } else if matches!(name, "id" | "element-with-id") {
        // Only xml:id attributes carry the ID type without a DTD or schema.
        let start = match args.get(1).map(Vec::as_slice) {
            Some([Item::Node(n)]) => *n,
            Some(_) => {
                return Err(err(
                    "XPTY0004",
                    "the id node argument must be a single node",
                ));
            }
            None => match focus.item {
                Item::Node(n) => n,
                _ => return Err(err("XPDY0002", "id() needs a context node")),
            },
        };
        let root = start.root();
        if !root.is_document() {
            return Err(err("FODC0001", "id() needs a node in a document tree"));
        }
        let mut wanted = std::collections::BTreeSet::new();
        for item in &args[0] {
            for token in atomize(item).string().split_whitespace() {
                wanted.insert(token.to_owned());
            }
        }
        let mut found = vec![];
        for element in root.descendants().into_iter().filter(|n| n.is_element()) {
            budget.tick()?;
            if element
                .raw
                .attribute(("http://www.w3.org/XML/1998/namespace", "id"))
                .is_some_and(|id| wanted.contains(id.trim()))
            {
                found.push(element);
            }
        }
        ordered(found)
    } else if matches!(name, "doc" | "document" | "doc-available") {
        let mut found: Value<'a> = vec![];
        let mut available = true;
        for item in &args[0] {
            let uri = atomize(item).string();
            match vars.get(&document_key(uri.trim())) {
                Some(document) => found.extend(document.iter().cloned()),
                None => available = false,
            }
        }
        if name == "doc-available" {
            vec![Item::Boolean(available && args[0].len() == 1)]
        } else if !available {
            return Err(err("FODC0002", "the document could not be loaded"));
        } else {
            found
        }
    } else if name == "regex-group" {
        let index = match args[0].as_slice() {
            [Item::Number(n)] => n.double(),
            _ => return Err(err("XPTY0004", "regex-group requires an integer")),
        };
        let groups = vars.get(REGEX_GROUPS);
        match groups.and_then(|g| g.get(index as usize).filter(|_| index >= 0.0)) {
            Some(group) => vec![group.clone()],
            None => vec![Item::String(String::new())],
        }
    } else if name == "key" {
        call_key(args, focus, vars, budget)?
    } else if name.starts_with("Q{") {
        call_user_function(name, args, vars, budget, depth, backwards)?
    } else if matches!(
        name,
        "for-each"
            | "filter"
            | "fold-left"
            | "fold-right"
            | "for-each-pair"
            | "function-arity"
            | "function-lookup"
            | "function-name"
            | "apply"
    ) || name == "sort" && args.len() == 3
        || crate::maps::higher_order(name)
    {
        higher_order(name, args, focus, vars, budget, depth, backwards)?
    } else {
        function(name, &args, focus, budget, backwards)?
    })
}
/// Atomization of arrays: members are atomized recursively; maps and functions
/// have no typed value (FOTY0013).
pub(crate) fn flatten_arrays(value: Value<'_>) -> Result<Value<'_>, Diagnostic> {
    if !value.iter().any(|i| matches!(i, Item::Array(_))) {
        if value
            .iter()
            .any(|i| matches!(i, Item::Map(_) | Item::Function(_)))
        {
            return Err(err("FOTY0013", "maps and functions cannot be atomized"));
        }
        return Ok(value);
    }
    let mut out = Vec::with_capacity(value.len());
    let mut pending: Vec<Item<'_>> = value.into_iter().rev().collect();
    while let Some(item) = pending.pop() {
        match item {
            Item::Array(members) => {
                for member in members.iter().rev() {
                    pending.extend(member.iter().rev().cloned());
                }
            }
            Item::Map(_) | Item::Function(_) => {
                return Err(err("FOTY0013", "maps and functions cannot be atomized"));
            }
            other => out.push(other),
        }
    }
    Ok(out)
}
fn function_arg<'a>(value: &[Item<'a>]) -> Result<std::rc::Rc<FunctionValue<'a>>, Diagnostic> {
    match value {
        [Item::Function(f)] => Ok(f.clone()),
        [Item::Map(m)] => Ok(std::rc::Rc::new(FunctionValue::Map(m.clone()))),
        [Item::Array(a)] => Ok(std::rc::Rc::new(FunctionValue::Array(a.clone()))),
        _ => Err(err("XPTY0004", "a single function item is required")),
    }
}
fn static_name<'a>(
    value: &[Item<'a>],
    namespaces: &BTreeMap<String, String>,
    backwards: bool,
    default_namespace: bool,
    code: &str,
) -> Result<Value<'a>, Diagnostic> {
    let text = crate::string_functions::string(value, false, backwards)?;
    let text = text.trim_matches([' ', '\t', '\n', '\r']);
    let resolved = crate::qname::resolve(text, namespaces, code)
        .map_err(|_| err(code, "invalid or unbound name"))?;
    let resolved = if code == "XTDE1400" && !text.starts_with("Q{") && !text.contains(':') {
        format!("Q{{http://www.w3.org/2005/xpath-functions}}{resolved}")
    } else if default_namespace && !text.starts_with("Q{") && !text.contains(':') {
        match namespaces.get("").filter(|uri| !uri.is_empty()) {
            Some(uri) => format!("Q{{{uri}}}{resolved}"),
            None => resolved,
        }
    } else {
        resolved
    };
    Ok(vec![Item::String(resolved)])
}
fn unwrap_lookup_call(expr: Expr) -> Expr {
    match expr {
        Expr::LookupContext(inner, _) => *inner,
        other => other,
    }
}

fn attach_lookup_context<'a>(value: Value<'a>, namespaces: &BTreeMap<String, String>) -> Value<'a> {
    value
        .into_iter()
        .map(|item| match item {
            Item::Function(function) => match function.as_ref() {
                FunctionValue::Named(name, arity, _)
                    if matches!(
                        name.as_str(),
                        "element-available"
                            | "system-property"
                            | "type-available"
                            | "function-available"
                            | "function-lookup"
                    ) =>
                {
                    Item::Function(std::rc::Rc::new(FunctionValue::Named(
                        name.clone(),
                        *arity,
                        Some(namespaces.clone()),
                    )))
                }
                _ => Item::Function(function),
            },
            other => other,
        })
        .collect()
}

fn named_function<'a>(
    name: String,
    arity: usize,
    namespaces: Option<BTreeMap<String, String>>,
    vars: &Variables<'a>,
) -> std::rc::Rc<FunctionValue<'a>> {
    std::rc::Rc::new(if name == "current" && arity == 0 {
        FunctionValue::Current(
            vars.get(CURRENT_ITEM)
                .and_then(|value| value.first())
                .cloned()
                .unwrap_or(Item::Absent),
        )
    } else {
        FunctionValue::Named(name, arity, namespaces)
    })
}

fn without_grouping_context<'a>(vars: &Variables<'a>) -> Variables<'a> {
    vars.iter()
        .filter(|(name, _)| !matches!(name.as_str(), CURRENT_GROUP | CURRENT_KEY))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn call_function_item<'a>(
    f: &FunctionValue<'a>,
    args: Vec<Value<'a>>,
    focus: &Focus<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    depth: usize,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    if args.len() != f.arity() {
        return Err(err(
            "XPTY0004",
            format!(
                "function of arity {} called with {} arguments",
                f.arity(),
                args.len()
            ),
        ));
    }
    match f {
        FunctionValue::Current(item) => {
            if matches!(item, Item::Absent) {
                Err(err("XTDE1360", "there is no current item"))
            } else {
                Ok(vec![item.clone()])
            }
        }
        FunctionValue::Named(name, _, namespaces) => {
            let mut args = args;
            if let Some(namespaces) = namespaces.as_ref().filter(|_| {
                matches!(
                    name.as_str(),
                    "element-available"
                        | "system-property"
                        | "type-available"
                        | "function-available"
                )
            }) {
                args[0] = static_name(
                    &args[0],
                    namespaces,
                    backwards,
                    name != "system-property",
                    if name == "element-available" {
                        "XTDE1440"
                    } else if name == "type-available" {
                        "XTDE1428"
                    } else if name == "function-available" {
                        "XTDE1400"
                    } else {
                        "XTDE1390"
                    },
                )?;
            }
            let result = if vars.contains_key(CURRENT_GROUP) || vars.contains_key(CURRENT_KEY) {
                let scope = without_grouping_context(vars);
                call_named(name, args, focus, &scope, budget, depth + 1, backwards)?
            } else {
                call_named(name, args, focus, vars, budget, depth + 1, backwards)?
            };
            Ok(match namespaces {
                Some(context) if name == "function-lookup" => {
                    attach_lookup_context(result, context)
                }
                _ => result,
            })
        }
        FunctionValue::Partial(base, slots) => {
            let mut supplied = args.into_iter();
            let full = slots
                .iter()
                .map(|slot| match slot {
                    Some(value) => value.clone(),
                    None => supplied.next().unwrap_or_default(),
                })
                .collect();
            call_function_item(base, full, focus, vars, budget, depth + 1, backwards)
        }
        FunctionValue::Map(map) => {
            let [key] = args[0].as_slice() else {
                return Err(err(
                    "XPTY0004",
                    "a map lookup key must be a single atomic value",
                ));
            };
            Ok(map.get(&atomize(key)).cloned().unwrap_or_default())
        }
        FunctionValue::Array(array) => crate::maps::array_member(array, &args[0]).cloned(),
        FunctionValue::Inline(def, captured) => {
            let mut scope = captured.clone();
            for ((param, ty), value) in def.params.iter().zip(args) {
                let value = match ty {
                    Some(ty) => ty.convert(value, "XPTY0004")?,
                    None => value,
                };
                scope.insert(param.clone(), value);
            }
            let absent = Focus {
                item: Item::Absent,
                position: 0,
                size: 0,
            };
            let result = eval(&def.body, &absent, &scope, budget, depth + 1, backwards)?;
            match &def.ret {
                Some(ty) => ty.convert(result, "XPTY0004"),
                None => Ok(result),
            }
        }
    }
}
fn higher_order<'a>(
    name: &str,
    args: Vec<Value<'a>>,
    focus: &Focus<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    depth: usize,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    let call = |f: &FunctionValue<'a>, args: Vec<Value<'a>>, budget: &mut Budget| {
        budget.tick()?;
        call_function_item(f, args, focus, vars, budget, depth, backwards)
    };
    match name {
        "function-arity" => {
            let f = function_arg(&args[0])?;
            Ok(vec![Item::Number(Number::from(f.arity()))])
        }
        "function-name" => {
            let f = function_arg(&args[0])?;
            let name = match &*f {
                FunctionValue::Named(name, _, _) => name.as_str(),
                FunctionValue::Current(_) => "current",
                _ => return Ok(vec![]),
            };
            let (uri, prefix, local) = if let Some(rest) = name.strip_prefix("Q{") {
                let (uri, local) = rest.split_once('}').unwrap_or(("", rest));
                (uri.to_owned(), None, local.to_owned())
            } else if let Some(local) = name.strip_prefix("xs:") {
                (
                    "http://www.w3.org/2001/XMLSchema".into(),
                    Some("xs"),
                    local.to_owned(),
                )
            } else if let Some(local) = name.strip_prefix("math:") {
                (
                    "http://www.w3.org/2005/xpath-functions/math".into(),
                    Some("math"),
                    local.to_owned(),
                )
            } else if let Some(local) = name.strip_prefix("map:") {
                (
                    "http://www.w3.org/2005/xpath-functions/map".into(),
                    Some("map"),
                    local.to_owned(),
                )
            } else if let Some(local) = name.strip_prefix("array:") {
                (
                    "http://www.w3.org/2005/xpath-functions/array".into(),
                    Some("array"),
                    local.to_owned(),
                )
            } else {
                (
                    "http://www.w3.org/2005/xpath-functions".into(),
                    Some("fn"),
                    name.to_owned(),
                )
            };
            let lexical = match prefix {
                Some(p) => format!("{p}:{local}"),
                None => local,
            };
            Ok(vec![Item::QName(crate::qname::AtomicQName::new(
                &uri, &lexical,
            )?)])
        }
        "apply" => {
            let f = function_arg(&args[0])?;
            let members = crate::maps::members(&args[1])?;
            if members.len() != f.arity() {
                return Err(err(
                    "FOAP0001",
                    "apply: wrong number of arguments for the function",
                ));
            }
            call(&f, members.to_vec(), budget)
        }
        "function-lookup" => {
            let (qname, arity) = match (args[0].as_slice(), args[1].as_slice()) {
                ([Item::QName(q)], [Item::Number(n)]) => (q.clone(), n.double()),
                _ => {
                    return Err(err(
                        "XPTY0004",
                        "function-lookup requires a QName and an integer",
                    ));
                }
            };
            let arity = arity as usize;
            let uri = qname.uri.as_deref().unwrap_or("");
            let local = qname.local.as_str();
            let fits = |name: &str| {
                (!budget.xpath_only || !xslt_function(name))
                    && function_arity(name).is_some_and(|(lo, hi)| (lo..=hi).contains(&arity))
            };
            let known = match uri {
                "" => None,
                "http://www.w3.org/2005/xpath-functions" => fits(local).then(|| local.to_owned()),
                "http://www.w3.org/2005/xpath-functions/math" => {
                    let math = format!("math:{local}");
                    fits(&math).then_some(math)
                }
                "http://www.w3.org/2005/xpath-functions/map" => {
                    let map = format!("map:{local}");
                    fits(&map).then_some(map)
                }
                "http://www.w3.org/2005/xpath-functions/array" => {
                    let array = format!("array:{local}");
                    fits(&array).then_some(array)
                }
                "http://www.w3.org/2001/XMLSchema" => {
                    let xs = format!("xs:{local}");
                    // xs:QName needs the static namespaces of the call site.
                    (local != "QName" && fits(&xs)).then_some(xs)
                }
                _ => {
                    let expanded = format!("Q{{{uri}}}{local}");
                    budget
                        .functions
                        .contains_key(&(expanded.clone(), arity))
                        .then_some(expanded)
                }
            };
            Ok(known
                .map(|n| vec![Item::Function(named_function(n, arity, None, vars))])
                .unwrap_or_default())
        }
        "for-each" => {
            let f = function_arg(&args[1])?;
            let mut out = vec![];
            for item in &args[0] {
                out.extend(call(&f, vec![vec![item.clone()]], budget)?);
            }
            Ok(out)
        }
        "filter" => {
            let f = function_arg(&args[1])?;
            let mut out = vec![];
            for item in &args[0] {
                let keep = call(&f, vec![vec![item.clone()]], budget)?;
                match keep.as_slice() {
                    [Item::Boolean(true)] => out.push(item.clone()),
                    [Item::Boolean(false)] => {}
                    _ => return Err(err("XPTY0004", "filter predicate must return one boolean")),
                }
            }
            Ok(out)
        }
        "fold-left" => {
            let f = function_arg(&args[2])?;
            let mut acc = args[1].clone();
            for item in &args[0] {
                acc = call(&f, vec![acc, vec![item.clone()]], budget)?;
            }
            Ok(acc)
        }
        "fold-right" => {
            let f = function_arg(&args[2])?;
            let mut acc = args[1].clone();
            for item in args[0].iter().rev() {
                acc = call(&f, vec![vec![item.clone()], acc], budget)?;
            }
            Ok(acc)
        }
        "for-each-pair" => {
            let f = function_arg(&args[2])?;
            let mut out = vec![];
            for (a, b) in args[0].iter().zip(&args[1]) {
                out.extend(call(&f, vec![vec![a.clone()], vec![b.clone()]], budget)?);
            }
            Ok(out)
        }
        "map:for-each" => {
            let f = function_arg(&args[1])?;
            let mut out = vec![];
            for (k, v) in crate::maps::entries(&args[0])?.iter() {
                out.extend(call(&f, vec![vec![k.clone()], v.clone()], budget)?);
            }
            Ok(out)
        }
        "array:for-each" => {
            let f = function_arg(&args[1])?;
            let mut members = vec![];
            for member in crate::maps::members(&args[0])?.iter() {
                members.push(call(&f, vec![member.clone()], budget)?);
            }
            Ok(crate::maps::new_array(members))
        }
        "array:filter" => {
            let f = function_arg(&args[1])?;
            let mut members = vec![];
            for member in crate::maps::members(&args[0])?.iter() {
                match call(&f, vec![member.clone()], budget)?.as_slice() {
                    [Item::Boolean(true)] => members.push(member.clone()),
                    [Item::Boolean(false)] => {}
                    _ => return Err(err("XPTY0004", "filter predicate must return one boolean")),
                }
            }
            Ok(crate::maps::new_array(members))
        }
        "array:fold-left" => {
            let f = function_arg(&args[2])?;
            let mut acc = args[1].clone();
            for member in crate::maps::members(&args[0])?.iter() {
                acc = call(&f, vec![acc, member.clone()], budget)?;
            }
            Ok(acc)
        }
        "array:fold-right" => {
            let f = function_arg(&args[2])?;
            let mut acc = args[1].clone();
            for member in crate::maps::members(&args[0])?.iter().rev() {
                acc = call(&f, vec![member.clone(), acc], budget)?;
            }
            Ok(acc)
        }
        "array:for-each-pair" => {
            let f = function_arg(&args[2])?;
            let (a, b) = (
                crate::maps::members(&args[0])?,
                crate::maps::members(&args[1])?,
            );
            let mut members = vec![];
            for (x, y) in a.iter().zip(b.iter()) {
                members.push(call(&f, vec![x.clone(), y.clone()], budget)?);
            }
            Ok(crate::maps::new_array(members))
        }
        "array:sort" => {
            let array = crate::maps::members(&args[0])?;
            let members: Value<'a> = array
                .iter()
                .map(|m| Item::Array(std::rc::Rc::new(vec![m.clone()])))
                .collect();
            let mut keys = Vec::with_capacity(members.len());
            for member in array.iter() {
                keys.push(match args.get(2) {
                    Some(f) => {
                        let f = function_arg(f)?;
                        call(&f, vec![member.clone()], budget)?
                    }
                    None => member.clone(),
                });
            }
            let sorted = crate::aggregate::sort(
                &[members, args.get(1).cloned().unwrap_or_default()][..args.len().clamp(1, 2)],
                Some(keys),
                budget,
            )?;
            let members = sorted
                .into_iter()
                .map(|item| match item {
                    Item::Array(a) => a[0].clone(),
                    _ => unreachable!(),
                })
                .collect();
            Ok(crate::maps::new_array(members))
        }
        _ => {
            // sort with a key function
            let f = function_arg(&args[2])?;
            let mut keys = Vec::with_capacity(args[0].len());
            for item in &args[0] {
                keys.push(call(&f, vec![vec![item.clone()]], budget)?);
            }
            crate::aggregate::sort(&args[..2], Some(keys), budget)
        }
    }
}
/// Keys compare untyped values as strings; compatibility keys stringify every value.
fn key_values<'a>(
    value: Value<'a>,
    backwards: bool,
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    flatten_arrays(value)?
        .iter()
        .map(|item| {
            budget.tick()?;
            let atomic = atomize(item);
            Ok(if backwards {
                Item::String(atomic.string())
            } else if let Item::Untyped(s) = atomic {
                Item::String(s)
            } else {
                atomic
            })
        })
        .collect()
}

fn owned_key_value(item: Item<'_>, budget: &mut Budget) -> Result<Item<'static>, Diagnostic> {
    budget.charge(match &item {
        Item::String(s) | Item::Untyped(s) | Item::AnyUri(s) | Item::DerivedString(_, s) => s.len(),
        Item::Binary(_, bytes) => bytes.len(),
        _ => item.string().len(),
    })?;
    Ok(match item {
        Item::String(s) => Item::String(s),
        Item::Untyped(s) => Item::Untyped(s),
        Item::AnyUri(s) => Item::AnyUri(s),
        Item::DerivedString(kind, s) => Item::DerivedString(kind, s),
        Item::Number(n) => Item::Number(n),
        Item::Boolean(b) => Item::Boolean(b),
        Item::QName(q) => Item::QName(q),
        Item::Binary(kind, bytes) => Item::Binary(kind, bytes),
        Item::Temporal(t) => Item::Temporal(t),
        _ => return Err(err("FOTY0013", "key values must be atomic")),
    })
}

fn call_key<'a>(
    args: Vec<Value<'a>>,
    focus: &Focus<'a>,
    vars: &Variables<'a>,
    budget: &mut Budget,
) -> Result<Value<'a>, Diagnostic> {
    let name = match args[0].as_slice() {
        [item] => atomize(item).string(),
        _ => return Err(err("XPTY0004", "key name must be a singleton string")),
    };
    let keys = budget.keys.clone();
    let definitions = keys
        .get(&name)
        .ok_or_else(|| err("XTDE1260", format!("unknown key: {name}")))?;
    let start = match args.get(2).map(Vec::as_slice) {
        Some([Item::Node(n)]) => *n,
        Some(_) => return Err(err("XTTE1100", "key top argument must be a single node")),
        None => match focus.item {
            Item::Node(n) => n,
            _ => return Err(err("XTDE1270", "key requires a context node")),
        },
    };
    if !start.root().is_document() {
        return Err(err(
            "XTDE1270",
            "key requires a tree rooted at a document node",
        ));
    }
    // Key declarations use their global lexical scope, regardless of caller locals.
    let scope: Variables<'a> = vars
        .iter()
        .filter_map(|(k, v)| {
            let plain = k.strip_prefix(GLOBAL_PREFIX)?;
            Some([(plain.to_owned(), v.clone()), (k.clone(), v.clone())])
        })
        .flatten()
        .collect();
    let compatibility = definitions.iter().any(|definition| definition.backwards);
    let wanted = key_values(args[1].clone(), compatibility, budget)?;
    let composite = definitions[0].composite;
    if wanted.is_empty() && !composite {
        return Ok(vec![]);
    }
    let root = start.root();
    let order = root.order();
    let identity = (name, order.0, order.1);
    let index = if let Some(index) = budget.key_indexes.get(&identity) {
        index.clone()
    } else {
        if budget.active_keys.contains(&identity) {
            return Err(err("XTDE0640", "circular dependency in key definitions"));
        }
        if budget.active_keys.len() >= budget.max_depth.min(128) {
            return Err(err("XPDY0130", "key dependency depth limit exceeded"));
        }
        budget.active_keys.push(identity.clone());
        let result = (|| {
            let mut entries = vec![];
            let mut candidates = root.descendants();
            for node in candidates.clone() {
                if node.is_element() {
                    candidates.extend(node.attributes());
                }
            }
            for node in candidates {
                budget.tick()?;
                for definition in definitions {
                    let mut matched = false;
                    for branch in &definition.branches {
                        if crate::pattern::matches(
                            branch,
                            node,
                            &scope,
                            budget,
                            definition.backwards,
                        )? {
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        continue;
                    }
                    let at = Focus {
                        item: Item::Node(node),
                        position: 1,
                        size: 1,
                    };
                    let mut key_scope = scope.clone();
                    key_scope.insert(CURRENT_ITEM.into(), vec![Item::Node(node)]);
                    let values = eval(
                        &definition.value,
                        &at,
                        &key_scope,
                        budget,
                        1,
                        definition.backwards,
                    )?;
                    let values = key_values(values, compatibility, budget)?
                        .into_iter()
                        .map(|item| owned_key_value(item, budget))
                        .collect::<Result<_, _>>()?;
                    entries.push(KeyEntry {
                        node: node.raw.id(),
                        attribute: node
                            .attribute
                            .map(|(local, uri)| (local.to_owned(), uri.map(str::to_owned))),
                        values,
                    });
                }
            }
            Ok(std::rc::Rc::new(entries))
        })();
        budget.active_keys.pop();
        let index = result?;
        budget.key_indexes.insert(identity, index.clone());
        index
    };
    let collation = definitions[0].collation;
    let mut found = vec![];
    for entry in index.iter() {
        budget.tick()?;
        let mut equal = composite && entry.values.len() == wanted.len();
        if composite {
            if equal {
                for (value, requested) in entry.values.iter().zip(&wanted) {
                    budget.tick()?;
                    if !crate::deep_equal::atomic_equal_with(value, requested, false, collation) {
                        equal = false;
                        break;
                    }
                }
            }
        } else {
            'values: for value in &entry.values {
                for requested in &wanted {
                    budget.tick()?;
                    if crate::deep_equal::atomic_equal_with(value, requested, false, collation) {
                        equal = true;
                        break 'values;
                    }
                }
            }
        }
        if !equal {
            continue;
        }
        let raw = root
            .raw
            .document()
            .get_node(entry.node)
            .ok_or_else(|| err("XPDY0002", "cached key node is unavailable"))?;
        let mut node = root.relative(raw);
        if let Some((local, uri)) = &entry.attribute {
            node = node
                .attributes()
                .into_iter()
                .find(|attribute| {
                    attribute
                        .attribute
                        .is_some_and(|(l, u)| l == local && u == uri.as_deref())
                })
                .ok_or_else(|| err("XPDY0002", "cached key attribute is unavailable"))?;
        }
        if args.len() == 3 {
            let mut ancestor = Some(node);
            let mut in_subtree = false;
            while let Some(candidate) = ancestor {
                budget.tick()?;
                if candidate == start {
                    in_subtree = true;
                    break;
                }
                ancestor = candidate.parent();
            }
            if !in_subtree {
                continue;
            }
        }
        found.push(node);
    }
    Ok(ordered(found))
}
fn call_user_function<'a>(
    name: &str,
    args: Vec<Value<'a>>,
    vars: &Variables<'a>,
    budget: &mut Budget,
    depth: usize,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    let functions = budget.functions.clone();
    let function = functions
        .get(&(name.to_owned(), args.len()))
        .ok_or_else(|| {
            err(
                "XPST0017",
                format!("unknown function: {name}#{}", args.len()),
            )
        })?;
    let mut scope: Variables<'a> = vars
        .iter()
        .filter_map(|(k, v)| {
            let plain = k.strip_prefix(GLOBAL_PREFIX)?;
            Some([(plain.to_owned(), v.clone()), (k.clone(), v.clone())])
        })
        .flatten()
        .collect();
    for ((param, ty), value) in function.params.iter().zip(args) {
        let value = match ty {
            Some(ty) => ty.convert(value, "XTTE0790")?,
            None => value,
        };
        scope.insert(param.clone(), value);
    }
    let absent = Focus {
        item: Item::Absent,
        position: 0,
        size: 0,
    };
    let result = eval(
        &function.body,
        &absent,
        &scope,
        budget,
        depth + 1,
        backwards,
    )?;
    match &function.result {
        Some(ty) => ty.convert(result, "XTTE0780"),
        None => Ok(result),
    }
}
fn temporal_arithmetic<'a>(
    op: &str,
    left: &Item<'a>,
    right: &Item<'a>,
) -> Result<Item<'a>, Diagnostic> {
    let wrong = || {
        err(
            "XPTY0004",
            "unsupported operand types for temporal arithmetic",
        )
    };
    Ok(match (left, right, op) {
        (Item::Temporal(a), Item::Temporal(b), "+") => Item::Temporal(a.add(b)?),
        (Item::Temporal(a), Item::Temporal(b), "-") => Item::Temporal(a.sub(b)?),
        (Item::Temporal(a), Item::Temporal(b), "div") => {
            let (x, y) = a.ratio(b)?;
            Item::Number(Number::Integer(x.into()).arithmetic(&Number::Integer(y.into()), "div")?)
        }
        (Item::Temporal(a), Item::Number(n), "*" | "div") => {
            Item::Temporal(a.scale(n.double(), op == "div")?)
        }
        (Item::Number(n), Item::Temporal(a), "*") => Item::Temporal(a.scale(n.double(), false)?),
        _ => return Err(wrong()),
    })
}
fn numeric(v: &[Item<'_>], backwards: bool) -> Result<Number, Diagnostic> {
    if backwards {
        return Ok(Number::Double(v.first().map_or(f64::NAN, Item::number)));
    }
    match v {
        [Item::Number(n)] => Ok(n.clone()),
        [Item::Node(n)] if !n.is_comment() && !n.is_pi() => Number::cast("double", &n.string()),
        [Item::Untyped(s)] => Number::cast("double", s),
        [] => Ok(Number::Double(f64::NAN)),
        _ => Err(err(
            "XPTY0004",
            "numeric operand must be a numeric or untyped singleton",
        )),
    }
}
pub(crate) fn compare(
    a: &Item<'_>,
    b: &Item<'_>,
    op: &str,
    backwards: bool,
) -> Result<bool, Diagnostic> {
    use std::cmp::Ordering;
    let (mut a, mut b) = (atomize(a), atomize(b));
    if let Item::DerivedString(_, s) = a {
        a = Item::String(s);
    }
    if let Item::DerivedString(_, s) = b {
        b = Item::String(s);
    }
    if let Item::AnyUri(s) = a {
        a = Item::String(s);
    }
    if let Item::AnyUri(s) = b {
        b = Item::String(s);
    }
    if matches!(a, Item::QName(_)) || matches!(b, Item::QName(_)) {
        return match (&a, &b) {
            (Item::QName(a), Item::QName(b)) if op == "=" => Ok(a.equal(b)),
            (Item::QName(a), Item::QName(b)) if op == "!=" => Ok(!a.equal(b)),
            _ => Err(err(
                "XPTY0004",
                "QName values only support equality with other QNames",
            )),
        };
    }
    let cmp = if backwards {
        if ["=", "!="].contains(&op)
            && (matches!(a, Item::Boolean(_)) || matches!(b, Item::Boolean(_)))
        {
            boolean(std::slice::from_ref(&a))?.partial_cmp(&boolean(std::slice::from_ref(&b))?)
        } else if !["=", "!="].contains(&op)
            || matches!(a, Item::Number(_))
            || matches!(b, Item::Number(_))
        {
            a.number().partial_cmp(&b.number())
        } else {
            Some(a.string().cmp(&b.string()))
        }
    } else {
        fn convert<'a>(a: Item<'a>, other: &Item<'_>) -> Result<Item<'a>, Diagnostic> {
            let Item::Untyped(s) = a else {
                return Ok(a);
            };
            Ok(match other {
                Item::Number(_) => Item::Number(Number::cast("double", &s)?),
                Item::Binary(kind, _) => Item::Binary(*kind, kind.decode(&s)?),
                Item::Temporal(t) => Item::Temporal(crate::temporal::Temporal::parse(t.kind, &s)?),
                Item::Boolean(_) => Item::Boolean(match s.trim_matches([' ', '\t', '\r', '\n']) {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => return Err(err("FORG0001", "invalid untyped boolean")),
                }),
                _ => Item::String(s),
            })
        }
        a = convert(a, &b)?;
        b = convert(b, &a)?;
        match (&a, &b) {
            (Item::Number(a), Item::Number(b)) => a.compare(b),
            (Item::String(a), Item::String(b)) => Some(a.cmp(b)),
            (Item::Boolean(a), Item::Boolean(b)) => Some(a.cmp(b)),
            (Item::Binary(ak, a), Item::Binary(bk, b)) if ak == bk => Some(a.cmp(b)),
            (Item::Temporal(a), Item::Temporal(b)) => {
                let ordered = !matches!(op, "=" | "!=");
                if ordered && (!a.kind.is_ordered() || (a.kind.is_duration() && a.kind != b.kind)) {
                    return Err(err(
                        "XPTY0004",
                        "these date, time or duration values are not ordered",
                    ));
                }
                Some(
                    a.compare(b).ok_or_else(|| {
                        err("XPTY0004", "incomparable date, time or duration types")
                    })?,
                )
            }
            _ => return Err(err("XPTY0004", "incomparable atomic types")),
        }
    };
    Ok(match op {
        "=" => cmp == Some(Ordering::Equal),
        "!=" => cmp != Some(Ordering::Equal),
        "<" => cmp == Some(Ordering::Less),
        ">" => cmp == Some(Ordering::Greater),
        "<=" => matches!(cmp, Some(Ordering::Less | Ordering::Equal)),
        ">=" => matches!(cmp, Some(Ordering::Greater | Ordering::Equal)),
        _ => false,
    })
}
fn optional_qname(value: &[Item<'_>]) -> Result<Option<crate::qname::AtomicQName>, Diagnostic> {
    match value {
        [] => Ok(None),
        [item] => match atomize(item) {
            Item::QName(name) => Ok(Some(name)),
            Item::Untyped(_) => Err(err(
                "XPTY0117",
                "untyped values cannot be converted to namespace-sensitive QName arguments",
            )),
            _ => Err(err("XPTY0004", "argument must be an optional QName")),
        },
        _ => Err(err("XPTY0004", "argument must be an optional QName")),
    }
}
fn function<'a>(
    name: &str,
    args: &[Value<'a>],
    focus: &Focus<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    if name == "error" {
        let code = optional_qname(args.first().map(Vec::as_slice).unwrap_or(&[]))?
            .map_or_else(|| "FOER0000".into(), |name| name.error_code());
        let description = if let Some(description) = args.get(1) {
            let [item] = description.as_slice() else {
                return Err(err(
                    "XPTY0004",
                    "error description must be a singleton string",
                ));
            };
            match atomize(item) {
                Item::String(s)
                | Item::Untyped(s)
                | Item::AnyUri(s)
                | Item::DerivedString(_, s) => s,
                _ => return Err(err("XPTY0004", "error description must be a string")),
            }
        } else {
            "error() was called".into()
        };
        return Err(err(&code, description));
    }
    if args.is_empty()
        && matches!(focus.item, Item::Absent)
        && ![
            "true",
            "false",
            "position",
            "last",
            "default-collation",
            "current-dateTime",
            "current-date",
            "current-time",
            "implicit-timezone",
            "default-language",
            "static-base-uri",
            "available-environment-variables",
            "math:pi",
        ]
        .contains(&name)
    {
        return Err(err("XPDY0002", "context item is absent"));
    }
    if matches!(focus.item, Item::Absent) && ["position", "last"].contains(&name) {
        return Err(err("XPDY0002", "context focus is absent"));
    }
    let context = vec![focus.item.clone()];
    let first = args.first().unwrap_or(&context);
    let scalar = |v: &[Item<'_>]| -> Result<String, Diagnostic> {
        if v.len() > 1 && !backwards {
            return Err(err("XPTY0004", "function argument must be a singleton"));
        }
        Ok(v.first()
            .map_or_else(String::new, |item| item.string_compat(backwards)))
    };
    let s = || scalar(first);
    if matches!(
        name,
        "namespace-uri-from-QName" | "prefix-from-QName" | "local-name-from-QName"
    ) {
        let Some(qname) = optional_qname(first)? else {
            return Ok(vec![]);
        };
        return Ok(match name {
            "namespace-uri-from-QName" => vec![Item::AnyUri(qname.uri.unwrap_or_default())],
            "prefix-from-QName" => qname.prefix.map_or_else(Vec::new, |prefix| {
                vec![Item::DerivedString(
                    crate::string_type::StringType::NcName,
                    prefix,
                )]
            }),
            _ => vec![Item::DerivedString(
                crate::string_type::StringType::NcName,
                qname.local,
            )],
        });
    }
    if matches!(name, "matches" | "replace" | "tokenize") {
        return crate::regex_functions::evaluate(name, args, budget);
    }
    if name == "generate-id" {
        return match first.as_slice() {
            [] => Ok(vec![Item::String(String::new())]),
            [Item::Node(node)] => {
                let (document, index, attribute, local, uri) = node.order();
                let mut id = format!("d{document:x}n{index}");
                if attribute {
                    use std::fmt::Write;
                    id.push('a');
                    for byte in local.bytes() {
                        budget.tick()?;
                        write!(id, "{byte:02x}").unwrap();
                    }
                    id.push('u');
                    for byte in uri.unwrap_or("").bytes() {
                        budget.tick()?;
                        write!(id, "{byte:02x}").unwrap();
                    }
                }
                Ok(vec![Item::String(id)])
            }
            _ => Err(err("XPTY0004", "generate-id requires zero or one node")),
        };
    }
    if name == "lang" {
        let language = crate::string_functions::string(first, true, false)?.to_ascii_lowercase();
        let target = args.get(1).unwrap_or(&context);
        if args.len() == 1 && matches!(focus.item, Item::Absent) {
            return Err(err("XPDY0002", "lang requires a context node"));
        }
        let [Item::Node(node)] = target.as_slice() else {
            return Err(err("XPTY0004", "lang requires a singleton node"));
        };
        let mut current = Some(*node);
        while let Some(node) = current {
            budget.tick()?;
            if node.is_element()
                && let Some(value) = node
                    .raw
                    .attribute(("http://www.w3.org/XML/1998/namespace", "lang"))
            {
                let value = value.to_ascii_lowercase();
                return Ok(vec![Item::Boolean(
                    value == language
                        || value
                            .strip_prefix(&language)
                            .is_some_and(|suffix| suffix.starts_with('-')),
                )]);
            }
            current = node.parent();
        }
        return Ok(vec![Item::Boolean(false)]);
    }
    if let Some(now) = match name {
        "current-dateTime" => Some(budget.now.clone()),
        "current-date" => budget.now.cast(crate::temporal::TemporalType::Date).ok(),
        "current-time" => budget.now.cast(crate::temporal::TemporalType::Time).ok(),
        "implicit-timezone" => Some(crate::temporal::Temporal::implicit_timezone()),
        _ => None,
    } {
        return Ok(vec![Item::Temporal(now)]);
    }
    if name == "dateTime" {
        let one = |v: &[Item<'a>], kind| -> Result<Option<crate::temporal::Temporal>, Diagnostic> {
            match v {
                [] => Ok(None),
                [item] => match atomize(item) {
                    Item::Temporal(t) if t.kind == kind => Ok(Some(t)),
                    Item::Untyped(s) => Ok(Some(crate::temporal::Temporal::parse(kind, &s)?)),
                    _ => Err(err("XPTY0004", "dateTime requires a date and a time")),
                },
                _ => Err(err("XPTY0004", "dateTime requires singleton arguments")),
            }
        };
        let date = one(&args[0], crate::temporal::TemporalType::Date)?;
        let time = one(&args[1], crate::temporal::TemporalType::Time)?;
        return Ok(match (date, time) {
            (Some(date), Some(time)) => {
                vec![Item::Temporal(crate::temporal::Temporal::combine(
                    &date, &time,
                )?)]
            }
            _ => vec![],
        });
    }
    if let Some(kind) = match name {
        "adjust-dateTime-to-timezone" => Some(crate::temporal::TemporalType::DateTime),
        "adjust-date-to-timezone" => Some(crate::temporal::TemporalType::Date),
        "adjust-time-to-timezone" => Some(crate::temporal::TemporalType::Time),
        _ => None,
    } {
        let value = match first.as_slice() {
            [] => return Ok(vec![]),
            [item] => match atomize(item) {
                Item::Temporal(t) if t.kind == kind => t,
                Item::Untyped(s) => crate::temporal::Temporal::parse(kind, &s)?,
                _ => {
                    return Err(err(
                        "XPTY0004",
                        "timezone adjustment requires a matching value",
                    ));
                }
            },
            _ => return Err(err("XPTY0004", "timezone adjustment requires a singleton")),
        };
        let zone = match args.get(1).map(Vec::as_slice) {
            None => Some(0),
            Some([]) => None,
            Some([item]) => match atomize(item) {
                Item::Temporal(t) if t.kind == crate::temporal::TemporalType::DayTimeDuration => {
                    Some(t.as_timezone()?)
                }
                _ => return Err(err("XPTY0004", "timezone must be an xs:dayTimeDuration")),
            },
            Some(_) => return Err(err("XPTY0004", "timezone must be a singleton")),
        };
        return Ok(vec![Item::Temporal(value.adjust(zone)?)]);
    }
    if let Some((kind, component)) = temporal_accessor(name) {
        let value = match first.as_slice() {
            [] => return Ok(vec![]),
            [item] => match atomize(item) {
                Item::Temporal(t) => t,
                Item::Untyped(s) => crate::temporal::Temporal::parse(kind, &s)?,
                _ => return Err(err("XPTY0004", "accessor requires a temporal value")),
            },
            _ => return Err(err("XPTY0004", "accessor requires an optional singleton")),
        };
        if !kind.accepts(value.kind) {
            return Err(err(
                "XPTY0004",
                "accessor argument has the wrong temporal type",
            ));
        }
        return Ok(match component {
            "timezone" => value.timezone().map(Item::Temporal).into_iter().collect(),
            "seconds" => vec![Item::Number(Number::cast("decimal", &value.seconds())?)],
            _ => {
                let n = value.component(component).expect("known component");
                vec![Item::Number(Number::Integer(n.into()))]
            }
        });
    }
    if name == "path" {
        let node = match first.as_slice() {
            [] => return Ok(vec![]),
            [Item::Node(node)] => *node,
            _ => return Err(err("XPTY0004", "path requires an optional node")),
        };
        let mut steps = vec![];
        let mut current = node;
        while let Some(parent) = current.parent() {
            budget.tick()?;
            let (uri, local) = current.name();
            let qualified = match uri {
                Some(uri) if !uri.is_empty() => format!("Q{{{uri}}}{local}"),
                _ if current.is_element() => format!("Q{{}}{local}"),
                _ => local.to_owned(),
            };
            let same = |n: &XNode<'_>| {
                if current.is_element() {
                    n.is_element() && n.name() == current.name()
                } else if current.is_text() {
                    n.is_text()
                } else if current.is_comment() {
                    n.is_comment()
                } else {
                    n.is_pi() && n.name() == current.name()
                }
            };
            steps.push(if current.is_attribute() {
                format!("@{qualified}")
            } else {
                let position = parent
                    .children()
                    .take_while(|n| *n != current)
                    .filter(same)
                    .count()
                    + 1;
                let test = if current.is_element() {
                    qualified
                } else if current.is_text() {
                    "text()".into()
                } else if current.is_comment() {
                    "comment()".into()
                } else {
                    format!("processing-instruction({local})")
                };
                format!("{test}[{position}]")
            });
            current = parent;
        }
        let mut path = if current.is_document() {
            String::new()
        } else {
            "Q{http://www.w3.org/2005/xpath-functions}root()".into()
        };
        if steps.is_empty() && current.is_document() {
            path.push('/');
        }
        for step in steps.iter().rev() {
            path.push('/');
            path.push_str(step);
        }
        return Ok(vec![Item::String(path)]);
    }
    if let Some(kind) = match name {
        "format-dateTime" => Some(crate::temporal::TemporalType::DateTime),
        "format-date" => Some(crate::temporal::TemporalType::Date),
        "format-time" => Some(crate::temporal::TemporalType::Time),
        _ => None,
    } {
        if args.len() != 2 && args.len() != 5 {
            return Err(err(
                "XPST0017",
                "wrong number of arguments to the date formatting function",
            ));
        }
        let value = match args[0].as_slice() {
            [] => return Ok(vec![]),
            [item] => match atomize(item) {
                Item::Temporal(t) if t.kind == kind => t,
                Item::Untyped(s) => crate::temporal::Temporal::parse(kind, &s)?,
                _ => {
                    return Err(err(
                        "XPTY0004",
                        "date formatting requires the matching temporal type",
                    ));
                }
            },
            _ => {
                return Err(err(
                    "XPTY0004",
                    "date formatting requires an optional singleton",
                ));
            }
        };
        let picture = crate::string_functions::string(&args[1], false, backwards)?;
        return Ok(vec![Item::String(crate::format_datetime::format(
            &value, &picture,
        )?)]);
    }
    if matches!(
        name,
        "unparsed-text" | "unparsed-text-available" | "unparsed-text-lines" | "json-doc"
    ) {
        let href = match args[0].as_slice() {
            [] => return Ok(vec![]),
            _ => crate::string_functions::string(&args[0], false, backwards)?,
        };
        let load = || -> Result<String, Diagnostic> {
            let resolver = budget
                .resolver
                .clone()
                .ok_or_else(|| err("FOUT1170", "no resource resolver is configured"))?;
            (resolver.0)(&href)
        };
        if name == "unparsed-text-available" {
            return Ok(vec![Item::Boolean(load().is_ok())]);
        }
        let text = load().map_err(|mut e| {
            e.code = "FOUT1170".into();
            e
        })?;
        budget.charge(text.len())?;
        return match name {
            "unparsed-text" => Ok(vec![Item::String(text)]),
            "unparsed-text-lines" => Ok(text
                .lines()
                .map(|line| Item::String(line.to_owned()))
                .collect()),
            _ => {
                let options = crate::json::Options {
                    duplicates: crate::json::Duplicates::UseFirst,
                    liberal: false,
                    escape: false,
                };
                crate::json::parse(&text, &options)
            }
        };
    }
    match name {
        "default-language" => return Ok(vec![Item::String("en".into())]),
        "static-base-uri" => return Ok(vec![]),
        "available-environment-variables" => {
            return Ok(std::env::vars().map(|(k, _)| Item::String(k)).collect());
        }
        "environment-variable" => {
            let var = crate::string_functions::string(&args[0], false, backwards)?;
            return Ok(std::env::var(&var)
                .ok()
                .map(Item::String)
                .into_iter()
                .collect());
        }
        "base-uri" | "document-uri" => {
            // Source documents carry no URI in this API.
            if args.len() == 1 && args[0].len() > 1 {
                return Err(err("XPTY0004", "expected an optional node"));
            }
            return Ok(vec![]);
        }
        "collation-key" => {
            let text = crate::string_functions::string(&args[0], false, backwards)?;
            let collation = match args.get(1) {
                Some(uri) => crate::collation::Collation::parse(&crate::string_functions::string(
                    uri, false, backwards,
                )?)?,
                None => crate::collation::Collation::Codepoint,
            };
            budget.charge(text.len())?;
            return Ok(vec![Item::Binary(
                crate::binary::BinaryType::Base64,
                collation.sort_key(&text),
            )]);
        }
        _ => {}
    }
    if matches!(
        name,
        "system-property" | "element-available" | "function-available" | "type-available"
    ) {
        let text = crate::string_functions::string(&args[0], false, backwards)?;
        let text = text.trim();
        let local = text
            .strip_prefix("Q{http://www.w3.org/1999/XSL/Transform}")
            .or_else(|| text.rsplit_once(':').map(|(_, local)| local))
            .unwrap_or(text);
        let is_xsl =
            text.starts_with("Q{http://www.w3.org/1999/XSL/Transform}") || text.starts_with("xsl:");
        return Ok(vec![match name {
            "system-property" => Item::String(
                match (is_xsl, local) {
                    (true, "version") => "3.0",
                    (true, "vendor") => "rust-xslt",
                    (true, "vendor-url") => "https://github.com/vivainio/rustic-xslt",
                    (true, "product-name") => "rust-xslt",
                    (true, "product-version") => env!("CARGO_PKG_VERSION"),
                    (
                        true,
                        "is-schema-aware" | "supports-namespace-axis" | "supports-streaming",
                    ) => "no",
                    (
                        true,
                        "supports-backwards-compatibility"
                        | "supports-serialization"
                        | "supports-dynamic-evaluation",
                    ) => "yes",
                    (true, "xpath-version") => "3.1",
                    (true, "xsd-version") => "1.0",
                    _ => "",
                }
                .to_owned(),
            ),
            "element-available" => Item::Boolean(
                is_xsl
                    && matches!(
                        local,
                        "analyze-string"
                            | "apply-imports"
                            | "apply-templates"
                            | "attribute"
                            | "attribute-set"
                            | "break"
                            | "call-template"
                            | "catch"
                            | "choose"
                            | "comment"
                            | "copy"
                            | "copy-of"
                            | "decimal-format"
                            | "element"
                            | "evaluate"
                            | "for-each"
                            | "for-each-group"
                            | "function"
                            | "if"
                            | "import"
                            | "include"
                            | "iterate"
                            | "key"
                            | "map"
                            | "map-entry"
                            | "message"
                            | "mode"
                            | "namespace"
                            | "next-iteration"
                            | "next-match"
                            | "number"
                            | "otherwise"
                            | "output"
                            | "param"
                            | "perform-sort"
                            | "processing-instruction"
                            | "result-document"
                            | "sequence"
                            | "sort"
                            | "strip-space"
                            | "preserve-space"
                            | "template"
                            | "text"
                            | "try"
                            | "value-of"
                            | "variable"
                            | "when"
                            | "where-populated"
                            | "with-param"
                    ),
            ),
            "function-available" => {
                let arity = match args.get(1) {
                    None => None,
                    Some(value) => {
                        let number = match value.as_slice() {
                            [item] => match atomize(item) {
                                Item::Number(Number::Integer(n) | Number::DerivedInteger(_, n)) => {
                                    n
                                }
                                Item::Untyped(s) => match Number::cast("integer", &s)? {
                                    Number::Integer(n) => n,
                                    _ => unreachable!(),
                                },
                                _ => {
                                    return Err(err(
                                        "XPTY0004",
                                        "function-available arity must be an integer",
                                    ));
                                }
                            },
                            _ => {
                                return Err(err(
                                    "XPTY0004",
                                    "function-available arity must be one integer",
                                ));
                            }
                        };
                        match number.to_usize() {
                            Some(n) => Some(n),
                            None => return Ok(vec![Item::Boolean(false)]),
                        }
                    }
                };
                let (uri, local) = crate::qname::parts(text);
                let internal = match uri {
                    Some("http://www.w3.org/2005/xpath-functions") => Some(local.to_owned()),
                    Some("http://www.w3.org/2001/XMLSchema") => Some(format!("xs:{local}")),
                    Some("http://www.w3.org/2005/xpath-functions/math") => {
                        Some(format!("math:{local}"))
                    }
                    Some("http://www.w3.org/2005/xpath-functions/map") => {
                        Some(format!("map:{local}"))
                    }
                    Some("http://www.w3.org/2005/xpath-functions/array") => {
                        Some(format!("array:{local}"))
                    }
                    _ => None,
                };
                let builtin = internal
                    .as_deref()
                    .and_then(function_arity)
                    .is_some_and(|(lo, hi)| arity.is_none_or(|a| (lo..=hi).contains(&a)));
                let user = budget
                    .functions
                    .keys()
                    .any(|(name, a)| name == text && arity.is_none_or(|n| n == *a));
                Item::Boolean(builtin || user)
            }
            _ => Item::Boolean(
                text.strip_prefix("Q{http://www.w3.org/2001/XMLSchema}")
                    .is_some_and(|l| {
                        function_arity(&format!("xs:{l}")).is_some() && l != "QName"
                            || matches!(
                                l,
                                "QName"
                                    | "NOTATION"
                                    | "anyAtomicType"
                                    | "anySimpleType"
                                    | "anyType"
                                    | "untyped"
                                    | "untypedAtomic"
                                    | "numeric"
                            )
                            || crate::temporal::TemporalType::parse(l).is_some()
                    }),
            ),
        }]);
    }
    if name == "format-integer" {
        let picture = crate::string_functions::string(&args[1], false, backwards)?;
        if let Some(language) = args.get(2) {
            crate::string_functions::string(language, true, backwards)?;
        }
        let value = match args[0].as_slice() {
            [] => return Ok(vec![Item::String(String::new())]),
            [item] => match atomize(item) {
                Item::Number(Number::Integer(n) | Number::DerivedInteger(_, n)) => n,
                Item::Untyped(s) => match Number::cast("integer", &s)? {
                    Number::Integer(n) => n,
                    _ => unreachable!(),
                },
                _ => return Err(err("XPTY0004", "format-integer requires an integer")),
            },
            _ => {
                return Err(err(
                    "XPTY0004",
                    "format-integer requires an optional integer",
                ));
            }
        };
        return crate::numbering::format_integer(&value, &picture)
            .map(|text| vec![Item::String(text)])
            .map_err(|code| err(&code, "invalid integer picture"));
    }
    if name == "parse-json" {
        let text = match args[0].as_slice() {
            [] => return Ok(vec![]),
            _ => crate::string_functions::string(&args[0], false, backwards)?,
        };
        for _ in text.bytes() {
            budget.tick()?;
        }
        let mut options = crate::json::Options {
            duplicates: crate::json::Duplicates::UseFirst,
            liberal: false,
            escape: false,
        };
        if let Some(supplied) = args.get(1) {
            let map = crate::maps::entries(supplied)?;
            if let Some(v) = map.get(&Item::String("liberal".into())) {
                options.liberal = match v.as_slice() {
                    [Item::Boolean(b)] => *b,
                    _ => return Err(err("XPTY0004", "the liberal option must be a boolean")),
                };
            }
            if let Some(v) = map.get(&Item::String("escape".into())) {
                options.escape = match v.as_slice() {
                    [Item::Boolean(b)] => *b,
                    _ => return Err(err("XPTY0004", "the escape option must be a boolean")),
                };
            }
            if let Some(v) = map.get(&Item::String("duplicates".into())) {
                options.duplicates = match v.as_slice() {
                    [Item::String(s)] if s == "use-first" => crate::json::Duplicates::UseFirst,
                    [Item::String(s)] if s == "use-last" => crate::json::Duplicates::UseLast,
                    [Item::String(s)] if s == "reject" => crate::json::Duplicates::Reject,
                    _ => return Err(err("FOJS0005", "invalid duplicates option")),
                };
            }
        }
        return crate::json::parse(&text, &options);
    }
    if name == "format-number" {
        let format = match args.get(2).map(Vec::as_slice) {
            None | Some([]) => budget.decimal_formats.get("").cloned().unwrap_or_default(),
            Some(_) => {
                let name = crate::string_functions::string(&args[2], false, backwards)?;
                budget
                    .decimal_formats
                    .get(name.trim())
                    .cloned()
                    .ok_or_else(|| err("FODF1280", format!("unknown decimal format: {name}")))?
            }
        };
        let number = match args[0].as_slice() {
            [] => Number::Double(f64::NAN),
            [item] => match atomize(item) {
                Item::Number(n) => n,
                Item::Untyped(s) => Number::cast("double", &s)?,
                _ => return Err(err("XPTY0004", "format-number requires a numeric value")),
            },
            _ => return Err(err("XPTY0004", "format-number requires an optional number")),
        };
        let picture = crate::string_functions::string(&args[1], false, backwards)?;
        for _ in picture.bytes() {
            budget.tick()?;
        }
        return Ok(vec![Item::String(crate::format_number::format(
            &number, &picture, &format,
        )?)]);
    }
    if let Some(op) = name.strip_prefix("math:") {
        if op == "pi" {
            return Ok(vec![Item::Number(Number::Double(std::f64::consts::PI))]);
        }
        let mut operands = [0.0f64; 2];
        for (slot, value) in operands.iter_mut().zip(args) {
            *slot = match value.as_slice() {
                [] => return Ok(vec![]),
                [item] => match atomize(item) {
                    Item::Number(n) => n.double(),
                    Item::Untyped(s) => Number::cast("double", &s)?.double(),
                    _ => return Err(err("XPTY0004", "math functions require numeric arguments")),
                },
                _ => {
                    return Err(err(
                        "XPTY0004",
                        "math functions require singleton arguments",
                    ));
                }
            };
        }
        let [x, y] = operands;
        let result = match op {
            "exp" => x.exp(),
            "exp10" => 10f64.powf(x),
            "log" => x.ln(),
            "log10" => x.log10(),
            "sqrt" => x.sqrt(),
            "sin" => x.sin(),
            "cos" => x.cos(),
            "tan" => x.tan(),
            "asin" => x.asin(),
            "acos" => x.acos(),
            "atan" => x.atan(),
            "pow" => x.powf(y),
            "atan2" => x.atan2(y),
            _ => unreachable!("{op}"),
        };
        return Ok(vec![Item::Number(Number::Double(result))]);
    }
    if crate::maps::arity(name).is_some() && !crate::maps::higher_order(name) {
        return crate::maps::evaluate(name, args, budget);
    }
    if name == "sort" {
        return crate::aggregate::sort(args, None, budget);
    }
    if name == "resolve-uri" {
        let text = |v: &[Item<'a>], optional: bool| -> Result<Option<String>, Diagnostic> {
            match v {
                [] if optional => Ok(None),
                [item] => match atomize(item) {
                    Item::String(s)
                    | Item::Untyped(s)
                    | Item::AnyUri(s)
                    | Item::DerivedString(_, s) => Ok(Some(s)),
                    _ => Err(err("XPTY0004", "resolve-uri requires string arguments")),
                },
                _ => Err(err("XPTY0004", "resolve-uri requires singleton arguments")),
            }
        };
        let Some(relative) = text(&args[0], true)? else {
            return Ok(vec![]);
        };
        let base = match args.get(1) {
            Some(base) => text(base, false)?,
            None => None,
        };
        for _ in relative.bytes().chain(base.iter().flat_map(|b| b.bytes())) {
            budget.tick()?;
        }
        return Ok(vec![Item::AnyUri(crate::uri_resolution::resolve(
            relative.trim_matches([' ', '\t', '\r', '\n']),
            base.as_deref()
                .map(|b| b.trim_matches([' ', '\t', '\r', '\n'])),
        )?)]);
    }
    if name == "default-collation" {
        return Ok(vec![Item::AnyUri(
            "http://www.w3.org/2005/xpath-functions/collation/codepoint".into(),
        )]);
    }
    if name == "nilled" {
        return match first.as_slice() {
            [] => Ok(vec![]),
            [Item::Node(node)] if node.is_element() => Ok(vec![Item::Boolean(false)]),
            [Item::Node(_)] => Ok(vec![]),
            _ => Err(err(
                "XPTY0004",
                "nilled requires an optional singleton node",
            )),
        };
    }
    if name == "trace" {
        if let Some(label) = args.get(1) {
            let [item] = label.as_slice() else {
                return Err(err("XPTY0004", "trace label must be a singleton string"));
            };
            if !matches!(
                atomize(item),
                Item::String(_) | Item::Untyped(_) | Item::AnyUri(_)
            ) {
                return Err(err("XPTY0004", "trace label must be a string"));
            }
        }
        let mut result = Vec::with_capacity(first.len());
        for item in first {
            budget.tick()?;
            result.push(item.clone());
        }
        return Ok(result);
    }
    if name == "unordered" {
        for _ in first {
            budget.tick()?;
        }
        return Ok(first.clone());
    }
    if name == "has-children" {
        return match first.as_slice() {
            [] => Ok(vec![Item::Boolean(false)]),
            [Item::Node(node)] => Ok(vec![Item::Boolean(node.children().next().is_some())]),
            _ => Err(err("XPTY0004", "has-children requires zero or one node")),
        };
    }
    if matches!(name, "innermost" | "outermost") {
        let mut nodes = vec![];
        for item in first {
            budget.tick()?;
            let Item::Node(node) = item else {
                return Err(err(
                    "XPTY0004",
                    "node selection requires a sequence of nodes",
                ));
            };
            nodes.push(*node);
        }
        let selected = nodes
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let mut ancestors = std::collections::HashSet::new();
        let mut result = vec![];
        for node in &nodes {
            let mut parent = node.parent();
            let mut contained = false;
            while let Some(ancestor) = parent {
                budget.tick()?;
                if name == "innermost" {
                    ancestors.insert(ancestor);
                }
                if selected.contains(&ancestor) {
                    contained = true;
                }
                parent = ancestor.parent();
            }
            if name == "outermost" && !contained {
                result.push(*node);
            }
        }
        if name == "innermost" {
            result.extend(nodes.into_iter().filter(|node| !ancestors.contains(node)));
        }
        return Ok(ordered(result));
    }
    if name == "node-name" {
        return match first.as_slice() {
            [] => Ok(vec![]),
            [Item::Node(node)] => Ok(node
                .atomic_name()
                .map_or_else(Vec::new, |name| vec![Item::QName(name)])),
            _ => Err(err("XPTY0004", "node-name requires zero or one node")),
        };
    }
    if name == "namespace-uri-for-prefix" {
        let prefix = crate::string_functions::string(&args[0], true, false)?;
        let [Item::Node(n)] = args[1].as_slice() else {
            return Err(err(
                "XPTY0004",
                "namespace-uri-for-prefix requires an element node",
            ));
        };
        if !n.is_element() {
            return Err(err(
                "XPTY0004",
                "namespace-uri-for-prefix requires an element node",
            ));
        }
        let uri = if prefix == "xml" {
            Some("http://www.w3.org/XML/1998/namespace")
        } else {
            n.raw.lookup_namespace_uri(if prefix.is_empty() {
                None
            } else {
                Some(&prefix)
            })
        };
        return Ok(uri
            .filter(|u| !u.is_empty())
            .map(|u| vec![Item::AnyUri(u.to_owned())])
            .unwrap_or_default());
    }
    if name == "in-scope-prefixes" {
        let [Item::Node(n)] = first.as_slice() else {
            return Err(err(
                "XPTY0004",
                "in-scope-prefixes requires an element node",
            ));
        };
        if !n.is_element() {
            return Err(err(
                "XPTY0004",
                "in-scope-prefixes requires an element node",
            ));
        }
        let mut prefixes = n
            .raw
            .namespaces()
            .filter(|ns| !ns.uri().is_empty())
            .map(|ns| ns.name().unwrap_or("").to_owned())
            .collect::<Vec<_>>();
        prefixes.push("xml".into());
        prefixes.sort();
        prefixes.dedup();
        return Ok(prefixes.into_iter().map(Item::String).collect());
    }
    if name == "resolve-QName" {
        let lexical = crate::string_functions::string(&args[0], true, false)?;
        let [Item::Node(n)] = args[1].as_slice() else {
            return Err(err("XPTY0004", "resolve-QName requires an element node"));
        };
        if !n.is_element() {
            return Err(err("XPTY0004", "resolve-QName requires an element node"));
        }
        if args[0].is_empty() {
            return Ok(vec![]);
        }
        let trimmed = lexical.trim_matches([' ', '\t', '\n', '\r']);
        let _ = crate::qname::AtomicQName::new("urn:validation", trimmed)?;
        let (prefix, _) = trimmed
            .split_once(':')
            .map_or((None, trimmed), |(p, l)| (Some(p), l));
        let binding = if prefix == Some("xml") {
            Some("http://www.w3.org/XML/1998/namespace")
        } else {
            n.raw.lookup_namespace_uri(prefix)
        };
        let uri = match binding {
            Some(uri) => uri,
            None if prefix.is_some() => {
                return Err(err("FONS0004", "resolve-QName prefix is not bound"));
            }
            None => "",
        };
        return Ok(vec![Item::QName(crate::qname::AtomicQName::new(
            uri, trimmed,
        )?)]);
    }
    if name == "QName" {
        let uri = crate::string_functions::string(&args[0], true, false)?;
        let lexical = crate::string_functions::string(&args[1], false, false)?;
        for _ in uri.bytes().chain(lexical.bytes()) {
            budget.tick()?;
        }
        return Ok(vec![Item::QName(crate::qname::AtomicQName::new(
            uri.trim_matches([' ', '\t', '\n', '\r']),
            lexical.trim_matches([' ', '\t', '\n', '\r']),
        )?)]);
    }
    if matches!(name, "round" | "round-half-to-even") {
        let precision = match args.get(1).map(Vec::as_slice) {
            None => num_bigint::BigInt::from(0),
            Some([item]) => match atomize(item) {
                Item::Number(Number::Integer(n) | Number::DerivedInteger(_, n)) => n,
                Item::Untyped(s) => {
                    let Number::Integer(n) = Number::cast("integer", &s)? else {
                        unreachable!()
                    };
                    n
                }
                _ => return Err(err("XPTY0004", "rounding precision must be an integer")),
            },
            _ => {
                return Err(err(
                    "XPTY0004",
                    "rounding precision must be a singleton integer",
                ));
            }
        };
        if first.is_empty() && !backwards {
            return Ok(vec![]);
        }
        return Ok(vec![Item::Number(
            numeric(first, backwards)?.round(&precision, name == "round-half-to-even")?,
        )]);
    }
    if matches!(name, "abs" | "floor" | "ceiling") {
        if first.is_empty() && !backwards {
            return Ok(vec![]);
        }
        return Ok(vec![Item::Number(
            numeric(first, backwards)?.unary_function(name),
        )]);
    }
    if name == "root" {
        return match first.as_slice() {
            [] => Ok(vec![]),
            [Item::Node(n)] => Ok(vec![Item::Node(n.root())]),
            _ => Err(err("XPTY0004", "root requires an optional singleton node")),
        };
    }
    if matches!(name, "exactly-one" | "zero-or-one" | "one-or-more") {
        let invalid = match name {
            "exactly-one" if first.len() != 1 => Some("FORG0005"),
            "zero-or-one" if first.len() > 1 => Some("FORG0003"),
            "one-or-more" if first.is_empty() => Some("FORG0004"),
            _ => None,
        };
        if let Some(code) = invalid {
            return Err(err(
                code,
                format!("{name} argument has invalid cardinality"),
            ));
        }
        let mut result = Vec::with_capacity(first.len());
        for item in first {
            budget.tick()?;
            result.push(item.clone());
        }
        return Ok(result);
    }
    if matches!(
        name,
        "upper-case"
            | "lower-case"
            | "translate"
            | "substring"
            | "codepoints-to-string"
            | "string-to-codepoints"
            | "compare"
            | "codepoint-equal"
            | "normalize-unicode"
            | "encode-for-uri"
            | "iri-to-uri"
            | "escape-html-uri"
            | "contains"
            | "starts-with"
            | "ends-with"
            | "substring-before"
            | "substring-after"
            | "string-join"
            | "contains-token"
    ) {
        return crate::string_functions::evaluate(name, args, budget, backwards);
    }
    if name == "data" {
        return Ok(flatten_arrays(first.clone())?.iter().map(atomize).collect());
    }
    if let Some(kind) = name.strip_prefix("xs:") {
        if first.is_empty() {
            return Ok(vec![]);
        }
        if first.len() != 1 {
            return Err(err("XPTY0004", "constructor requires a singleton"));
        }
        let item = &first[0];
        if let Some(kind) = crate::temporal::TemporalType::parse(kind) {
            return Ok(vec![Item::Temporal(match atomize(item) {
                Item::Temporal(t) => t.cast(kind)?,
                Item::String(text) | Item::Untyped(text) | Item::DerivedString(_, text) => {
                    for _ in text.bytes() {
                        budget.tick()?;
                    }
                    crate::temporal::Temporal::parse(kind, &text)?
                }
                _ => {
                    return Err(err(
                        "XPTY0004",
                        "temporal constructor requires a string or temporal value",
                    ));
                }
            })]);
        }
        if let Some(kind) = crate::binary::BinaryType::parse(kind) {
            let bytes = match atomize(item) {
                Item::Binary(_, bytes) => bytes,
                Item::String(text) | Item::Untyped(text) | Item::DerivedString(_, text) => {
                    for _ in text.bytes() {
                        budget.tick()?;
                    }
                    kind.decode(&text)?
                }
                _ => {
                    return Err(err(
                        "XPTY0004",
                        "binary constructor requires a string or binary value",
                    ));
                }
            };
            return Ok(vec![Item::Binary(kind, bytes)]);
        }
        if matches!(item, Item::Temporal(_))
            && !matches!(kind, "string" | "untypedAtomic")
            && crate::string_type::StringType::parse(kind).is_none()
        {
            return Err(err(
                "XPTY0004",
                "date, time or duration cannot be cast to this type",
            ));
        }
        if matches!(item, Item::Binary(_, _))
            && !matches!(kind, "string" | "untypedAtomic")
            && crate::string_type::StringType::parse(kind).is_none()
        {
            return Err(err("XPTY0004", "binary value cannot be cast to this type"));
        }
        if let Some(kind) = crate::string_type::StringType::parse(kind) {
            let text = item.string();
            for _ in text.bytes() {
                budget.tick()?;
            }
            return Ok(vec![Item::DerivedString(kind, kind.cast(&text)?)]);
        }
        if matches!(item, Item::AnyUri(_)) && !matches!(kind, "string" | "untypedAtomic" | "anyURI")
        {
            return Err(err("XPTY0004", "anyURI cannot be cast to this atomic type"));
        }
        if matches!(item, Item::QName(_)) && !matches!(kind, "string" | "untypedAtomic") {
            return Err(err("XPTY0004", "QName cannot be cast to this atomic type"));
        }
        return Ok(vec![match kind {
            "string" => Item::String(item.string()),
            "untypedAtomic" => Item::Untyped(item.string()),
            "anyURI" => match atomize(item) {
                Item::String(s)
                | Item::Untyped(s)
                | Item::AnyUri(s)
                | Item::DerivedString(_, s) => {
                    for _ in s.bytes() {
                        budget.tick()?;
                    }
                    Item::AnyUri(crate::string_functions::collapse_xml_whitespace(&s))
                }
                _ => {
                    return Err(err(
                        "XPTY0004",
                        "anyURI constructor requires a string, untyped atomic or URI",
                    ));
                }
            },
            "boolean" => Item::Boolean(match item {
                Item::Boolean(b) => *b,
                Item::Number(n) => n.boolean(),
                _ => match item.string().trim_matches([' ', '\t', '\r', '\n']) {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => return Err(err("FORG0001", "invalid boolean lexical value")),
                },
            }),
            _ => Item::Number(match item {
                Item::Number(n) => n.convert(kind)?,
                Item::Boolean(b) => Number::from(usize::from(*b)).convert(kind)?,
                _ => Number::cast(kind, &item.string())?,
            }),
        }]);
    }
    if matches!(name, "distinct-values" | "index-of") {
        return crate::atomic_sequence::evaluate(name, args, budget);
    }
    if name == "deep-equal" {
        return Ok(vec![Item::Boolean(crate::deep_equal::evaluate(
            args, budget,
        )?)]);
    }
    if matches!(name, "sum" | "avg" | "min" | "max") {
        return crate::aggregate::evaluate(name, args, budget);
    }
    if matches!(
        name,
        "reverse" | "head" | "tail" | "remove" | "insert-before" | "subsequence"
    ) {
        fn integer(value: &[Item<'_>]) -> Result<num_bigint::BigInt, Diagnostic> {
            match value {
                [item] => match atomize(item) {
                    Item::Number(Number::Integer(n) | Number::DerivedInteger(_, n)) => Ok(n),
                    Item::Untyped(s) => {
                        let Number::Integer(n) = Number::cast("integer", &s)? else {
                            unreachable!()
                        };
                        Ok(n)
                    }
                    _ => Err(err("XPTY0004", "sequence position requires an integer")),
                },
                _ => Err(err("XPTY0004", "sequence position requires a singleton")),
            }
        }
        fn double(value: &[Item<'_>]) -> Result<f64, Diagnostic> {
            match value {
                [item] => match atomize(item) {
                    Item::Number(n) => Ok(n.double()),
                    Item::Untyped(s) => Ok(Number::cast("double", &s)?.double()),
                    _ => Err(err(
                        "XPTY0004",
                        "subsequence position requires a numeric value",
                    )),
                },
                _ => Err(err("XPTY0004", "subsequence position requires a singleton")),
            }
        }
        fn round(n: f64) -> f64 {
            let floor = n.floor();
            if n - floor < 0.5 { floor } else { n.ceil() }
        }
        let mut result = vec![];
        if name == "reverse" {
            for item in first.iter().rev() {
                budget.tick()?;
                result.push(item.clone());
            }
        } else if name == "head" {
            if let Some(item) = first.first() {
                budget.tick()?;
                result.push(item.clone());
            }
        } else if name == "tail" {
            for item in first.iter().skip(1) {
                budget.tick()?;
                result.push(item.clone());
            }
        } else if name == "remove" {
            let index = integer(&args[1])?.to_usize();
            for (i, item) in first.iter().enumerate() {
                budget.tick()?;
                if Some(i + 1) != index {
                    result.push(item.clone());
                }
            }
        } else if name == "insert-before" {
            let position = integer(&args[1])?;
            let index = if position <= 1.into() {
                0
            } else if position > first.len().into() {
                first.len()
            } else {
                position.to_usize().unwrap() - 1
            };
            for (i, item) in first.iter().enumerate() {
                if i == index {
                    for item in &args[2] {
                        budget.tick()?;
                        result.push(item.clone());
                    }
                }
                budget.tick()?;
                result.push(item.clone());
            }
            if index == first.len() {
                for item in &args[2] {
                    budget.tick()?;
                    result.push(item.clone());
                }
            }
        } else {
            let start = round(double(&args[1])?);
            let end = if let Some(length) = args.get(2) {
                start + round(double(length)?)
            } else {
                f64::INFINITY
            };
            for (i, item) in first.iter().enumerate() {
                budget.tick()?;
                let position = (i + 1) as f64;
                if position >= start && position < end {
                    result.push(item.clone());
                }
            }
        }
        return Ok(result);
    }
    let result = match name {
        "true" => Item::Boolean(true),
        "false" => Item::Boolean(false),
        "position" => Item::Number(Number::from(focus.position)),
        "last" => Item::Number(Number::from(focus.size)),
        "string" => Item::String(s()?),
        "number" => Item::Number(Number::Double(first.first().map_or(f64::NAN, Item::number))),
        "count" => Item::Number(Number::from(first.len())),
        "exists" => Item::Boolean(!first.is_empty()),
        "empty" => Item::Boolean(first.is_empty()),
        "boolean" => Item::Boolean(boolean(first)?),
        "not" => Item::Boolean(!boolean(first)?),
        "normalize-space" => Item::String(
            s()?.split([' ', '\t', '\r', '\n'])
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        ),
        "string-length" => Item::Number(Number::from(s()?.chars().count())),
        "concat" => {
            let mut s = String::new();
            for a in args {
                s.push_str(&scalar(a)?);
            }
            Item::String(s)
        }
        "name" | "local-name" | "namespace-uri" => {
            let text = match first.as_slice() {
                [] => String::new(),
                [Item::Node(n)] => {
                    let (uri, local) = n.name();
                    if name == "namespace-uri" {
                        uri.unwrap_or("").into()
                    } else if name == "local-name" {
                        local.into()
                    } else {
                        n.atomic_name()
                            .map_or_else(|| local.into(), |name| name.lexical())
                    }
                }
                _ => return Err(err("XPTY0004", "name function requires one node")),
            };
            if name == "namespace-uri" {
                Item::AnyUri(text)
            } else {
                Item::String(text)
            }
        }
        _ => unreachable!(),
    };
    Ok(vec![result])
}
