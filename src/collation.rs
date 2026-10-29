use crate::Diagnostic;

const CODEPOINT: &str = "http://www.w3.org/2005/xpath-functions/collation/codepoint";
const HTML_ASCII_CASE_INSENSITIVE: &str =
    "http://www.w3.org/2005/xpath-functions/collation/html-ascii-case-insensitive";
const CASE_BLIND: &str = "http://www.w3.org/2010/09/qt-fots-catalog/collation/caseblind";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Collation {
    Codepoint,
    AsciiCaseInsensitive,
    CaseBlind,
}

impl Collation {
    pub(crate) fn parse(uri: &str) -> Result<Self, Diagnostic> {
        match uri {
            CODEPOINT => Ok(Self::Codepoint),
            HTML_ASCII_CASE_INSENSITIVE => Ok(Self::AsciiCaseInsensitive),
            CASE_BLIND => Ok(Self::CaseBlind),
            _ => Err(Diagnostic::new("FOCH0002", "unsupported collation", "run")),
        }
    }

    /// Collation keys are sequences of folded characters; one source character
    /// always yields one key character so key positions map back to the source.
    pub(crate) fn key(self, s: &str) -> Vec<char> {
        match self {
            Self::Codepoint => s.chars().collect(),
            Self::AsciiCaseInsensitive => s.chars().map(|c| c.to_ascii_lowercase()).collect(),
            Self::CaseBlind => s
                .chars()
                .map(|c| {
                    let mut lower = c.to_lowercase();
                    match (lower.next(), lower.next()) {
                        (Some(l), None) => l,
                        _ => c,
                    }
                })
                .collect(),
        }
    }

    pub(crate) fn compare(self, a: &str, b: &str) -> std::cmp::Ordering {
        match self {
            Self::Codepoint => a.cmp(b),
            _ => self.key(a).cmp(&self.key(b)),
        }
    }

    /// Character index of the first match of `needle` in `haystack`, if any.
    pub(crate) fn find(self, haystack: &[char], needle: &[char]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        haystack.windows(needle.len()).position(|w| w == needle)
    }
}
