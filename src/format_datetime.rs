//! `fn:format-date`, `fn:format-time` and `fn:format-dateTime`: picture strings
//! with `[Y0001]`-style variable markers, English names and numbering modifiers.
use crate::{
    Diagnostic,
    numbering::{self, Token},
    temporal::{Temporal, TemporalType, day_of_year, iso_week, iso_weekday},
};

fn picture_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("FOFD1340", message, "run")
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

struct Marker {
    component: char,
    presentation: String,
    ordinal: bool,
    min_width: Option<usize>,
    max_width: Option<usize>,
}

fn parse_marker(text: &str) -> Result<Marker, Diagnostic> {
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut chars = text.chars();
    let component = chars
        .next()
        .ok_or_else(|| picture_error("empty variable marker"))?;
    if !"YMDdFWwECHhPmsfZz".contains(component) {
        return Err(picture_error(format!(
            "unknown component specifier {component}"
        )));
    }
    let rest: String = chars.collect();
    let (modifier, width) = match rest.split_once(',') {
        Some((m, w)) => (m.to_owned(), Some(w.to_owned())),
        None => (rest, None),
    };
    let (mut presentation, ordinal) = match modifier.strip_suffix('o') {
        Some(p) => (p.to_owned(), true),
        None => (modifier.clone(), false),
    };
    if !matches!(component, 'Z' | 'z')
        && presentation.ends_with(['a', 't'])
        && presentation.len() > 1
        && !presentation.starts_with(['a', 'A'])
    {
        presentation.pop();
    }
    let (mut min_width, mut max_width) = (None, None);
    if let Some(width) = width {
        let (min, max) = width.split_once('-').unwrap_or((width.as_str(), ""));
        let parse = |s: &str| -> Result<Option<usize>, Diagnostic> {
            if s.is_empty() || s == "*" {
                Ok(None)
            } else {
                s.parse()
                    .map(Some)
                    .map_err(|_| picture_error("invalid width modifier"))
            }
        };
        min_width = parse(min)?;
        max_width = if width.contains('-') {
            parse(max)?
        } else {
            None
        };
        if !width.contains('-') && min_width.is_some() {
            max_width = None;
        }
    }
    Ok(Marker {
        component,
        presentation,
        ordinal,
        min_width,
        max_width,
    })
}

fn pad_to(mut text: String, min: Option<usize>, max: Option<usize>, numeric: bool) -> String {
    if let Some(max) = max
        && text.chars().count() > max
    {
        text = if numeric {
            let skip = text.chars().count() - max;
            text.chars().skip(skip).collect()
        } else {
            text.chars().take(max).collect()
        };
    }
    if let Some(min) = min {
        while text.chars().count() < min {
            if numeric {
                text.insert(0, '0');
            } else {
                text.push(' ');
            }
        }
    }
    text
}

fn name_case(name: &str, presentation: &str) -> String {
    match presentation {
        "N" => name.to_uppercase(),
        "n" => name.to_lowercase(),
        _ => name.to_owned(),
    }
}

/// Format a time zone offset according to the marker's presentation modifier.
fn timezone_text(m: &Marker, tz: Option<i32>) -> String {
    let military = m.component == 'Z' && m.presentation == "Z";
    let Some(offset) = tz else {
        return if military { "J".into() } else { String::new() };
    };
    if military && offset % 60 == 0 && (-720..=720).contains(&offset) {
        let hours = offset / 60;
        const AHEAD: &str = "ZABCDEFGHIKLM";
        const BEHIND: &str = "NOPQRSTUVWXY";
        return if hours >= 0 {
            AHEAD.chars().nth(hours as usize).unwrap().to_string()
        } else {
            BEHIND
                .chars()
                .nth((-hours - 1) as usize)
                .unwrap()
                .to_string()
        };
    }
    let mut presentation = if military || m.presentation.is_empty() {
        "01:01".to_owned()
    } else {
        m.presentation.clone()
    };
    let zulu = presentation.ends_with('t');
    if zulu {
        presentation.pop();
    }
    if zulu && offset == 0 {
        return "Z".into();
    }
    let digits_of = |text: &str| text.chars().filter(|c| numbering_digit(*c)).count();
    let zero = presentation
        .chars()
        .find(|c| numbering_digit(*c))
        .map_or('0', zero_digit);
    let (hours, minutes) = (offset.unsigned_abs() / 60, offset.unsigned_abs() % 60);
    let separator = presentation.chars().find(|c| !numbering_digit(*c));
    let (hour_width, minute_width, joined) = match separator {
        Some(sep) => {
            let (left, right) = presentation.split_once(sep).unwrap();
            (
                digits_of(left).max(1),
                Some(digits_of(right).max(1)),
                Some(sep),
            )
        }
        None => match digits_of(&presentation) {
            0 | 1 => (1, None, None),
            2 => (2, None, None),
            n => (n - 2, Some(2), Some('\0')),
        },
    };
    let localize = |text: String| -> String {
        text.chars()
            .map(|c| match c.to_digit(10) {
                Some(d) => char::from_u32(zero as u32 + d).unwrap_or(c),
                None => c,
            })
            .collect()
    };
    let mut out = String::new();
    if m.component == 'z' {
        out.push_str("GMT");
    }
    out.push(if offset < 0 { '-' } else { '+' });
    out.push_str(&localize(format!("{hours:0>hour_width$}")));
    match (minute_width, joined) {
        (Some(width), Some(sep)) => {
            if sep != '\0' {
                out.push(sep);
            }
            out.push_str(&localize(format!("{minutes:0>width$}")));
        }
        _ => {
            if minutes != 0 || (m.component == 'z' && m.presentation.is_empty()) {
                out.push(':');
                out.push_str(&localize(format!("{minutes:02}")));
            }
        }
    }
    out
}
fn numbering_digit(c: char) -> bool {
    c.is_ascii_digit() || zero_digit_run(c)
}
fn zero_digit_run(c: char) -> bool {
    c.is_numeric() && !c.is_ascii()
}
fn zero_digit(c: char) -> char {
    if c.is_ascii_digit() {
        return '0';
    }
    let mut start = c as u32;
    for _ in 0..9 {
        match char::from_u32(start.wrapping_sub(1)) {
            Some(p) if p.is_numeric() => start -= 1,
            _ => break,
        }
    }
    char::from_u32(start).unwrap_or('0')
}
fn component_text(m: &Marker, value: &Temporal) -> Result<String, Diagnostic> {
    let fields = value.fields();
    let has_date = matches!(value.kind, TemporalType::DateTime | TemporalType::Date);
    let has_time = matches!(value.kind, TemporalType::DateTime | TemporalType::Time);
    let needs_date = "YMDdFWwEC".contains(m.component);
    let needs_time = "HhPmsf".contains(m.component);
    if (needs_date && !has_date) || (needs_time && !has_time) {
        return Err(Diagnostic::new(
            "FOFD1350",
            format!(
                "component {} is not available for {}",
                m.component,
                value.kind.name()
            ),
            "run",
        ));
    }
    let weekday = iso_weekday(fields.year, fields.month, fields.day);
    let (number, default_presentation): (Option<i64>, &str) = match m.component {
        'Y' => (Some(fields.year), "1"),
        'M' => (Some(i64::from(fields.month)), "1"),
        'D' => (Some(i64::from(fields.day)), "1"),
        'd' => (
            Some(i64::from(day_of_year(
                fields.year,
                fields.month,
                fields.day,
            ))),
            "1",
        ),
        'F' => (Some(i64::from(weekday)), "n"),
        'W' => (
            Some(i64::from(iso_week(fields.year, fields.month, fields.day).1)),
            "1",
        ),
        'w' => (
            Some(i64::from(
                (fields.day as u32 + iso_weekday(fields.year, fields.month, 1) - 1)
                    .div_ceil(7)
                    .max(1),
            )),
            "1",
        ),
        'H' => (Some(i64::from(fields.hour)), "1"),
        'h' => (
            Some(i64::from(match fields.hour % 12 {
                0 => 12,
                h => h,
            })),
            "1",
        ),
        'm' => (Some(i64::from(fields.minute)), "01"),
        's' => (Some(i64::from(fields.second)), "01"),
        'f' => (Some(i64::from(fields.nanos)), "1"),
        _ => (None, "n"),
    };
    let presentation = if m.presentation.is_empty() {
        default_presentation.to_owned()
    } else {
        m.presentation.clone()
    };
    match m.component {
        'P' => {
            let text = if fields.hour < 12 { "am" } else { "pm" };
            let text = match presentation.as_str() {
                "N" => text.to_uppercase(),
                "Nn" => {
                    let mut chars = text.chars();
                    chars
                        .next()
                        .map_or(String::new(), |f| f.to_uppercase().chain(chars).collect())
                }
                _ => text.to_owned(),
            };
            return Ok(pad_to(text, m.min_width, m.max_width, false));
        }
        'E' => {
            let text = if fields.year > 0 { "AD" } else { "BC" };
            return Ok(pad_to(
                name_case(text, &presentation),
                m.min_width,
                m.max_width,
                false,
            ));
        }
        'C' => return Ok(pad_to("ISO".to_owned(), m.min_width, m.max_width, false)),
        'Z' | 'z' => {
            return Ok(pad_to(
                timezone_text(m, fields.tz),
                m.min_width,
                m.max_width,
                false,
            ));
        }
        _ => {}
    }
    let number = number.unwrap();
    // Names for month and weekday components.
    if matches!(presentation.as_str(), "N" | "n" | "Nn") && matches!(m.component, 'M' | 'F') {
        let name = if m.component == 'M' {
            MONTHS[(number - 1) as usize]
        } else {
            DAYS[(number - 1) as usize]
        };
        let name = name_case(name, &presentation);
        // Without a width, names are not truncated.
        return Ok(pad_to(name, m.min_width, m.max_width, false));
    }
    if m.component == 'f' {
        // Fractional seconds are truncated, never rounded.
        let digits = format!("{number:09}");
        let trimmed = digits.trim_end_matches('0');
        let token = numbering::token(&presentation);
        let (zero, width) = match token {
            Token::Decimal { zero, width } => (zero, width),
            _ => ('0', 1),
        };
        let text: String = if m.min_width.is_some() || m.max_width.is_some() {
            let min = m.min_width.unwrap_or(1);
            let mut text = trimmed.to_owned();
            while text.len() < min {
                text.push('0');
            }
            if let Some(max) = m.max_width {
                text.truncate(max);
            }
            text
        } else if presentation == "1" || presentation.is_empty() {
            if trimmed.is_empty() {
                "0".to_owned()
            } else {
                trimmed.to_owned()
            }
        } else {
            let mut text = digits.clone();
            while text.len() < width {
                text.push('0');
            }
            text.truncate(width);
            text
        };
        return Ok(text
            .chars()
            .map(|c| char::from_u32(zero as u32 + c.to_digit(10).unwrap_or(0)).unwrap_or(c))
            .collect());
    }
    let token = numbering::token(&presentation);
    let negative = number < 0;
    let magnitude = number.unsigned_abs();
    let mut text = numbering::format(magnitude, &token, m.ordinal);
    let numeric = matches!(token, Token::Decimal { .. });
    if m.component == 'Y' && numeric && m.max_width.is_none() {
        // Two-digit year pictures keep the rightmost digits.
        if let Token::Decimal { width: 2, .. } = token {
            let skip = text.chars().count().saturating_sub(2);
            text = text.chars().skip(skip).collect();
        }
    }
    let text = pad_to(text, m.min_width, m.max_width, numeric);
    Ok(if negative { format!("-{text}") } else { text })
}

pub(crate) fn format(value: &Temporal, picture: &str) -> Result<String, Diagnostic> {
    let chars: Vec<char> = picture.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '[' if chars.get(i + 1) == Some(&'[') => {
                out.push('[');
                i += 2;
            }
            ']' if chars.get(i + 1) == Some(&']') => {
                out.push(']');
                i += 2;
            }
            '[' => {
                let end = chars[i..]
                    .iter()
                    .position(|c| *c == ']')
                    .map(|p| i + p)
                    .ok_or_else(|| picture_error("unterminated variable marker"))?;
                let marker: String = chars[i + 1..end].iter().collect();
                out.push_str(&component_text(&parse_marker(&marker)?, value)?);
                i = end + 1;
            }
            ']' => return Err(picture_error("unmatched ']' in picture")),
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    Ok(out)
}
