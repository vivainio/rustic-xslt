//! XML Schema date, time and duration value spaces: strict lexical parsing,
//! canonical strings, ordering, component access and calendar normalization.
use crate::Diagnostic;
use regex::Regex;
use std::{cmp::Ordering, sync::LazyLock};

const NANOS_PER_SECOND: i128 = 1_000_000_000;
const NANOS_PER_MINUTE: i128 = 60 * NANOS_PER_SECOND;
const NANOS_PER_HOUR: i128 = 60 * NANOS_PER_MINUTE;
const NANOS_PER_DAY: i128 = 24 * NANOS_PER_HOUR;
/// Moments without a date use this reference day, as the XPath comparison rules do.
const REFERENCE_DAY: (i64, u8, u8) = (1972, 12, 31);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemporalType {
    DateTime,
    Date,
    Time,
    Duration,
    YearMonthDuration,
    DayTimeDuration,
}

impl TemporalType {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "dateTime" => Self::DateTime,
            "date" => Self::Date,
            "time" => Self::Time,
            "duration" => Self::Duration,
            "yearMonthDuration" => Self::YearMonthDuration,
            "dayTimeDuration" => Self::DayTimeDuration,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::DateTime => "dateTime",
            Self::Date => "date",
            Self::Time => "time",
            Self::Duration => "duration",
            Self::YearMonthDuration => "yearMonthDuration",
            Self::DayTimeDuration => "dayTimeDuration",
        }
    }
    pub fn is_duration(self) -> bool {
        matches!(
            self,
            Self::Duration | Self::YearMonthDuration | Self::DayTimeDuration
        )
    }
    /// Whether a value of type `actual` is an instance of `self`.
    pub fn accepts(self, actual: Self) -> bool {
        self == actual || (self == Self::Duration && actual.is_duration())
    }
}

/// A moment (`dateTime`, `date`, `time`) or a duration, discriminated by `kind`.
/// Moment fields hold the local fields and `tz` offset in minutes; duration
/// fields hold signed months and signed nanoseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Temporal {
    pub kind: TemporalType,
    year: i64,
    month: u8,
    day: u8,
    nanos: i128,
    tz: Option<i32>,
    months: i64,
}

fn invalid(what: &str) -> Diagnostic {
    Diagnostic::new("FORG0001", format!("invalid {what} lexical value"), "run")
}

const TZ: &str = r"(Z|[+-]\d\d:\d\d)?";
static DATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(-?\d{{4,}})-(\d\d)-(\d\d){TZ}$")).unwrap());
static TIME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(\d\d):(\d\d):(\d\d)(?:\.(\d+))?{TZ}$")).unwrap());
static DATE_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(-?\d{{4,}})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)(?:\.(\d+))?{TZ}$"
    ))
    .unwrap()
});
static DURATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(-)?P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)(?:\.(\d+))?S)?)?$",
    )
    .unwrap()
});

pub(crate) fn days_from_civil(year: i64, month: u8, day: u8) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub(crate) fn civil_from_days(days: i64) -> (i64, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if year.rem_euclid(4) == 0
            && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0) =>
        {
            29
        }
        _ => 28,
    }
}

fn timezone(text: Option<&str>) -> Result<Option<i32>, Diagnostic> {
    let Some(text) = text else { return Ok(None) };
    if text == "Z" {
        return Ok(Some(0));
    }
    let sign = if text.starts_with('-') { -1 } else { 1 };
    let hours: i32 = text[1..3].parse().unwrap();
    let minutes: i32 = text[4..6].parse().unwrap();
    if minutes > 59 || hours * 60 + minutes > 14 * 60 {
        return Err(invalid("timezone"));
    }
    Ok(Some(sign * (hours * 60 + minutes)))
}

fn fraction(digits: Option<&str>) -> i128 {
    let digits = digits.unwrap_or("");
    let mut padded = digits.chars().take(9).collect::<String>();
    while padded.len() < 9 {
        padded.push('0');
    }
    padded.parse().unwrap()
}

impl Temporal {
    fn moment(kind: TemporalType) -> Self {
        let (year, month, day) = REFERENCE_DAY;
        Self {
            kind,
            year,
            month,
            day,
            nanos: 0,
            tz: None,
            months: 0,
        }
    }

    pub fn parse(kind: TemporalType, text: &str) -> Result<Self, Diagnostic> {
        let text = text.trim_matches([' ', '\t', '\r', '\n']);
        match kind {
            TemporalType::Duration
            | TemporalType::YearMonthDuration
            | TemporalType::DayTimeDuration => Self::parse_duration(kind, text),
            _ => Self::parse_moment(kind, text),
        }
    }

    fn parse_moment(kind: TemporalType, text: &str) -> Result<Self, Diagnostic> {
        let name = kind.name();
        let re = match kind {
            TemporalType::Date => &DATE,
            TemporalType::Time => &TIME,
            _ => &DATE_TIME,
        };
        let caps = re.captures(text).ok_or_else(|| invalid(name))?;
        let get = |i: usize| caps.get(i).map(|m| m.as_str());
        let number = |i: usize| -> u32 { get(i).unwrap().parse().unwrap() };
        let mut value = Self::moment(kind);
        let mut next = 1;
        if kind != TemporalType::Time {
            let digits = get(1).unwrap().trim_start_matches('-');
            if digits.len() > 4 && digits.starts_with('0') {
                return Err(invalid(name));
            }
            value.year = get(1).unwrap().parse().map_err(|_| invalid(name))?;
            value.month = number(2) as u8;
            value.day = number(3) as u8;
            if !(1..=12).contains(&value.month)
                || value.day == 0
                || value.day > days_in_month(value.year, value.month)
            {
                return Err(invalid(name));
            }
            next = 4;
        }
        if kind != TemporalType::Date {
            let (h, m, s) = (number(next), number(next + 1), number(next + 2));
            let frac = fraction(get(next + 3));
            let end_of_day = h == 24;
            if m > 59 || s > 59 || h > 24 || (end_of_day && (m != 0 || s != 0 || frac != 0)) {
                return Err(invalid(name));
            }
            value.nanos = i128::from(h) * NANOS_PER_HOUR
                + i128::from(m) * NANOS_PER_MINUTE
                + i128::from(s) * NANOS_PER_SECOND
                + frac;
            next += 4;
        }
        value.tz = timezone(get(next))?;
        if value.nanos == NANOS_PER_DAY {
            value.nanos = 0;
            if kind == TemporalType::DateTime {
                value.shift_days(1);
            }
        }
        Ok(value)
    }

    fn parse_duration(kind: TemporalType, text: &str) -> Result<Self, Diagnostic> {
        let caps = DURATION.captures(text).ok_or_else(|| invalid("duration"))?;
        let get = |i: usize| caps.get(i).map(|m| m.as_str());
        let any =
            |range: std::ops::RangeInclusive<usize>| range.into_iter().any(|i| get(i).is_some());
        let part = |i: usize| -> Result<i128, Diagnostic> {
            get(i)
                .map_or(Ok(0), |s| s.parse::<i128>())
                .map_err(|_| invalid("duration"))
        };
        if !any(2..=7) || text.ends_with('T') {
            return Err(invalid("duration"));
        }
        let sign: i128 = if get(1).is_some() { -1 } else { 1 };
        let months = sign * (part(2)? * 12 + part(3)?);
        let nanos = sign
            * (part(4)? * NANOS_PER_DAY
                + part(5)? * NANOS_PER_HOUR
                + part(6)? * NANOS_PER_MINUTE
                + part(7)? * NANOS_PER_SECOND
                + fraction(get(8)));
        let valid = match kind {
            TemporalType::YearMonthDuration => !any(4..=7),
            TemporalType::DayTimeDuration => !any(2..=3),
            _ => true,
        };
        if !valid {
            return Err(invalid(kind.name()));
        }
        Ok(Self {
            months: i64::try_from(months).map_err(|_| invalid("duration"))?,
            nanos,
            ..Self::moment(kind)
        })
    }

    fn shift_days(&mut self, days: i64) {
        let (year, month, day) =
            civil_from_days(days_from_civil(self.year, self.month, self.day) + days);
        (self.year, self.month, self.day) = (year, month, day);
    }

    pub fn string(&self) -> String {
        match self.kind {
            TemporalType::Date => format!("{}{}", self.date_string(), self.tz_string()),
            TemporalType::Time => format!("{}{}", self.time_string(), self.tz_string()),
            TemporalType::DateTime => format!(
                "{}T{}{}",
                self.date_string(),
                self.time_string(),
                self.tz_string()
            ),
            _ => self.duration_string(),
        }
    }

    fn date_string(&self) -> String {
        let sign = if self.year < 0 { "-" } else { "" };
        format!(
            "{sign}{:04}-{:02}-{:02}",
            self.year.unsigned_abs(),
            self.month,
            self.day
        )
    }

    fn time_string(&self) -> String {
        let (h, m, s, frac) = (
            self.nanos / NANOS_PER_HOUR,
            self.nanos / NANOS_PER_MINUTE % 60,
            self.nanos / NANOS_PER_SECOND % 60,
            self.nanos % NANOS_PER_SECOND,
        );
        let mut out = format!("{h:02}:{m:02}:{s:02}");
        if frac != 0 {
            out.push('.');
            out.push_str(format!("{frac:09}").trim_end_matches('0'));
        }
        out
    }

    fn tz_string(&self) -> String {
        match self.tz {
            None => String::new(),
            Some(0) => "Z".into(),
            Some(tz) => format!(
                "{}{:02}:{:02}",
                if tz < 0 { '-' } else { '+' },
                tz.abs() / 60,
                tz.abs() % 60
            ),
        }
    }

    fn duration_string(&self) -> String {
        if self.months == 0 && self.nanos == 0 {
            return if self.kind == TemporalType::YearMonthDuration {
                "P0M"
            } else {
                "PT0S"
            }
            .into();
        }
        let negative = self.months < 0 || self.nanos < 0;
        let months = self.months.unsigned_abs();
        let nanos = self.nanos.unsigned_abs() as i128;
        let mut out = String::from(if negative { "-P" } else { "P" });
        fn push(out: &mut String, value: i128, unit: char) {
            if value != 0 {
                out.push_str(&format!("{value}{unit}"));
            }
        }
        push(&mut out, i128::from(months / 12), 'Y');
        push(&mut out, i128::from(months % 12), 'M');
        push(&mut out, nanos / NANOS_PER_DAY, 'D');
        let (h, m) = (nanos / NANOS_PER_HOUR % 24, nanos / NANOS_PER_MINUTE % 60);
        let (s, frac) = (nanos / NANOS_PER_SECOND % 60, nanos % NANOS_PER_SECOND);
        if h != 0 || m != 0 || s != 0 || frac != 0 {
            out.push('T');
            push(&mut out, h, 'H');
            push(&mut out, m, 'M');
            if s != 0 || frac != 0 {
                out.push_str(&s.to_string());
                if frac != 0 {
                    out.push('.');
                    out.push_str(format!("{frac:09}").trim_end_matches('0'));
                }
                out.push('S');
            }
        }
        out
    }

    /// Instant in nanoseconds on the proleptic calendar, with absent time zones
    /// read as UTC (the implicit time zone).
    fn instant(&self) -> i128 {
        i128::from(days_from_civil(self.year, self.month, self.day)) * NANOS_PER_DAY + self.nanos
            - i128::from(self.tz.unwrap_or(0)) * NANOS_PER_MINUTE
    }

    /// Value ordering; `None` when the two kinds are not comparable.
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        if self.kind.is_duration() && other.kind.is_duration() {
            return Some((self.months, self.nanos).cmp(&(other.months, other.nanos)));
        }
        (self.kind == other.kind && !self.kind.is_duration())
            .then(|| self.instant().cmp(&other.instant()))
    }

    pub fn cast(&self, target: TemporalType) -> Result<Self, Diagnostic> {
        let bad = || {
            Diagnostic::new(
                "XPTY0004",
                "temporal value cannot be cast to this type",
                "run",
            )
        };
        if self.kind == target {
            return Ok(self.clone());
        }
        let mut value = self.clone();
        value.kind = target;
        match (self.kind, target) {
            (TemporalType::DateTime, TemporalType::Date) => value.nanos = 0,
            (TemporalType::DateTime, TemporalType::Time) => {
                (value.year, value.month, value.day) = REFERENCE_DAY;
            }
            (TemporalType::Date, TemporalType::DateTime) => {}
            (from, to) if from.is_duration() && to.is_duration() => match to {
                TemporalType::YearMonthDuration => value.nanos = 0,
                TemporalType::DayTimeDuration => value.months = 0,
                _ => {}
            },
            _ => return Err(bad()),
        }
        Ok(value)
    }

    /// The execution-wide current instant as a dateTime in UTC.
    pub fn now() -> Self {
        let since = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i128);
        let days = since.div_euclid(NANOS_PER_DAY) as i64;
        let (year, month, day) = civil_from_days(days);
        Self {
            kind: TemporalType::DateTime,
            year,
            month,
            day,
            nanos: since.rem_euclid(NANOS_PER_DAY),
            tz: Some(0),
            months: 0,
        }
    }

    pub fn implicit_timezone() -> Self {
        Self {
            kind: TemporalType::DayTimeDuration,
            ..Self::moment(TemporalType::DayTimeDuration)
        }
    }

    pub fn timezone(&self) -> Option<Self> {
        self.tz.map(|tz| Self {
            kind: TemporalType::DayTimeDuration,
            nanos: i128::from(tz) * NANOS_PER_MINUTE,
            ..Self::moment(TemporalType::DayTimeDuration)
        })
    }

    /// Integer component by accessor name (`year`, `month`, `day`, `hours`,
    /// `minutes`, `years`, `months`, `days`) or the seconds with nanoseconds.
    pub fn component(&self, name: &str) -> Option<i128> {
        let n = self.nanos;
        Some(match (self.kind.is_duration(), name) {
            (false, "year") => self.year.into(),
            (false, "month") => self.month.into(),
            (false, "day") => self.day.into(),
            (false, "hours") => n / NANOS_PER_HOUR,
            (false, "minutes") => n / NANOS_PER_MINUTE % 60,
            (true, "years") => i128::from(self.months / 12),
            (true, "months") => i128::from(self.months % 12),
            (true, "days") => n / NANOS_PER_DAY,
            (true, "hours") => n % NANOS_PER_DAY / NANOS_PER_HOUR,
            (true, "minutes") => n % NANOS_PER_HOUR / NANOS_PER_MINUTE,
            _ => return None,
        })
    }

    /// Seconds including the fraction, as decimal text (sign preserved).
    pub fn seconds(&self) -> String {
        let rem = self.nanos % NANOS_PER_MINUTE;
        let sign = if rem < 0 { "-" } else { "" };
        let rem = rem.abs();
        let frac = rem % NANOS_PER_SECOND;
        if frac == 0 {
            format!("{sign}{}", rem / NANOS_PER_SECOND)
        } else {
            format!(
                "{sign}{}.{}",
                rem / NANOS_PER_SECOND,
                format!("{frac:09}").trim_end_matches('0')
            )
        }
    }
}
