//! `fn:parse-json`: RFC 8259 text to maps, arrays, strings, doubles and booleans.
use crate::{
    Diagnostic,
    number::Number,
    xpath::{Item, MapValue, Value},
};
use std::rc::Rc;

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("FOJS0001", message, "run")
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Duplicates {
    UseFirst,
    UseLast,
    Reject,
}

pub(crate) struct Options {
    pub duplicates: Duplicates,
    pub liberal: bool,
    /// Keep JSON escape sequences in strings instead of expanding them.
    pub escape: bool,
}

struct Parser<'t> {
    chars: Vec<char>,
    index: usize,
    options: &'t Options,
}

const MAX_DEPTH: usize = 128;

/// Characters outside the XML 1.0 `Char` production become U+FFFD.
fn xml_char(code: u32) -> char {
    match code {
        0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF => {
            char::from_u32(code).unwrap_or('\u{FFFD}')
        }
        _ => '\u{FFFD}',
    }
}

impl<'t> Parser<'t> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }
    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.index += 1;
        }
    }
    fn expect(&mut self, c: char) -> Result<(), Diagnostic> {
        if self.peek() == Some(c) {
            self.index += 1;
            Ok(())
        } else {
            Err(invalid(format!("expected '{c}' in JSON text")))
        }
    }
    fn literal(&mut self, word: &str) -> Result<(), Diagnostic> {
        for c in word.chars() {
            self.expect(c)?;
        }
        Ok(())
    }
    fn value<'a>(&mut self, depth: usize) -> Result<Value<'a>, Diagnostic> {
        if depth > MAX_DEPTH {
            return Err(invalid("JSON text is nested too deeply"));
        }
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.object(depth),
            Some('[') => self.array(depth),
            Some('"') => Ok(vec![Item::String(self.string()?)]),
            Some('t') => {
                self.literal("true")?;
                Ok(vec![Item::Boolean(true)])
            }
            Some('f') => {
                self.literal("false")?;
                Ok(vec![Item::Boolean(false)])
            }
            Some('n') => {
                self.literal("null")?;
                Ok(vec![])
            }
            Some('-' | '0'..='9') => self.number(),
            _ => Err(invalid("unexpected character in JSON text")),
        }
    }
    fn number<'a>(&mut self) -> Result<Value<'a>, Diagnostic> {
        let start = self.index;
        if self.peek() == Some('-') {
            self.index += 1;
        }
        match self.peek() {
            Some('0') => self.index += 1,
            Some('1'..='9') => {
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.index += 1;
                }
            }
            _ => return Err(invalid("invalid JSON number")),
        }
        if self.peek() == Some('.') {
            self.index += 1;
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(invalid("invalid JSON number"));
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.index += 1;
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.index += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.index += 1;
            }
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(invalid("invalid JSON number"));
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.index += 1;
            }
        }
        let text: String = self.chars[start..self.index].iter().collect();
        let number: f64 = text.parse().map_err(|_| invalid("invalid JSON number"))?;
        Ok(vec![Item::Number(Number::Double(number))])
    }
    fn hex4(&mut self) -> Result<u32, Diagnostic> {
        let mut value = 0;
        for _ in 0..4 {
            let digit = self
                .peek()
                .and_then(|c| c.to_digit(16))
                .ok_or_else(|| invalid("invalid \\u escape in JSON string"))?;
            value = value * 16 + digit;
            self.index += 1;
        }
        Ok(value)
    }
    fn string(&mut self) -> Result<String, Diagnostic> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            let c = self
                .peek()
                .ok_or_else(|| invalid("unterminated JSON string"))?;
            self.index += 1;
            match c {
                '"' => return Ok(out),
                '\\' if self.options.escape => {
                    let escape = self
                        .peek()
                        .ok_or_else(|| invalid("unterminated JSON string"))?;
                    self.index += 1;
                    out.push('\\');
                    out.push(escape);
                    match escape {
                        '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' => {}
                        'u' => {
                            for _ in 0..4 {
                                let digit = self
                                    .peek()
                                    .filter(|c| c.is_ascii_hexdigit())
                                    .ok_or_else(|| invalid("invalid \\u escape in JSON string"))?;
                                out.push(digit);
                                self.index += 1;
                            }
                        }
                        _ if self.options.liberal => {}
                        _ => return Err(invalid("invalid escape in JSON string")),
                    }
                }
                '\\' => {
                    let escape = self
                        .peek()
                        .ok_or_else(|| invalid("unterminated JSON string"))?;
                    self.index += 1;
                    match escape {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let first = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&first)
                                && self.peek() == Some('\\')
                                && self.chars.get(self.index + 1) == Some(&'u')
                            {
                                let saved = self.index;
                                self.index += 2;
                                match self.hex4() {
                                    Ok(second) if (0xDC00..0xE000).contains(&second) => {
                                        0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                                    }
                                    _ => {
                                        self.index = saved;
                                        first
                                    }
                                }
                            } else {
                                first
                            };
                            out.push(xml_char(code));
                        }
                        _ if self.options.liberal => out.push(escape),
                        _ => return Err(invalid("invalid escape in JSON string")),
                    }
                }
                c if (c as u32) < 0x20 && !self.options.liberal => {
                    return Err(invalid("control character in JSON string"));
                }
                c => out.push(if self.options.escape { c } else { xml_char(c as u32) }),
            }
        }
    }
    fn array<'a>(&mut self, depth: usize) -> Result<Value<'a>, Diagnostic> {
        self.expect('[')?;
        let mut members: Vec<Value<'a>> = vec![];
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.index += 1;
            return Ok(vec![Item::Array(Rc::new(members))]);
        }
        loop {
            members.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => self.index += 1,
                Some(']') => {
                    self.index += 1;
                    return Ok(vec![Item::Array(Rc::new(members))]);
                }
                _ => return Err(invalid("expected ',' or ']' in JSON array")),
            }
        }
    }
    fn object<'a>(&mut self, depth: usize) -> Result<Value<'a>, Diagnostic> {
        self.expect('{')?;
        let mut map = MapValue::default();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.index += 1;
            return Ok(vec![Item::Map(Rc::new(map))]);
        }
        loop {
            self.skip_whitespace();
            let key = Item::String(self.string()?);
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.value(depth + 1)?;
            match (map.get(&key).is_some(), self.options.duplicates) {
                (true, Duplicates::Reject) => {
                    return Err(Diagnostic::new(
                        "FOJS0003",
                        "duplicate key in JSON object",
                        "run",
                    ));
                }
                (true, Duplicates::UseFirst) => {}
                _ => map.insert(key, value),
            }
            self.skip_whitespace();
            match self.peek() {
                Some(',') => self.index += 1,
                Some('}') => {
                    self.index += 1;
                    return Ok(vec![Item::Map(Rc::new(map))]);
                }
                _ => return Err(invalid("expected ',' or '}' in JSON object")),
            }
        }
    }
}

pub(crate) fn parse<'a>(text: &str, options: &Options) -> Result<Value<'a>, Diagnostic> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        index: 0,
        options,
    };
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.index != parser.chars.len() {
        return Err(invalid("unexpected content after the JSON value"));
    }
    Ok(value)
}
