//! XPath expression machinery. The AST mirrors the reference engine's separation
//! of literals, binary expressions, function calls and axis steps.
use crate::{Diagnostic, Limits, number::Number};
use roxmltree::Node;
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
    Context,
    Root,
    Path(Box<Expr>, Vec<Step>),
    Filter(Box<Expr>, Vec<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Unary(bool, Box<Expr>),
    Call(String, Vec<Expr>),
    Sequence(Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum XNode<'a> {
    Tree(Node<'a, 'a>),
    Attribute(Node<'a, 'a>, &'a str, Option<&'a str>),
}
impl<'a> XNode<'a> {
    pub fn string(self) -> String {
        match self {
            Self::Tree(n) if n.is_element() || n.is_root() => n
                .descendants()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .collect(),
            Self::Tree(n) => n
                .pi()
                .and_then(|pi| pi.value)
                .or_else(|| n.text())
                .unwrap_or_default()
                .into(),
            Self::Attribute(n, local, uri) => n
                .attributes()
                .find(|a| a.name() == local && a.namespace() == uri)
                .map_or("", |a| a.value())
                .into(),
        }
    }
    pub fn parent(self) -> Option<Self> {
        match self {
            Self::Tree(n) => n.parent().map(Self::Tree),
            Self::Attribute(n, ..) => Some(Self::Tree(n)),
        }
    }
    pub fn root(self) -> Self {
        Self::Tree(match self {
            Self::Tree(n) | Self::Attribute(n, ..) => n.document().root(),
        })
    }
    pub fn name(self) -> (Option<&'a str>, &'a str) {
        match self {
            Self::Attribute(_, n, uri) => (uri, n),
            Self::Tree(n) if n.is_element() => (n.tag_name().namespace(), n.tag_name().name()),
            Self::Tree(n) => (None, n.pi().map_or("", |pi| pi.target)),
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) enum Item<'a> {
    Absent,
    Node(XNode<'a>),
    String(String),
    Untyped(String),
    Number(Number),
    Boolean(bool),
}
pub(crate) type Value<'a> = Vec<Item<'a>>;
pub(crate) type Variables<'a> = BTreeMap<String, Value<'a>>;
impl Item<'_> {
    pub fn string(&self) -> String {
        match self {
            Self::Absent => String::new(),
            Self::Node(n) => n.string(),
            Self::String(s) | Self::Untyped(s) => s.clone(),
            Self::Number(n) => n.string(),
            Self::Boolean(b) => b.to_string(),
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
        [Item::String(s) | Item::Untyped(s)] => Ok(!s.is_empty()),
        [Item::Number(n)] => Ok(n.boolean()),
        _ => Err(err(
            "FORG0006",
            "effective boolean value requires a node sequence or a single atomic value",
        )),
    }
}
pub(crate) fn string(value: &[Item<'_>], separator: &str) -> String {
    value
        .iter()
        .map(Item::string)
        .collect::<Vec<_>>()
        .join(separator)
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
        if c.is_whitespace() {
            i += 1;
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
            let n: String = chars[start..i].iter().collect();
            out.push(Token::Number(Number::literal(&n).map_err(|mut e| {
                e.phase = "compile";
                e
            })?));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '-' | '.'))
            {
                i += 1;
            }
            if chars.get(i) == Some(&':') && chars.get(i + 1) != Some(&':') {
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
            if matches!(pair.as_str(), "//" | ".." | "!=" | "<=" | ">=" | "::") {
                out.push(Token::Symbol(pair));
                i += 2;
            } else if "/.@[](),$+-*=<>|".contains(c) {
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
    let mut p = Parser {
        tokens: tokenize(s)?,
        index: 0,
        namespaces,
        depth: 0,
    };
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
        let mut left = self.primary()?;
        loop {
            let (op, prec) = match self.peek() {
                Token::Symbol(s) if s == "," => (s.clone(), 1),
                Token::Name(s) if s == "or" => (s.clone(), 2),
                Token::Name(s) if s == "and" => (s.clone(), 3),
                Token::Name(s) if ["eq", "ne", "lt", "gt", "le", "ge"].contains(&s.as_str()) => {
                    (s.clone(), 4)
                }
                Token::Symbol(s) if ["=", "!=", "<", ">", "<=", ">="].contains(&s.as_str()) => {
                    (s.clone(), 4)
                }
                Token::Symbol(s) if s == "|" => (s.clone(), 5),
                Token::Symbol(s) if s == "+" || s == "-" => (s.clone(), 6),
                Token::Symbol(s) if s == "*" => (s.clone(), 7),
                Token::Name(s) if s == "div" || s == "mod" => (s.clone(), 7),
                _ => break,
            };
            if prec < minimum {
                break;
            }
            self.next();
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
        let mut e = if self.eat("-") {
            Expr::Unary(false, Box::new(self.expression(8)?))
        } else if self.eat("+") {
            Expr::Unary(true, Box::new(self.expression(8)?))
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
                Token::Name(n) if !n.contains(':') => Expr::Variable(n),
                _ => return Err(syntax("expected unprefixed variable name")),
            }
        } else if self.eat("/") {
            if self.step_start() {
                let step = self.step()?;
                Expr::Path(Box::new(Expr::Root), vec![step])
            } else {
                Expr::Root
            }
        } else if self.eat("//") {
            let step = self.step()?;
            Expr::Path(
                Box::new(Expr::Root),
                vec![
                    Step {
                        axis: Axis::DescendantOrSelf,
                        test: Test::Node,
                        predicates: vec![],
                    },
                    step,
                ],
            )
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
                    let local = if let Some((prefix, local)) = n.split_once(':') {
                        match self.namespaces.get(prefix).map(String::as_str) {
                            Some("http://www.w3.org/2005/xpath-functions") => local.into(),
                            Some("http://www.w3.org/2001/XMLSchema") => format!("xs:{local}"),
                            _ => {
                                return Err(Diagnostic::new(
                                    "XPST0017",
                                    format!("unknown function namespace: {n}"),
                                    "compile",
                                ));
                            }
                        }
                    } else {
                        n.clone()
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
                    Expr::Call(local, args)
                }
                _ if self.step_start() => Expr::Path(Box::new(Expr::Context), vec![self.step()?]),
                _ => return Err(syntax("expected XPath expression")),
            }
        };
        loop {
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
            } else if self.eat("/") {
                let step = self.step()?;
                e = match e {
                    Expr::Path(base, mut steps) => {
                        steps.push(step);
                        Expr::Path(base, steps)
                    }
                    _ => Expr::Path(Box::new(e), vec![step]),
                };
            } else if self.eat("//") {
                let steps = vec![
                    Step {
                        axis: Axis::DescendantOrSelf,
                        test: Test::Node,
                        predicates: vec![],
                    },
                    self.step()?,
                ];
                e = Expr::Path(Box::new(e), steps);
            } else {
                break;
            }
        }
        Ok(e)
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
                                    let target = if let Token::String(s) = self.peek().clone() {
                                        self.next();
                                        Some(s)
                                    } else {
                                        None
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
        | "xs:float" | "xs:boolean" => (1, 1),
        "true" | "false" | "position" | "last" => (0, 0),
        "string" | "number" | "normalize-space" | "string-length" | "name" | "local-name"
        | "namespace-uri" => (0, 1),
        "count" | "boolean" | "not" | "sum" | "exists" | "empty" | "data" => (1, 1),
        "contains" | "starts-with" | "ends-with" | "substring-before" | "substring-after"
        | "string-join" => (2, 2),
        "concat" => (2, usize::MAX),
        _ => return None,
    })
}
pub(crate) fn test_node(test: &Test, n: XNode<'_>, axis: Axis) -> bool {
    match test {
        Test::Node => true,
        Test::Any => match n {
            XNode::Attribute(..) => matches!(axis, Axis::Attribute),
            XNode::Tree(n) => n.is_element(),
        },
        Test::Name(name) => {
            let (uri, local) = n.name();
            let principal = match n {
                XNode::Attribute(..) => matches!(axis, Axis::Attribute),
                XNode::Tree(n) => n.is_element(),
            };
            principal && uri == name.uri.as_deref() && local == name.local
        }
        Test::Text => matches!(n, XNode::Tree(n) if n.is_text()),
        Test::Comment => matches!(n, XNode::Tree(n) if n.is_comment()),
        Test::Pi(target) => {
            matches!(n, XNode::Tree(n) if n.pi().is_some_and(|pi| target.as_ref().is_none_or(|s| s == pi.target)))
        }
        Test::Document => matches!(n, XNode::Tree(n) if n.is_root()),
        Test::Element(name) => {
            matches!(n, XNode::Tree(node) if node.is_element())
                && name
                    .as_ref()
                    .is_none_or(|name| n.name() == (name.uri.as_deref(), name.local.as_str()))
        }
        Test::AttributeKind(name) => {
            matches!(n, XNode::Attribute(..))
                && name
                    .as_ref()
                    .is_none_or(|name| n.name() == (name.uri.as_deref(), name.local.as_str()))
        }
        Test::Namespace(uri) => test_node(&Test::Any, n, axis) && n.name().0 == Some(uri.as_str()),
        Test::Local(local) => test_node(&Test::Any, n, axis) && n.name().1 == local,
    }
}
fn ordered<'a>(nodes: Vec<XNode<'a>>) -> Value<'a> {
    let mut nodes = nodes;
    nodes.sort_by_key(|n| match n {
        XNode::Tree(n) => (n.id().get(), 0, ""),
        XNode::Attribute(n, name, _) => (n.id().get(), 1, *name),
    });
    nodes.dedup();
    nodes.into_iter().map(Item::Node).collect()
}
fn filter<'a>(
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
        Expr::Context => {
            if matches!(focus.item, Item::Absent) {
                return Err(err("XPDY0002", "context item is absent"));
            } else {
                vec![focus.item.clone()]
            }
        }
        Expr::Root => match focus.item {
            Item::Node(n) => vec![Item::Node(n.root())],
            Item::Absent => return Err(err("XPDY0002", "context item is absent")),
            _ => return Err(err("XPTY0020", "root requires a node context")),
        },
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
                        Axis::Child => match n {
                            XNode::Tree(n) => n.children().map(XNode::Tree).collect(),
                            _ => vec![],
                        },
                        Axis::Attribute => match n {
                            XNode::Tree(n) => n
                                .attributes()
                                .map(|a| XNode::Attribute(n, a.name(), a.namespace()))
                                .collect(),
                            _ => vec![],
                        },
                        Axis::Ancestor | Axis::AncestorOrSelf => {
                            let mut ancestors = vec![];
                            let mut current = if matches!(step.axis, Axis::AncestorOrSelf) {
                                Some(n)
                            } else {
                                n.parent()
                            };
                            while let Some(node) = current {
                                budget.tick()?;
                                ancestors.push(node);
                                current = node.parent();
                            }
                            ancestors
                        }
                        Axis::FollowingSibling | Axis::PrecedingSibling => match n {
                            XNode::Tree(n) => {
                                if matches!(step.axis, Axis::FollowingSibling) {
                                    n.next_siblings().skip(1).map(XNode::Tree).collect()
                                } else {
                                    n.prev_siblings().skip(1).map(XNode::Tree).collect()
                                }
                            }
                            _ => vec![],
                        },
                        Axis::Following | Axis::Preceding => {
                            let (owner, attribute) = match n {
                                XNode::Tree(n) => (n, false),
                                XNode::Attribute(n, ..) => (n, true),
                            };
                            let mut selected = vec![];
                            for candidate in owner.document().descendants() {
                                budget.tick()?;
                                let keep = if matches!(step.axis, Axis::Following) {
                                    candidate.id().get() > owner.id().get()
                                        && (attribute || !candidate.ancestors().any(|a| a == owner))
                                } else {
                                    candidate.id().get() < owner.id().get()
                                        && !owner.ancestors().any(|a| a == candidate)
                                };
                                if keep {
                                    selected.push(XNode::Tree(candidate));
                                }
                            }
                            if matches!(step.axis, Axis::Preceding) {
                                selected.reverse();
                            }
                            selected
                        }
                        Axis::Descendant | Axis::DescendantOrSelf => match n {
                            XNode::Tree(n) => n
                                .descendants()
                                .skip(usize::from(matches!(step.axis, Axis::Descendant)))
                                .map(XNode::Tree)
                                .collect(),
                            _ if matches!(step.axis, Axis::DescendantOrSelf) => vec![n],
                            _ => vec![],
                        },
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
            if op == "and" && !boolean(&a)? {
                return Ok(vec![Item::Boolean(false)]);
            }
            if op == "or" && boolean(&a)? {
                return Ok(vec![Item::Boolean(true)]);
            }
            let b = eval(b, focus, vars, budget, depth + 1, backwards)?;
            match op.as_str() {
                "and" | "or" => vec![Item::Boolean(boolean(&b)?)],
                "|" => {
                    let nodes = a
                        .into_iter()
                        .chain(b)
                        .map(|v| {
                            if let Item::Node(n) = v {
                                Ok(n)
                            } else {
                                Err(err("XPTY0004", "union requires nodes"))
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    ordered(nodes)
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
        [Item::Node(n)] => Number::cast("double", &n.string()),
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
    fn atom<'a>(i: &Item<'a>) -> Item<'a> {
        match i {
            Item::Node(n) => Item::Untyped(n.string()),
            _ => i.clone(),
        }
    }
    let (mut a, mut b) = (atom(a), atom(b));
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
fn function<'a>(
    name: &str,
    args: &[Value<'a>],
    focus: &Focus<'a>,
    budget: &mut Budget,
    backwards: bool,
) -> Result<Value<'a>, Diagnostic> {
    if args.is_empty()
        && matches!(focus.item, Item::Absent)
        && !["true", "false", "position", "last"].contains(&name)
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
        Ok(v.first().map_or(String::new(), Item::string))
    };
    let s = || scalar(first);
    if name == "data" {
        return Ok(first
            .iter()
            .map(|i| match i {
                Item::Node(n) => Item::Untyped(n.string()),
                _ => i.clone(),
            })
            .collect());
    }
    if let Some(kind) = name.strip_prefix("xs:") {
        if first.is_empty() {
            return Ok(vec![]);
        }
        if first.len() != 1 {
            return Err(err("XPTY0004", "constructor requires a singleton"));
        }
        let item = &first[0];
        return Ok(vec![match kind {
            "string" => Item::String(item.string()),
            "untypedAtomic" => Item::Untyped(item.string()),
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
        "sum" => {
            let mut sum = Number::from(0usize);
            for item in first {
                budget.tick()?;
                sum = sum.arithmetic(&numeric(std::slice::from_ref(item), backwards)?, "+")?;
            }
            Item::Number(sum)
        }
        "concat" => {
            let mut s = String::new();
            for a in args {
                s.push_str(&scalar(a)?);
            }
            Item::String(s)
        }
        "string-join" => Item::String(string(first, &scalar(&args[1])?)),
        "contains" | "starts-with" | "ends-with" | "substring-before" | "substring-after" => {
            let a = s()?;
            let b = scalar(&args[1])?;
            match name {
                "contains" => Item::Boolean(a.contains(&b)),
                "starts-with" => Item::Boolean(a.starts_with(&b)),
                "ends-with" => Item::Boolean(a.ends_with(&b)),
                "substring-before" => Item::String(a.find(&b).map_or("", |i| &a[..i]).into()),
                _ => Item::String(a.find(&b).map_or("", |i| &a[i + b.len()..]).into()),
            }
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
                        match n {
                            XNode::Tree(node) | XNode::Attribute(node, ..) => {
                                if let Some(uri) = uri {
                                    node.lookup_prefix(uri)
                                        .filter(|p| !p.is_empty())
                                        .map_or_else(|| local.into(), |p| format!("{p}:{local}"))
                                } else {
                                    local.into()
                                }
                            }
                        }
                    }
                }
                _ => return Err(err("XPTY0004", "name function requires one node")),
            };
            Item::String(text)
        }
        _ => unreachable!(),
    };
    Ok(vec![result])
}
