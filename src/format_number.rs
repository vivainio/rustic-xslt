//! `fn:format-number` with the default decimal format: picture parsing,
//! rounding half to even, grouping, percent and per-mille scaling.
use crate::{Diagnostic, number::Number};
use bigdecimal::{BigDecimal, RoundingMode};
use num_traits::Zero;

fn picture_error(message: &str) -> Diagnostic {
    Diagnostic::new(
        "FODF1310",
        format!("invalid picture string: {message}"),
        "run",
    )
}

struct Pattern {
    prefix: String,
    suffix: String,
    min_integer: usize,
    /// Digit signs to the left of the leftmost grouping separator.
    leading_digits: usize,
    min_fraction: usize,
    max_fraction: usize,
    /// Distances (in digits) from the decimal point to each grouping separator.
    groups: Vec<usize>,
    scale: u32,
    /// Minimum exponent digits for a scientific picture such as `0.00e000`.
    exponent_digits: Option<usize>,
    /// Mandatory plus optional digit signs in the integer part.
    integer_size: usize,
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn parse(sub: &str) -> Result<Pattern, Diagnostic> {
    let all: Vec<char> = sub.chars().collect();
    let exponent = all.iter().enumerate().position(|(i, c)| {
        *c == 'e'
            && i > 0
            && (is_digit(all[i - 1]) || matches!(all[i - 1], '#' | '.' | ','))
            && all.get(i + 1).is_some_and(|d| is_digit(*d))
    });
    if let Some(e) = exponent {
        let digits = all[e + 1..].iter().take_while(|c| is_digit(**c)).count();
        let mantissa: String = all[..e].iter().collect();
        let mut pattern = parse(&mantissa)?;
        if !pattern.suffix.is_empty() {
            return Err(picture_error("text between the mantissa and the exponent"));
        }
        pattern.suffix = all[e + 1 + digits..].iter().collect();
        pattern.exponent_digits = Some(digits);
        return Ok(pattern);
    }
    let chars = all;
    let active = |c: char| is_digit(c) || matches!(c, '#' | '.' | ',');
    let first = chars
        .iter()
        .position(|c| active(*c))
        .ok_or_else(|| picture_error("no digit sign"))?;
    let last = chars.iter().rposition(|c| active(*c)).unwrap();
    if chars[first..=last].iter().any(|c| !active(*c)) {
        return Err(picture_error("passive characters inside the number"));
    }
    let prefix: String = chars[..first].iter().collect();
    let suffix: String = chars[last + 1..].iter().collect();
    let body = &chars[first..=last];
    if !body.iter().any(|c| is_digit(*c) || *c == '#') {
        return Err(picture_error("no digit sign"));
    }
    let percents = sub.chars().filter(|c| *c == '%').count();
    let permilles = sub.chars().filter(|c| *c == '\u{2030}').count();
    if percents + permilles > 1 {
        return Err(picture_error("more than one percent or per-mille sign"));
    }
    let point = body.iter().position(|c| *c == '.');
    if body.iter().filter(|c| **c == '.').count() > 1 {
        return Err(picture_error("more than one decimal separator"));
    }
    let (integer, fraction) = match point {
        Some(i) => (&body[..i], &body[i + 1..]),
        None => (body, &body[..0]),
    };
    if integer.last() == Some(&',')
        || fraction.first() == Some(&',')
        || fraction.last() == Some(&',')
        || integer.first() == Some(&',')
    {
        return Err(picture_error(
            "grouping separator adjacent to the decimal separator",
        ));
    }
    if body.windows(2).any(|w| w[0] == ',' && w[1] == ',') {
        return Err(picture_error("adjacent grouping separators"));
    }
    if fraction.contains(&',') {
        return Err(picture_error("grouping separator in the fractional part"));
    }
    // '#' may not follow a mandatory digit in the integer part, nor precede one in the fraction.
    let int_digits: Vec<char> = integer.iter().copied().filter(|c| *c != ',').collect();
    if let Some(zero) = int_digits.iter().position(|c| is_digit(*c))
        && int_digits[zero..].contains(&'#')
    {
        return Err(picture_error("optional digit after a mandatory digit"));
    }
    if let Some(hash) = fraction.iter().position(|c| *c == '#')
        && fraction[hash..].iter().any(|c| is_digit(*c))
    {
        return Err(picture_error("mandatory digit after an optional digit"));
    }
    let mut groups = vec![];
    let mut digits_to_right = 0;
    for c in integer.iter().rev() {
        if *c == ',' {
            groups.push(digits_to_right);
        } else {
            digits_to_right += 1;
        }
    }
    Ok(Pattern {
        prefix,
        suffix,
        min_integer: int_digits.iter().filter(|c| is_digit(**c)).count(),
        leading_digits: integer.iter().take_while(|c| **c != ',').count(),
        min_fraction: fraction.iter().filter(|c| is_digit(**c)).count(),
        max_fraction: fraction.len(),
        groups,
        exponent_digits: None,
        integer_size: int_digits.len(),
        scale: if percents > 0 {
            100
        } else if permilles > 0 {
            1000
        } else {
            1
        },
    })
}

fn group(digits: &str, groups: &[usize], leading: usize) -> String {
    if groups.is_empty() {
        return digits.to_owned();
    }
    let smallest = *groups.iter().min().unwrap();
    let regular = smallest > 0
        && (1..=smallest).contains(&leading)
        && groups.iter().all(|g| g % smallest == 0)
        && (1..=groups.len()).all(|i| groups.contains(&(i * smallest)));
    let separator_after = |from_right: usize| {
        if regular {
            from_right > 0 && from_right % smallest == 0
        } else {
            groups.contains(&from_right)
        }
    };
    let chars: Vec<char> = digits.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        let from_right = chars.len() - i;
        out.push(*c);
        if from_right > 1 && separator_after(from_right - 1) {
            out.push(',');
        }
    }
    out
}

pub(crate) fn format(value: &Number, picture: &str) -> Result<String, Diagnostic> {
    let parts: Vec<&str> = picture.split(';').collect();
    if parts.len() > 2 {
        return Err(picture_error("more than one pattern separator"));
    }
    let positive = parse(parts[0])?;
    let negative = parts.get(1).map(|p| parse(p)).transpose()?;
    let double = value.double();
    if double.is_nan() {
        return Ok("NaN".into());
    }
    let negative_value = double.is_sign_negative()
        && !matches!(value, Number::Integer(_) | Number::DerivedInteger(..))
        || double < 0.0;
    let (pattern, sign) = match (&negative, negative_value) {
        (Some(n), true) => (n, ""),
        (None, true) => (&positive, "-"),
        (_, false) => (&positive, ""),
    };
    let overflow = matches!(value, Number::Double(_) | Number::Float(_))
        && !(double * f64::from(pattern.scale)).is_finite();
    let decimal = value.shortest_decimal().filter(|_| !overflow);
    let Some(decimal) = decimal else {
        return Ok(format!(
            "{sign}{}Infinity{}",
            pattern.prefix, pattern.suffix
        ));
    };
    let mut scaled = decimal.abs() * BigDecimal::from(pattern.scale);
    let mut exponent = 0i64;
    let shift = |value: &BigDecimal, by: i64| {
        let (int, scale) = value.as_bigint_and_exponent();
        BigDecimal::new(int, scale + by)
    };
    if pattern.exponent_digits.is_some() && !scaled.is_zero() {
        // Scale the mantissa so that its integer part has the picture's digit count.
        let size = pattern.min_integer as i64;
        let (int, scale) = scaled.as_bigint_and_exponent();
        exponent = int.to_string().len() as i64 - scale - size;
        scaled = shift(&scaled, exponent);
    }
    let mut rounded = scaled.with_scale_round(pattern.max_fraction as i64, RoundingMode::HalfEven);
    if pattern.exponent_digits.is_some() {
        let size = pattern.min_integer as i64;
        let (int, scale) = rounded.as_bigint_and_exponent();
        if size > 0 && !rounded.is_zero() && int.to_string().len() as i64 - scale > size {
            exponent += 1;
            rounded = shift(&rounded, 1)
                .with_scale_round(pattern.max_fraction as i64, RoundingMode::HalfEven);
        }
    }
    let text = rounded.to_string();
    let (int_part, frac_part) = match text.split_once('.') {
        Some((i, f)) => (i.to_owned(), f.to_owned()),
        None => (text, String::new()),
    };
    let mut int_part = int_part.trim_start_matches('0').to_owned();
    while int_part.len() < pattern.min_integer {
        int_part.insert(0, '0');
    }
    let mut frac_part = frac_part;
    while frac_part.len() > pattern.min_fraction && frac_part.ends_with('0') {
        frac_part.pop();
    }
    while frac_part.len() < pattern.min_fraction {
        frac_part.push('0');
    }
    if pattern.exponent_digits.is_some() && int_part.is_empty() && pattern.integer_size > 0 {
        int_part.push('0');
    }
    if int_part.is_empty() && frac_part.is_empty() {
        if pattern.max_fraction > 0 {
            frac_part.push('0');
        } else {
            int_part.push('0');
        }
    }
    let mut out = format!(
        "{sign}{}{}",
        pattern.prefix,
        group(&int_part, &pattern.groups, pattern.leading_digits)
    );
    if !frac_part.is_empty() {
        out.push('.');
        out.push_str(&frac_part);
    }
    if let Some(width) = pattern.exponent_digits {
        out.push('e');
        if exponent < 0 {
            out.push('-');
        }
        out.push_str(&format!("{:0width$}", exponent.unsigned_abs()));
    }
    out.push_str(&pattern.suffix);
    Ok(out)
}
