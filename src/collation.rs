//! String collations: codepoint, case-insensitive variants and an approximation of
//! the Unicode Collation Algorithm (primary base letters, secondary accents,
//! tertiary case) selected through `http://www.w3.org/2013/collation/UCA` URIs.
use crate::Diagnostic;
use std::cmp::Ordering;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

const CODEPOINT: &str = "http://www.w3.org/2005/xpath-functions/collation/codepoint";
const HTML_ASCII_CASE_INSENSITIVE: &str =
    "http://www.w3.org/2005/xpath-functions/collation/html-ascii-case-insensitive";
const CASE_BLIND: &str = "http://www.w3.org/2010/09/qt-fots-catalog/collation/caseblind";
const UCA: &str = "http://www.w3.org/2013/collation/UCA";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Strength {
    Primary,
    Secondary,
    Tertiary,
    Identical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Collation {
    Codepoint,
    AsciiCaseInsensitive,
    CaseBlind,
    Uca {
        strength: Strength,
        numeric: bool,
        /// Spaces and punctuation are ignored (alternate=blanked/shifted).
        ignore_variable: bool,
        upper_first: bool,
        /// Primary strength that still distinguishes case (caseLevel=yes).
        case_level: bool,
    },
}

/// One collation element group: a base character with its accents and case.
struct Unit {
    base: char,
    accents: Vec<char>,
    upper: bool,
    start: usize,
    end: usize,
    original: char,
}

fn unsupported() -> Diagnostic {
    Diagnostic::new("FOCH0002", "unsupported collation", "run")
}

impl Collation {
    pub(crate) fn parse(uri: &str) -> Result<Self, Diagnostic> {
        match uri {
            CODEPOINT => return Ok(Self::Codepoint),
            HTML_ASCII_CASE_INSENSITIVE => return Ok(Self::AsciiCaseInsensitive),
            CASE_BLIND => return Ok(Self::CaseBlind),
            _ => {}
        }
        let Some(rest) = uri.strip_prefix(UCA) else {
            return Err(unsupported());
        };
        let query = match rest {
            "" => "",
            _ => rest.strip_prefix('?').ok_or_else(unsupported)?,
        };
        let mut strength = Strength::Tertiary;
        let (mut numeric, mut ignore_variable, mut upper_first, mut case_level) =
            (false, false, false, false);
        let mut fallback = true;
        for parameter in query.split(';').filter(|p| !p.is_empty()) {
            let (key, value) = parameter.split_once('=').ok_or_else(unsupported)?;
            match (key, value) {
                ("lang", _) | ("version", _) | ("maxVariable", _) | ("reorder", _) => {}
                ("fallback", "yes") => fallback = true,
                ("fallback", "no") => fallback = false,
                ("strength", "primary") => strength = Strength::Primary,
                ("strength", "secondary") => strength = Strength::Secondary,
                ("strength", "tertiary" | "quaternary") => strength = Strength::Tertiary,
                ("strength", "identical") => strength = Strength::Identical,
                ("alternate", "blanked" | "shifted") => ignore_variable = true,
                ("alternate", "non-ignorable") => ignore_variable = false,
                ("numeric", "yes") => numeric = true,
                ("numeric", "no") => numeric = false,
                ("caseFirst", "upper") => upper_first = true,
                ("caseFirst", "lower" | "off") => upper_first = false,
                ("caseLevel", "yes") => case_level = true,
                ("caseLevel", "no") => case_level = false,
                ("normalization", "yes" | "no") => {}
                _ if fallback => {}
                _ => return Err(unsupported()),
            }
        }
        // Parameters seen after a `fallback=no` still count; recheck unknown ones strictly.
        if !fallback {
            for parameter in query.split(';').filter(|p| !p.is_empty()) {
                let key = parameter.split('=').next().unwrap_or("");
                if !matches!(
                    key,
                    "lang"
                        | "version"
                        | "maxVariable"
                        | "reorder"
                        | "fallback"
                        | "strength"
                        | "alternate"
                        | "numeric"
                        | "caseFirst"
                        | "caseLevel"
                        | "normalization"
                ) {
                    return Err(unsupported());
                }
            }
        }
        Ok(Self::Uca {
            strength,
            numeric,
            ignore_variable,
            upper_first,
            case_level,
        })
    }

    fn units(self, text: &str) -> Vec<Unit> {
        let mut units: Vec<Unit> = vec![];
        for (index, original) in text.chars().enumerate() {
            if is_combining_mark(original)
                && let Some(last) = units.last_mut()
                && matches!(self, Self::Uca { .. })
            {
                last.accents.push(original);
                last.end = index + 1;
                continue;
            }
            let (base, accents, upper) = match self {
                Self::Codepoint => (original, vec![], false),
                Self::AsciiCaseInsensitive => (original.to_ascii_lowercase(), vec![], false),
                Self::CaseBlind => (fold(original), vec![], false),
                Self::Uca { .. } => {
                    let mut decomposed = original.to_string().nfd().collect::<Vec<_>>();
                    let first = decomposed.remove(0);
                    let upper = first.is_uppercase();
                    (fold(first), decomposed, upper)
                }
            };
            units.push(Unit {
                base,
                accents,
                upper,
                start: index,
                end: index + 1,
                original,
            });
        }
        units
    }

    /// Primary weight: symbols sort before digits, digits before letters.
    fn primary(unit: &Unit) -> (u8, char) {
        let c = unit.base;
        if c.is_alphabetic() {
            (2, c)
        } else if c.is_numeric() {
            (1, c)
        } else {
            (0, c)
        }
    }

    fn variable(unit: &Unit) -> bool {
        !unit.base.is_alphanumeric()
    }

    fn unit_equal(self, a: &Unit, b: &Unit) -> bool {
        match self {
            Self::Uca {
                strength,
                case_level,
                ..
            } => {
                a.base == b.base
                    && (strength < Strength::Secondary || a.accents == b.accents)
                    && (strength < Strength::Tertiary && !case_level || a.upper == b.upper)
            }
            _ => a.base == b.base,
        }
    }

    pub(crate) fn compare(self, a: &str, b: &str) -> Ordering {
        let Self::Uca {
            strength,
            numeric,
            ignore_variable,
            upper_first,
            case_level,
        } = self
        else {
            return match self {
                Self::Codepoint => a.cmp(b),
                _ => {
                    let (left, right) = (self.units(a), self.units(b));
                    left.iter()
                        .map(|u| u.base)
                        .cmp(right.iter().map(|u| u.base))
                }
            };
        };
        let filter = |units: Vec<Unit>| -> Vec<Unit> {
            if ignore_variable {
                units.into_iter().filter(|u| !Self::variable(u)).collect()
            } else {
                units
            }
        };
        let (left, right) = (filter(self.units(a)), filter(self.units(b)));
        // Primary level, with optional numeric ordering of digit runs.
        let weights = |units: &[Unit]| -> Vec<(u8, u64, char)> {
            let mut out = vec![];
            let mut i = 0;
            while i < units.len() {
                let (class, c) = Self::primary(&units[i]);
                if numeric && class == 1 && units[i].base.is_ascii_digit() {
                    let mut value = 0u64;
                    while i < units.len() && units[i].base.is_ascii_digit() {
                        value = value
                            .saturating_mul(10)
                            .saturating_add(u64::from(units[i].base.to_digit(10).unwrap()));
                        i += 1;
                    }
                    out.push((1, value, '\0'));
                } else {
                    out.push((class, 0, c));
                    i += 1;
                }
            }
            out
        };
        let primary = weights(&left).cmp(&weights(&right));
        if primary != Ordering::Equal {
            return primary;
        }
        if strength >= Strength::Secondary {
            let secondary = left
                .iter()
                .map(|u| u.accents.clone())
                .cmp(right.iter().map(|u| u.accents.clone()));
            if secondary != Ordering::Equal {
                return secondary;
            }
        }
        if strength >= Strength::Tertiary || case_level {
            let tertiary = left
                .iter()
                .map(|u| u.upper)
                .cmp(right.iter().map(|u| u.upper));
            if tertiary != Ordering::Equal {
                return if upper_first {
                    tertiary.reverse()
                } else {
                    tertiary
                };
            }
        }
        if strength == Strength::Identical {
            return a.cmp(b);
        }
        Ordering::Equal
    }

    /// A byte string whose lexicographic order matches `compare` (for `fn:collation-key`).
    pub(crate) fn sort_key(self, text: &str) -> Vec<u8> {
        let Self::Uca {
            strength,
            numeric,
            ignore_variable,
            upper_first,
            case_level,
        } = self
        else {
            return match self {
                Self::Codepoint => text.as_bytes().to_vec(),
                _ => self
                    .units(text)
                    .iter()
                    .flat_map(|u| u.base.to_string().into_bytes())
                    .collect(),
            };
        };
        let units: Vec<Unit> = self
            .units(text)
            .into_iter()
            .filter(|u| !(ignore_variable && Self::variable(u)))
            .collect();
        let mut key = vec![];
        let mut i = 0;
        while i < units.len() {
            let (class, c) = Self::primary(&units[i]);
            if numeric && class == 1 && units[i].base.is_ascii_digit() {
                let mut value = 0u64;
                while i < units.len() && units[i].base.is_ascii_digit() {
                    value = value
                        .saturating_mul(10)
                        .saturating_add(u64::from(units[i].base.to_digit(10).unwrap()));
                    i += 1;
                }
                key.push(1);
                key.extend(value.to_be_bytes());
                key.extend(0u32.to_be_bytes());
            } else {
                key.push(class);
                key.extend((c as u32).to_be_bytes());
                i += 1;
            }
        }
        if strength >= Strength::Secondary {
            key.push(0);
            for unit in &units {
                for accent in &unit.accents {
                    key.extend((*accent as u32).to_be_bytes());
                }
                key.push(0xFF);
            }
        }
        if strength >= Strength::Tertiary || case_level {
            key.push(0);
            for unit in &units {
                key.push(u8::from(unit.upper != upper_first));
            }
        }
        if strength == Strength::Identical {
            key.push(0);
            key.extend(text.as_bytes());
        }
        key
    }

    /// Whether two strings are equal under this collation.
    pub(crate) fn equal(self, a: &str, b: &str) -> bool {
        self.compare(a, b) == Ordering::Equal
    }

    /// Character span (start, end) of the first occurrence of `needle` in `haystack`.
    pub(crate) fn find(self, haystack: &str, needle: &str) -> Option<(usize, usize)> {
        let ignore_variable = matches!(
            self,
            Self::Uca {
                ignore_variable: true,
                ..
            }
        );
        let filter = |units: Vec<Unit>| -> Vec<Unit> {
            if ignore_variable {
                units.into_iter().filter(|u| !Self::variable(u)).collect()
            } else {
                units
            }
        };
        let (text, pattern) = (filter(self.units(haystack)), filter(self.units(needle)));
        if pattern.is_empty() {
            return Some((0, 0));
        }
        (0..=text.len().checked_sub(pattern.len())?).find_map(|from| {
            let window = &text[from..from + pattern.len()];
            window
                .iter()
                .zip(&pattern)
                .all(|(a, b)| self.unit_equal(a, b))
                .then(|| (window[0].start, window[window.len() - 1].end))
        })
    }

    pub(crate) fn starts_with(self, haystack: &str, needle: &str) -> bool {
        let (text, pattern) = (self.units(haystack), self.units(needle));
        text.len() >= pattern.len()
            && text
                .iter()
                .zip(&pattern)
                .all(|(a, b)| self.unit_equal(a, b))
    }

    pub(crate) fn ends_with(self, haystack: &str, needle: &str) -> bool {
        let (text, pattern) = (self.units(haystack), self.units(needle));
        text.len() >= pattern.len()
            && text[text.len() - pattern.len()..]
                .iter()
                .zip(&pattern)
                .all(|(a, b)| self.unit_equal(a, b))
    }

    /// The source characters of a haystack, for slicing around a span.
    pub(crate) fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }
}

fn fold(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

impl Unit {
    #[allow(dead_code)]
    fn source(&self) -> char {
        self.original
    }
}
