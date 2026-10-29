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
    Sequence(Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    TypeOp(String, Box<Expr>, crate::seqtype::SequenceType),
    QNameCast(Box<Expr>, BTreeMap<String, String>, bool, bool),
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
            Self::AnyUri(_) | Self::QName(_) | Self::Binary(_, _) => f64::NAN,
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
pub(crate) struct Budget {
    remaining: u64,
    pub max_depth: usize,
}
impl Budget {
    pub fn new(limits: Limits) -> Self {
        Self {
            remaining: limits.max_steps,
            max_depth: limits.max_depth,
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
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '_' | '-' | '.'))
            {
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
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '-' | '.'))
            {
                i += 1;
            }
            if chars.get(i) == Some(&':') && !matches!(chars.get(i + 1), Some(':' | '=')) {
                i += 1;
                if chars.get(i) == Some(&'*') {
                    i += 1;
                    out.push(Token::Name(chars[start..i].iter().collect()));
                    continue;
                }
                if !chars.get(i).is_some_and(|c| c.is_alphabetic() || *c == '_') {
                    return Err(syntax("expected local name after prefix"));
                }
                i += 1;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '-' | '.'))
                {
                    i += 1;
                }
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else if c == '*' && chars.get(i + 1) == Some(&':') {
            let start = i;
            i += 2;
            if !chars.get(i).is_some_and(|c| c.is_alphabetic() || *c == '_') {
                return Err(syntax("expected wildcard local name"));
            }
            i += 1;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '-' | '.'))
            {
                i += 1;
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else {
            let pair: String = chars[i..chars.len().min(i + 2)].iter().collect();
            if matches!(
                pair.as_str(),
                "//" | ".." | "!=" | "<=" | ">=" | "::" | "<<" | ">>" | ":=" | "||"
            ) {
                out.push(Token::Symbol(pair));
                i += 2;
            } else if "/.@[](),$+-*=<>|?!".contains(c) {
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
                            Token::Symbol(s) if ["*", ",", "?"].contains(&s.as_str()) => {
                                name.push_str(&s)
                            }
                            _ => return Err(syntax("invalid sequence type")),
                        }
                    }
                }
                for suffix in if matches!(op.as_str(), "cast" | "castable") {
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
        let mut e = if self.eat("-") {
            Expr::Unary(false, Box::new(self.expression(15)?))
        } else if self.eat("+") {
            Expr::Unary(true, Box::new(self.expression(15)?))
        } else if self.eat("(") {
            if self.eat(")") {
                Expr::Sequence(vec![])
            } else {
                let e = self.expression(0)?;
                self.expect(")")?;
                e
            }
        } else if self.eat("$") {
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
                    let mut args = vec![];
                    if !self.eat(")") {
                        loop {
                            args.push(self.expression(2)?);
                            if self.eat(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    let name = self.qname(n.clone())?;
                    let local = match name.uri.as_deref() {
                        None if !n.starts_with("Q{") => name.local,
                        Some("http://www.w3.org/2005/xpath-functions") => name.local,
                        Some("http://www.w3.org/2001/XMLSchema") => format!("xs:{}", name.local),
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
                        Expr::QNameCast(
                            Box::new(args.remove(0)),
                            self.namespaces.clone(),
                            true,
                            false,
                        )
                    } else {
                        Expr::Call(local, args)
                    }
                }
                _ if self.step_start() => Expr::Path(Box::new(Expr::Context), vec![self.step()?]),
                _ => return Err(syntax("expected XPath expression")),
            }
        };
        let mut path_steps = 0;
        loop {
            if path_steps > 128 {
                return Err(syntax("XPath path nesting limit exceeded"));
            }
            if self.eat("[") {
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
                                    let name = match self.peek().clone() {
                                        Token::Name(n) => {
                                            self.next();
                                            Some(self.qname(n)?)
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
                            Test::Name(self.qname(n)?)
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
fn function_arity(name: &str) -> Option<(usize, usize)> {
    Some(match name {
        "xs:string" | "xs:untypedAtomic" | "xs:integer" | "xs:decimal" | "xs:double"
        | "xs:float" | "xs:boolean" | "xs:QName" | "xs:anyURI" | "xs:hexBinary"
        | "xs:base64Binary" => (1, 1),
        "true" | "false" | "position" | "last" | "default-collation" => (0, 0),
        "trace" => (1, 2),
        "nilled" => (1, 1),
        "error" => (0, 3),
        "QName" => (2, 2),
        "namespace-uri-from-QName" | "prefix-from-QName" | "local-name-from-QName" => (1, 1),
        "namespace-uri-for-prefix" | "resolve-QName" => (2, 2),
        "lang" => (1, 2),
        "in-scope-prefixes" | "innermost" | "outermost" | "unordered" => (1, 1),
        "string" | "number" | "normalize-space" | "string-length" | "name" | "local-name"
        | "namespace-uri" | "node-name" | "has-children" | "generate-id" | "root" => (0, 1),
        "count" | "boolean" | "not" | "exists" | "empty" | "data" | "reverse" | "head" | "tail" => {
            (1, 1)
        }
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
            if op == "!" {
                let size = a.len();
                let mut result = vec![];
                for (index, item) in a.into_iter().enumerate() {
                    budget.tick()?;
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
            function(name, &args, focus, budget, backwards)?
        }
    };
    Ok(result)
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
fn compare(a: &Item<'_>, b: &Item<'_>, op: &str, backwards: bool) -> Result<bool, Diagnostic> {
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
        && !["true", "false", "position", "last", "default-collation"].contains(&name)
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
            [Item::Node(n)] => {
                let (uri, local) = n.name();
                if local.is_empty() {
                    return Ok(vec![]);
                }
                let prefix = uri
                    .and_then(|u| {
                        if u == "http://www.w3.org/XML/1998/namespace" {
                            Some("xml")
                        } else {
                            n.raw.lookup_prefix(u)
                        }
                    })
                    .filter(|p| !p.is_empty());
                let lexical = prefix.map_or_else(|| local.to_owned(), |p| format!("{p}:{local}"));
                Ok(vec![Item::QName(crate::qname::AtomicQName::new(
                    uri.unwrap_or(""),
                    &lexical,
                )?)])
            }
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
            &uri, &lexical,
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
    ) {
        return crate::string_functions::evaluate(name, args, budget, backwards);
    }
    if name == "data" {
        return Ok(first.iter().map(atomize).collect());
    }
    if let Some(kind) = name.strip_prefix("xs:") {
        if first.is_empty() {
            return Ok(vec![]);
        }
        if first.len() != 1 {
            return Err(err("XPTY0004", "constructor requires a singleton"));
        }
        let item = &first[0];
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
                        let node = n.raw;
                        if let Some(uri) = uri {
                            node.lookup_prefix(uri)
                                .filter(|p| !p.is_empty())
                                .map_or_else(|| local.into(), |p| format!("{p}:{local}"))
                        } else {
                            local.into()
                        }
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
