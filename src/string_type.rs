//! XML Schema string restriction types and their whitespace/name facets.
use crate::Diagnostic;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StringType {
    Normalized,
    Token,
    Language,
    Name,
    NcName,
    NmToken,
    Id,
    IdRef,
    Entity,
}
impl StringType {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "normalizedString" => Self::Normalized,
            "token" => Self::Token,
            "language" => Self::Language,
            "Name" => Self::Name,
            "NCName" => Self::NcName,
            "NMTOKEN" => Self::NmToken,
            "ID" => Self::Id,
            "IDREF" => Self::IdRef,
            "ENTITY" => Self::Entity,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Normalized => "normalizedString",
            Self::Token => "token",
            Self::Language => "language",
            Self::Name => "Name",
            Self::NcName => "NCName",
            Self::NmToken => "NMTOKEN",
            Self::Id => "ID",
            Self::IdRef => "IDREF",
            Self::Entity => "ENTITY",
        }
    }
    pub fn subtype_of(self, target: Self) -> bool {
        let mut current = Some(self);
        while let Some(kind) = current {
            if kind == target {
                return true;
            }
            current = match kind {
                Self::Id | Self::IdRef | Self::Entity => Some(Self::NcName),
                Self::NcName => Some(Self::Name),
                Self::Name | Self::Language | Self::NmToken => Some(Self::Token),
                Self::Token => Some(Self::Normalized),
                Self::Normalized => None,
            };
        }
        false
    }
    pub fn cast(self, text: &str) -> Result<String, Diagnostic> {
        let text = if self == Self::Normalized {
            text.chars()
                .map(|c| {
                    if matches!(c, '\t' | '\n' | '\r') {
                        ' '
                    } else {
                        c
                    }
                })
                .collect()
        } else {
            crate::string_functions::collapse_xml_whitespace(text)
        };
        let valid = match self {
            Self::Normalized | Self::Token => true,
            Self::Language => {
                let mut parts = text.split('-');
                parts.next().is_some_and(|p| {
                    (1..=8).contains(&p.len()) && p.bytes().all(|c| c.is_ascii_alphabetic())
                }) && parts.all(|p| {
                    (1..=8).contains(&p.len()) && p.bytes().all(|c| c.is_ascii_alphanumeric())
                })
            }
            Self::Name => {
                let mut chars = text.chars();
                chars
                    .next()
                    .is_some_and(|c| c == ':' || crate::qname::xml_ncname_start(c))
                    && chars.all(|c| c == ':' || crate::qname::xml_ncname_char(c))
            }
            Self::NmToken => {
                !text.is_empty()
                    && text
                        .chars()
                        .all(|c| c == ':' || crate::qname::xml_ncname_char(c))
            }
            Self::NcName | Self::Id | Self::IdRef | Self::Entity => crate::qname::xml_ncname(&text),
        };
        if valid {
            Ok(text)
        } else {
            Err(Diagnostic::new(
                "FORG0001",
                format!("value does not satisfy xs:{} facets", self.name()),
                "run",
            ))
        }
    }
}
