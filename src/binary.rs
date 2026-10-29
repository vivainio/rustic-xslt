//! XML Schema binary value spaces, strict lexical decoding and canonical strings.
use crate::Diagnostic;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryType {
    Hex,
    Base64,
}
impl BinaryType {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "hexBinary" => Some(Self::Hex),
            "base64Binary" => Some(Self::Base64),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Hex => "hexBinary",
            Self::Base64 => "base64Binary",
        }
    }
    pub fn decode(self, text: &str) -> Result<Vec<u8>, Diagnostic> {
        let invalid = || Diagnostic::new("FORG0001", "invalid binary lexical value", "run");
        if self == Self::Hex {
            let text = text.trim_matches([' ', '\t', '\n', '\r']);
            if !text.len().is_multiple_of(2) {
                return Err(invalid());
            }
            fn nibble(c: u8) -> Option<u8> {
                match c {
                    b'0'..=b'9' => Some(c - b'0'),
                    b'a'..=b'f' => Some(c - b'a' + 10),
                    b'A'..=b'F' => Some(c - b'A' + 10),
                    _ => None,
                }
            }
            return text
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| {
                    Ok(nibble(pair[0]).ok_or_else(invalid)? * 16
                        + nibble(pair[1]).ok_or_else(invalid)?)
                })
                .collect();
        }
        let text = text
            .bytes()
            .filter(|b| !b" \t\r\n".contains(b))
            .collect::<Vec<_>>();
        if !text.len().is_multiple_of(4) {
            return Err(invalid());
        }
        fn digit(c: u8) -> Option<u8> {
            match c {
                b'A'..=b'Z' => Some(c - b'A'),
                b'a'..=b'z' => Some(c - b'a' + 26),
                b'0'..=b'9' => Some(c - b'0' + 52),
                b'+' => Some(62),
                b'/' => Some(63),
                _ => None,
            }
        }
        let mut result = Vec::new();
        for (index, quartet) in text.as_chunks::<4>().0.iter().enumerate() {
            let a = digit(quartet[0]).ok_or_else(invalid)?;
            let b = digit(quartet[1]).ok_or_else(invalid)?;
            let last = (index + 1) * 4 == text.len();
            result.push((a << 2) | (b >> 4));
            if quartet[2] == b'=' {
                if !last || quartet[3] != b'=' || b & 15 != 0 {
                    return Err(invalid());
                }
            } else {
                let c = digit(quartet[2]).ok_or_else(invalid)?;
                result.push((b << 4) | (c >> 2));
                if quartet[3] == b'=' {
                    if !last || c & 3 != 0 {
                        return Err(invalid());
                    }
                } else {
                    result.push((c << 6) | digit(quartet[3]).ok_or_else(invalid)?);
                }
            }
        }
        Ok(result)
    }
    pub fn encode(self, bytes: &[u8]) -> String {
        const HEX: &[u8] = b"0123456789ABCDEF";
        const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut text = String::new();
        if self == Self::Hex {
            for &byte in bytes {
                text.push(HEX[(byte >> 4) as usize] as char);
                text.push(HEX[(byte & 15) as usize] as char);
            }
        } else {
            for chunk in bytes.chunks(3) {
                let a = chunk[0];
                let b = chunk.get(1).copied().unwrap_or(0);
                let c = chunk.get(2).copied().unwrap_or(0);
                text.push(BASE64[(a >> 2) as usize] as char);
                text.push(BASE64[(((a & 3) << 4) | (b >> 4)) as usize] as char);
                text.push(if chunk.len() > 1 {
                    BASE64[(((b & 15) << 2) | (c >> 6)) as usize] as char
                } else {
                    '='
                });
                text.push(if chunk.len() > 2 {
                    BASE64[(c & 63) as usize] as char
                } else {
                    '='
                });
            }
        }
        text
    }
}
