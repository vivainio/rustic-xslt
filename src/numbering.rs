//! Number formatting for `xsl:number` and `fn:format-integer`: decimal digit
//! patterns, alphabetic and Roman numerals, English words, grouping separators.

/// One formatting token: its characters decide the numbering system.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// Decimal digits with a minimum width and the zero digit of the family.
    Decimal {
        zero: char,
        width: usize,
    },
    Alpha(bool),
    Roman(bool),
    Words(Case),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Case {
    Lower,
    Upper,
    Title,
}

/// The zero digit of the Unicode decimal-digit run containing `c`, if any.
fn zero_of(c: char) -> Option<char> {
    if c.is_ascii_digit() {
        return Some('0');
    }
    if !c.is_numeric() {
        return None;
    }
    // A run of ten consecutive digits starts at the first code point whose
    // predecessor is not a digit of the same run.
    let mut start = c as u32;
    for _ in 0..9 {
        let previous = char::from_u32(start.checked_sub(1)?)?;
        if previous.is_numeric() && !previous.is_alphabetic() && start > 0 {
            start -= 1;
        } else {
            break;
        }
    }
    char::from_u32(start)
}

/// Interpret a format token such as `1`, `001`, `a`, `I`, `w`, `Ww`.
pub(crate) fn token(text: &str) -> Token {
    match text {
        "a" => Token::Alpha(false),
        "A" => Token::Alpha(true),
        "i" => Token::Roman(false),
        "I" => Token::Roman(true),
        "w" => Token::Words(Case::Lower),
        "W" => Token::Words(Case::Upper),
        "Ww" => Token::Words(Case::Title),
        _ => {
            let digits: Vec<char> = text.chars().filter(|c| c.is_numeric()).collect();
            match digits.last().copied().and_then(zero_of) {
                Some(zero) => Token::Decimal {
                    zero,
                    width: digits.len(),
                },
                None => Token::Decimal {
                    zero: '0',
                    width: 1,
                },
            }
        }
    }
}

fn alphabetic(mut n: u64, upper: bool) -> String {
    if n == 0 {
        return "0".into();
    }
    let base = if upper { b'A' } else { b'a' };
    let mut out = vec![];
    while n > 0 {
        n -= 1;
        out.push(base + (n % 26) as u8);
        n /= 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

fn roman(n: u64, upper: bool) -> Option<String> {
    if n == 0 || n >= 4000 {
        return None;
    }
    const TABLE: [(u64, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut rest = n;
    let mut out = String::new();
    for (value, symbol) in TABLE {
        while rest >= value {
            out.push_str(symbol);
            rest -= value;
        }
    }
    Some(if upper { out.to_uppercase() } else { out })
}

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];
const SCALES: [&str; 7] = [
    "",
    " thousand",
    " million",
    " billion",
    " trillion",
    " quadrillion",
    " quintillion",
];

fn below_thousand(n: u64) -> String {
    let mut parts = vec![];
    if n >= 100 {
        parts.push(format!("{} hundred", ONES[(n / 100) as usize]));
    }
    let rest = n % 100;
    if rest > 0 {
        let words = if rest < 20 {
            ONES[rest as usize].to_owned()
        } else if rest.is_multiple_of(10) {
            TENS[(rest / 10) as usize].to_owned()
        } else {
            format!(
                "{}-{}",
                TENS[(rest / 10) as usize],
                ONES[(rest % 10) as usize]
            )
        };
        if n >= 100 {
            parts.push(format!("and {words}"));
        } else {
            parts.push(words);
        }
    }
    parts.join(" ")
}

fn words(n: u64) -> String {
    if n == 0 {
        return "zero".into();
    }
    let mut groups = vec![];
    let mut rest = n;
    let mut scale = 0;
    while rest > 0 {
        let group = rest % 1000;
        if group > 0 {
            groups.push(format!("{}{}", below_thousand(group), SCALES[scale]));
        }
        rest /= 1000;
        scale += 1;
    }
    groups.reverse();
    groups.join(" ")
}

fn ordinal_words(n: u64) -> String {
    let cardinal = words(n);
    const IRREGULAR: [(&str, &str); 8] = [
        ("one", "first"),
        ("two", "second"),
        ("three", "third"),
        ("five", "fifth"),
        ("eight", "eighth"),
        ("nine", "ninth"),
        ("twelve", "twelfth"),
        ("zero", "zeroth"),
    ];
    let split = cardinal.rfind([' ', '-']).map_or(0, |i| i + 1);
    let (head, last) = cardinal.split_at(split);
    let replaced = if let Some((_, ordinal)) = IRREGULAR.iter().find(|(c, _)| *c == last) {
        (*ordinal).to_owned()
    } else if let Some(stem) = last.strip_suffix('y') {
        format!("{stem}ieth")
    } else {
        format!("{last}th")
    };
    format!("{head}{replaced}")
}

fn ordinal_suffix(n: u64) -> &'static str {
    match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    }
}

fn apply_case(text: String, case: Case) -> String {
    match case {
        Case::Lower => text,
        Case::Upper => text.to_uppercase(),
        Case::Title => {
            let mut out = String::new();
            let mut start = true;
            for c in text.chars() {
                if start {
                    out.extend(c.to_uppercase());
                } else {
                    out.push(c);
                }
                start = c == ' ' || c == '-';
            }
            out
        }
    }
}

/// Format one non-negative integer with a token.
pub(crate) fn format(n: u64, token: &Token, ordinal: bool) -> String {
    match token {
        Token::Decimal { zero, width } => {
            let digits = n.to_string();
            let padded = format!("{digits:0>width$}");
            let mut out: String = padded
                .chars()
                .map(|c| char::from_u32(*zero as u32 + c.to_digit(10).unwrap()).unwrap_or(c))
                .collect();
            if ordinal && *zero == '0' {
                out.push_str(ordinal_suffix(n));
            }
            out
        }
        Token::Alpha(upper) => alphabetic(n, *upper),
        Token::Roman(upper) => roman(n, *upper).unwrap_or_else(|| n.to_string()),
        Token::Words(case) => apply_case(if ordinal { ordinal_words(n) } else { words(n) }, *case),
    }
}

/// Insert `separator` every `size` digits from the right.
pub(crate) fn group(digits: &str, separator: &str, size: usize) -> String {
    if size == 0 || separator.is_empty() {
        return digits.to_owned();
    }
    let chars: Vec<char> = digits.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(size) {
            out.push_str(separator);
        }
        out.push(*c);
    }
    out
}

/// Split an `xsl:number` format string into (prefix, [(separator, token)], suffix).
pub(crate) fn parse_format(format: &str) -> (String, Vec<(String, String)>, String) {
    let chars: Vec<char> = format.chars().collect();
    let alnum = |c: char| c.is_alphanumeric();
    let mut i = 0;
    let mut prefix = String::new();
    while i < chars.len() && !alnum(chars[i]) {
        prefix.push(chars[i]);
        i += 1;
    }
    let mut tokens: Vec<(String, String)> = vec![];
    let mut separator = String::new();
    while i < chars.len() {
        let mut token = String::new();
        while i < chars.len() && alnum(chars[i]) {
            token.push(chars[i]);
            i += 1;
        }
        tokens.push((std::mem::take(&mut separator), token));
        let mut non = String::new();
        while i < chars.len() && !alnum(chars[i]) {
            non.push(chars[i]);
            i += 1;
        }
        if i < chars.len() {
            separator = non;
        } else {
            return (prefix, tokens, non);
        }
    }
    let suffix = if tokens.is_empty() {
        prefix.clone()
    } else {
        String::new()
    };
    (prefix, tokens, suffix)
}

pub(crate) struct Grouping {
    pub separator: String,
    pub size: usize,
}

/// Format a list of numbers according to an `xsl:number` format string.
pub(crate) fn format_list(
    numbers: &[num_bigint::BigInt],
    picture: &str,
    ordinal: bool,
    grouping: Option<&Grouping>,
) -> String {
    let (prefix, mut tokens, suffix) = parse_format(picture);
    if tokens.is_empty() {
        tokens.push((String::new(), "1".into()));
    }
    let mut out = prefix;
    for (index, n) in numbers.iter().enumerate() {
        // The last token and separator repeat for further numbers.
        let (separator, token_text) = match tokens.get(index) {
            Some((separator, text)) => (separator.clone(), text.clone()),
            None => {
                let (last_separator, last_text) = tokens.last().unwrap();
                let separator = if last_separator.is_empty() {
                    ".".to_owned()
                } else {
                    last_separator.clone()
                };
                (separator, last_text.clone())
            }
        };
        if index > 0 {
            out.push_str(&separator);
        }
        let tok = token(&token_text);
        use num_traits::{Signed, ToPrimitive};
        let magnitude = n.abs();
        let mut text = magnitude.to_u64().map_or_else(
            || match &tok {
                Token::Decimal { zero, width } => format!("{:0>width$}", magnitude.to_string())
                    .chars()
                    .map(|c| char::from_u32(*zero as u32 + c.to_digit(10).unwrap()).unwrap_or(c))
                    .collect(),
                _ => magnitude.to_string(),
            },
            |value| format(value, &tok, ordinal),
        );
        if let (Some(g), Token::Decimal { .. }) = (grouping, &tok) {
            text = group(&text, &g.separator, g.size);
        }
        if n.is_negative() {
            out.push('-');
        }
        out.push_str(&text);
    }
    out.push_str(&suffix);
    out
}

/// `fn:format-integer`: `picture` is a primary format token with optional modifiers.
pub(crate) fn format_integer(
    value: &bigdecimal::num_bigint::BigInt,
    picture: &str,
) -> Result<String, String> {
    use num_traits::{Signed, ToPrimitive};
    let invalid = || "FODF1310".to_owned();
    let (primary, modifier) = match picture.rsplit_once(';') {
        Some((p, m)) => (p, m),
        None => (picture, ""),
    };
    if primary.is_empty() {
        return Err(invalid());
    }
    let ordinal = modifier.starts_with('o');
    let magnitude = value.abs().to_u64().ok_or_else(invalid)?;
    let sign = if value.is_negative() { "-" } else { "" };
    let chars: Vec<char> = primary.chars().collect();
    let is_digit = |c: char| zero_of(c).is_some();
    let decimal_like = chars
        .iter()
        .all(|c| is_digit(*c) || *c == '#' || !c.is_alphanumeric())
        && chars.iter().any(|c| is_digit(*c) || *c == '#');
    let body = if decimal_like {
        if let Some(first_digit) = chars.iter().position(|c| is_digit(*c))
            && chars[first_digit..].contains(&'#')
        {
            return Err(invalid());
        }
        let digits: Vec<char> = chars.iter().copied().filter(|c| is_digit(*c)).collect();
        let zero = digits.first().and_then(|c| zero_of(*c)).unwrap_or('0');
        if digits.iter().any(|c| zero_of(*c) != Some(zero)) {
            return Err(invalid());
        }
        if chars
            .first()
            .is_some_and(|c| !c.is_alphanumeric() && *c != '#')
            || chars
                .last()
                .is_some_and(|c| !c.is_alphanumeric() && *c != '#')
            || chars.windows(2).any(|w| {
                !w[0].is_alphanumeric() && !w[1].is_alphanumeric() && w[0] != '#' && w[1] != '#'
            })
        {
            return Err(invalid());
        }
        let mut separators: Vec<(char, usize)> = vec![];
        let mut to_right = 0;
        for c in chars.iter().rev() {
            if c.is_alphanumeric() || *c == '#' {
                to_right += 1;
            } else {
                separators.push((*c, to_right));
            }
        }
        let width = digits.len();
        let token = Token::Decimal { zero, width };
        let mut text = format(magnitude, &token, ordinal);
        if let Some(&(separator, first)) = separators.first() {
            let regular = separators
                .iter()
                .enumerate()
                .all(|(i, (c, pos))| *c == separator && *pos == first * (i + 1));
            let rendered: String = if regular {
                group(&text, &separator.to_string(), first)
            } else {
                let chars: Vec<char> = text.chars().collect();
                let mut out = String::new();
                for (i, c) in chars.iter().enumerate() {
                    out.push(*c);
                    let remaining = chars.len() - i - 1;
                    if let Some((sep, _)) = separators.iter().find(|(_, pos)| *pos == remaining)
                        && remaining > 0
                    {
                        out.push(*sep);
                    }
                }
                out
            };
            text = rendered;
        }
        text
    } else {
        match primary {
            "a" | "A" | "i" | "I" | "w" | "W" | "Ww" => format(magnitude, &token(primary), ordinal),
            // Unrecognized tokens fall back to plain decimal numbering.
            _ => format(magnitude, &token("1"), ordinal),
        }
    };
    Ok(format!("{sign}{body}"))
}

pub(crate) fn parse_start_at(value: &str) -> Result<Vec<num_bigint::BigInt>, ()> {
    if value.is_empty()
        || value.starts_with([' ', '\t', '\r', '\n'])
        || value.ends_with([' ', '\t', '\r', '\n'])
    {
        return Err(());
    }
    value
        .split([' ', '\t', '\r', '\n'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let digits = part.strip_prefix('-').unwrap_or(part);
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(());
            }
            part.parse().map_err(|_| ())
        })
        .collect()
}
