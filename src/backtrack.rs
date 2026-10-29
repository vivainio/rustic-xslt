//! A bounded backtracking matcher for XPath regular expressions that use
//! backreferences, which the linear-time engine cannot express. Single-character
//! atoms (classes, escapes, `.`) are delegated to precompiled regexes.
use crate::{Diagnostic, xpath::Budget};
use regex::Regex;

fn invalid() -> Diagnostic {
    Diagnostic::new("FORX0002", "invalid XPath regular expression", "run")
}

const MAX_PROGRAM: usize = 50_000;

enum Node {
    Empty,
    Literal(char),
    Atom(usize),
    Bol,
    Eol,
    Group(Box<Node>, Option<usize>),
    Concat(Vec<Node>),
    Alternation(Vec<Node>),
    Repeat {
        node: Box<Node>,
        min: usize,
        max: Option<usize>,
        greedy: bool,
    },
    Backref(usize),
}

#[derive(Clone, Copy)]
enum Inst {
    Literal(char),
    Atom(usize),
    Split(usize, usize),
    Jump(usize),
    Save(usize),
    Mark(usize),
    Check(usize),
    Bol,
    Eol,
    Backref(usize),
    Match,
}

pub(crate) struct Backtracking {
    program: Vec<Inst>,
    atoms: Vec<Regex>,
    groups: usize,
    registers: usize,
    ignore_case: bool,
    multiline: bool,
}

/// Capture spans in character indices; `None` for groups that did not participate.
pub(crate) type Spans = Vec<Option<(usize, usize)>>;

struct Parser<'a> {
    chars: Vec<char>,
    index: usize,
    groups: usize,
    /// Groups whose closing parenthesis has been read; only these can be referenced.
    closed: Vec<bool>,
    extended: bool,
    atoms: Vec<Regex>,
    translate: &'a dyn Fn(&str) -> Result<Regex, Diagnostic>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }
    fn skip_extended(&mut self) {
        while self.extended && matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
            self.index += 1;
        }
    }
    fn is_closed(&self, group: usize) -> bool {
        self.closed.get(group).copied().unwrap_or(false)
    }
    fn atom(&mut self, fragment: &str) -> Result<Node, Diagnostic> {
        self.atoms.push((self.translate)(fragment)?);
        Ok(Node::Atom(self.atoms.len() - 1))
    }
    fn alternation(&mut self, depth: usize) -> Result<Node, Diagnostic> {
        if depth > 64 {
            return Err(Diagnostic::new(
                "XPDY0130",
                "regex nesting limit exceeded",
                "run",
            ));
        }
        let mut branches = vec![self.concatenation(depth)?];
        while self.peek() == Some('|') {
            self.index += 1;
            branches.push(self.concatenation(depth)?);
        }
        Ok(if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            Node::Alternation(branches)
        })
    }
    fn concatenation(&mut self, depth: usize) -> Result<Node, Diagnostic> {
        let mut items = vec![];
        loop {
            self.skip_extended();
            match self.peek() {
                None | Some('|') | Some(')') => break,
                _ => {}
            }
            let atom = self.quantified(depth)?;
            items.push(atom);
        }
        Ok(match items.len() {
            0 => Node::Empty,
            1 => items.pop().unwrap(),
            _ => Node::Concat(items),
        })
    }
    fn number(&mut self) -> Option<usize> {
        let start = self.index;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.index += 1;
        }
        self.chars[start..self.index]
            .iter()
            .collect::<String>()
            .parse()
            .ok()
    }
    fn quantified(&mut self, depth: usize) -> Result<Node, Diagnostic> {
        let node = self.primary(depth)?;
        self.skip_extended();
        let (min, max) = match self.peek() {
            Some('*') => {
                self.index += 1;
                (0, None)
            }
            Some('+') => {
                self.index += 1;
                (1, None)
            }
            Some('?') => {
                self.index += 1;
                (0, Some(1))
            }
            Some('{') => {
                self.index += 1;
                let min = self.number().ok_or_else(invalid)?;
                let max = if self.peek() == Some(',') {
                    self.index += 1;
                    if self.peek() == Some('}') {
                        None
                    } else {
                        Some(self.number().ok_or_else(invalid)?)
                    }
                } else {
                    Some(min)
                };
                if self.peek() != Some('}') || max.is_some_and(|m| m < min) {
                    return Err(invalid());
                }
                self.index += 1;
                (min, max)
            }
            _ => return Ok(node),
        };
        let greedy = if self.peek() == Some('?') {
            self.index += 1;
            false
        } else {
            true
        };
        Ok(Node::Repeat {
            node: Box::new(node),
            min,
            max,
            greedy,
        })
    }
    fn primary(&mut self, depth: usize) -> Result<Node, Diagnostic> {
        let c = self.peek().ok_or_else(invalid)?;
        self.index += 1;
        match c {
            '(' => {
                let capture = if self.peek() == Some('?') {
                    if self.chars.get(self.index + 1) != Some(&':') {
                        return Err(invalid());
                    }
                    self.index += 2;
                    None
                } else {
                    self.groups += 1;
                    self.closed.push(false);
                    Some(self.groups)
                };
                let inner = self.alternation(depth + 1)?;
                if self.peek() != Some(')') {
                    return Err(invalid());
                }
                self.index += 1;
                if let Some(n) = capture {
                    self.closed[n] = true;
                }
                Ok(Node::Group(Box::new(inner), capture))
            }
            '[' => {
                let start = self.index - 1;
                let mut nesting = 1;
                while nesting > 0 {
                    match self.peek().ok_or_else(invalid)? {
                        '\\' => self.index += 1,
                        '[' => nesting += 1,
                        ']' => nesting -= 1,
                        _ => {}
                    }
                    self.index += 1;
                }
                let fragment: String = self.chars[start..self.index].iter().collect();
                self.atom(&fragment)
            }
            '.' => self.atom("."),
            '^' => Ok(Node::Bol),
            '$' => Ok(Node::Eol),
            '\\' => {
                let escaped = self.peek().ok_or_else(invalid)?;
                self.index += 1;
                match escaped {
                    '1'..='9' => {
                        let mut number = escaped.to_digit(10).unwrap() as usize;
                        while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
                            let wider = number * 10 + d as usize;
                            if !self.is_closed(wider) {
                                break;
                            }
                            number = wider;
                            self.index += 1;
                        }
                        if !self.is_closed(number) {
                            return Err(invalid());
                        }
                        Ok(Node::Backref(number))
                    }
                    'n' => Ok(Node::Literal('\n')),
                    'r' => Ok(Node::Literal('\r')),
                    't' => Ok(Node::Literal('\t')),
                    '\\' | '|' | '.' | '-' | '^' | '?' | '*' | '+' | '{' | '}' | '(' | ')'
                    | '[' | ']' | '$' => Ok(Node::Literal(escaped)),
                    'd' | 'D' | 's' | 'S' | 'w' | 'W' | 'i' | 'I' | 'c' | 'C' => {
                        self.atom(&format!("\\{escaped}"))
                    }
                    'p' | 'P' => {
                        if self.peek() != Some('{') {
                            return Err(invalid());
                        }
                        let start = self.index - 2;
                        while self.peek().ok_or_else(invalid)? != '}' {
                            self.index += 1;
                        }
                        self.index += 1;
                        let fragment: String = self.chars[start..self.index].iter().collect();
                        self.atom(&fragment)
                    }
                    _ => Err(invalid()),
                }
            }
            '*' | '+' | '?' | '{' | '}' | ']' => Err(invalid()),
            c => Ok(Node::Literal(c)),
        }
    }
}

fn emit(
    node: &Node,
    program: &mut Vec<Inst>,
    registers: &mut usize,
    total_groups: usize,
) -> Result<(), Diagnostic> {
    if program.len() > MAX_PROGRAM {
        return Err(Diagnostic::new(
            "XPDY0130",
            "compiled regex size limit exceeded",
            "run",
        ));
    }
    match node {
        Node::Empty => {}
        Node::Literal(c) => program.push(Inst::Literal(*c)),
        Node::Atom(i) => program.push(Inst::Atom(*i)),
        Node::Bol => program.push(Inst::Bol),
        Node::Eol => program.push(Inst::Eol),
        Node::Backref(n) => {
            if *n > total_groups {
                return Err(invalid());
            }
            program.push(Inst::Backref(*n));
        }
        Node::Group(inner, capture) => {
            if let Some(n) = capture {
                program.push(Inst::Save(2 * n));
                emit(inner, program, registers, total_groups)?;
                program.push(Inst::Save(2 * n + 1));
            } else {
                emit(inner, program, registers, total_groups)?;
            }
        }
        Node::Concat(items) => {
            for item in items {
                emit(item, program, registers, total_groups)?;
            }
        }
        Node::Alternation(branches) => {
            let mut jumps = vec![];
            for (index, branch) in branches.iter().enumerate() {
                if index + 1 < branches.len() {
                    let split = program.len();
                    program.push(Inst::Split(split + 1, 0));
                    emit(branch, program, registers, total_groups)?;
                    jumps.push(program.len());
                    program.push(Inst::Jump(0));
                    let next = program.len();
                    program[split] = Inst::Split(split + 1, next);
                } else {
                    emit(branch, program, registers, total_groups)?;
                }
            }
            let end = program.len();
            for j in jumps {
                program[j] = Inst::Jump(end);
            }
        }
        Node::Repeat {
            node,
            min,
            max,
            greedy,
        } => {
            for _ in 0..*min {
                emit(node, program, registers, total_groups)?;
            }
            match max {
                None => {
                    let register = *registers;
                    *registers += 1;
                    let split = program.len();
                    program.push(Inst::Split(0, 0));
                    program.push(Inst::Mark(register));
                    emit(node, program, registers, total_groups)?;
                    program.push(Inst::Check(register));
                    program.push(Inst::Jump(split));
                    let end = program.len();
                    program[split] = if *greedy {
                        Inst::Split(split + 1, end)
                    } else {
                        Inst::Split(end, split + 1)
                    };
                }
                Some(max) => {
                    let mut splits = vec![];
                    for _ in *min..*max {
                        splits.push(program.len());
                        program.push(Inst::Split(0, 0));
                        emit(node, program, registers, total_groups)?;
                    }
                    let end = program.len();
                    for s in splits {
                        program[s] = if *greedy {
                            Inst::Split(s + 1, end)
                        } else {
                            Inst::Split(end, s + 1)
                        };
                    }
                }
            }
        }
    }
    Ok(())
}

fn fold(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

enum Undo {
    Branch(usize, usize),
    Capture(usize, Option<usize>),
    Register(usize, usize),
}

impl Backtracking {
    pub fn new(
        pattern: &str,
        flags: &str,
        translate: &dyn Fn(&str) -> Result<Regex, Diagnostic>,
    ) -> Result<Self, Diagnostic> {
        let chars: Vec<char> = pattern.chars().collect();
        let mut parser = Parser {
            chars,
            index: 0,
            groups: 0,
            closed: vec![true],
            extended: flags.contains('x'),
            atoms: vec![],
            translate,
        };
        let root = parser.alternation(0)?;
        if parser.index != parser.chars.len() {
            return Err(invalid());
        }
        let mut program = vec![Inst::Save(0)];
        let mut registers = 0;
        emit(&root, &mut program, &mut registers, parser.groups)?;
        program.push(Inst::Save(1));
        program.push(Inst::Match);
        Ok(Self {
            program,
            atoms: parser.atoms,
            groups: parser.groups,
            registers,
            ignore_case: flags.contains('i'),
            multiline: flags.contains('m'),
        })
    }

    pub fn groups(&self) -> usize {
        self.groups
    }

    fn atom_matches(&self, index: usize, c: char) -> bool {
        let mut buffer = [0u8; 4];
        self.atoms[index].is_match(c.encode_utf8(&mut buffer))
    }

    fn run(
        &self,
        text: &[char],
        start: usize,
        budget: &mut Budget,
    ) -> Result<Option<Spans>, Diagnostic> {
        let mut captures: Vec<Option<usize>> = vec![None; 2 * (self.groups + 1)];
        let mut registers = vec![usize::MAX; self.registers];
        let mut stack: Vec<Undo> = vec![];
        let (mut pc, mut pos) = (0usize, start);
        loop {
            budget.tick()?;
            let ok = match self.program[pc] {
                Inst::Literal(c) => {
                    let hit = text
                        .get(pos)
                        .is_some_and(|t| *t == c || self.ignore_case && fold(*t) == fold(c));
                    if hit {
                        pos += 1;
                        pc += 1;
                    }
                    hit
                }
                Inst::Atom(index) => {
                    let hit = text.get(pos).is_some_and(|t| self.atom_matches(index, *t));
                    if hit {
                        pos += 1;
                        pc += 1;
                    }
                    hit
                }
                Inst::Split(first, second) => {
                    stack.push(Undo::Branch(second, pos));
                    pc = first;
                    true
                }
                Inst::Jump(target) => {
                    pc = target;
                    true
                }
                Inst::Save(slot) => {
                    stack.push(Undo::Capture(slot, captures[slot]));
                    captures[slot] = Some(pos);
                    pc += 1;
                    true
                }
                Inst::Mark(register) => {
                    stack.push(Undo::Register(register, registers[register]));
                    registers[register] = pos;
                    pc += 1;
                    true
                }
                Inst::Check(register) => {
                    let progressed = registers[register] != pos;
                    if progressed {
                        pc += 1;
                    }
                    progressed
                }
                Inst::Bol => {
                    let hit = pos == 0 || self.multiline && text[pos - 1] == '\n';
                    if hit {
                        pc += 1;
                    }
                    hit
                }
                Inst::Eol => {
                    let hit = pos == text.len() || self.multiline && text[pos] == '\n';
                    if hit {
                        pc += 1;
                    }
                    hit
                }
                Inst::Backref(n) => match (captures[2 * n], captures[2 * n + 1]) {
                    (Some(s), Some(e)) if e >= s => {
                        let length = e - s;
                        let hit = pos + length <= text.len()
                            && (0..length).all(|i| {
                                let (a, b) = (text[s + i], text[pos + i]);
                                a == b || self.ignore_case && fold(a) == fold(b)
                            });
                        if hit {
                            pos += length;
                            pc += 1;
                        }
                        hit
                    }
                    _ => false,
                },
                Inst::Match => {
                    return Ok(Some(
                        (0..=self.groups)
                            .map(|g| match (captures[2 * g], captures[2 * g + 1]) {
                                (Some(s), Some(e)) => Some((s, e)),
                                _ => None,
                            })
                            .collect(),
                    ));
                }
            };
            if ok {
                continue;
            }
            loop {
                match stack.pop() {
                    None => return Ok(None),
                    Some(Undo::Branch(next_pc, next_pos)) => {
                        pc = next_pc;
                        pos = next_pos;
                        break;
                    }
                    Some(Undo::Capture(slot, old)) => captures[slot] = old,
                    Some(Undo::Register(register, old)) => registers[register] = old,
                }
            }
        }
    }

    /// All non-overlapping matches, scanning left to right.
    pub fn find_all(&self, text: &[char], budget: &mut Budget) -> Result<Vec<Spans>, Diagnostic> {
        let mut found = vec![];
        let mut start = 0;
        while start <= text.len() {
            let mut matched = None;
            for from in start..=text.len() {
                if let Some(spans) = self.run(text, from, budget)? {
                    matched = Some(spans);
                    break;
                }
            }
            let Some(spans) = matched else { break };
            let (s, e) = spans[0].unwrap();
            start = if e > s { e } else { e + 1 };
            found.push(spans);
        }
        Ok(found)
    }
}
