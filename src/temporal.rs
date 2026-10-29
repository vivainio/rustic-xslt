//! XML Schema date, time and duration value spaces: strict lexical parsing,
//! canonical strings, ordering, component access and calendar normalization.
use crate::Diagnostic;
use regex::Regex;
use std::{cmp::Ordering, sync::LazyLock};

const MAX_YEAR: u64 = 1_000_000_000;
const MAX_COMPONENT: i128 = 1_000_000_000_000_000_000;
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
            if value.year.unsigned_abs() > MAX_YEAR {
                return Err(Diagnostic::new("FODT0001", "year is out of range", "run"));
            }
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
                .ok()
                .filter(|n| *n < MAX_COMPONENT)
                .ok_or_else(|| Diagnostic::new("FODT0002", "duration component overflow", "run"))
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

    fn type_error() -> Diagnostic {
        Diagnostic::new(
            "XPTY0004",
            "unsupported operand types for date, time or duration arithmetic",
            "run",
        )
    }

    fn checked_year(self) -> Result<Self, Diagnostic> {
        if self.year.unsigned_abs() > MAX_YEAR {
            return Err(Diagnostic::new(
                "FODT0001",
                "date arithmetic overflow",
                "run",
            ));
        }
        Ok(self)
    }

    fn add_months(&self, months: i64) -> Result<Self, Diagnostic> {
        let mut value = self.clone();
        let total = self
            .year
            .checked_mul(12)
            .and_then(|y| y.checked_add(i64::from(self.month) - 1))
            .and_then(|m| m.checked_add(months))
            .ok_or_else(|| Diagnostic::new("FODT0001", "date arithmetic overflow", "run"))?;
        value.year = total.div_euclid(12);
        value.month = total.rem_euclid(12) as u8 + 1;
        value.day = value.day.min(days_in_month(value.year, value.month));
        value.checked_year()
    }

    fn add_nanos(&self, delta: i128) -> Result<Self, Diagnostic> {
        let mut value = self.clone();
        if self.kind == TemporalType::Time {
            value.nanos = (self.nanos + delta).rem_euclid(NANOS_PER_DAY);
            return Ok(value);
        }
        let local = i128::from(days_from_civil(self.year, self.month, self.day)) * NANOS_PER_DAY
            + self.nanos
            + delta;
        let days = i64::try_from(local.div_euclid(NANOS_PER_DAY))
            .map_err(|_| Diagnostic::new("FODT0001", "date arithmetic overflow", "run"))?;
        if days.unsigned_abs() > MAX_YEAR * 366 {
            return Err(Diagnostic::new(
                "FODT0001",
                "date arithmetic overflow",
                "run",
            ));
        }
        (value.year, value.month, value.day) = civil_from_days(days);
        value.nanos = if self.kind == TemporalType::Date {
            0
        } else {
            local.rem_euclid(NANOS_PER_DAY)
        };
        value.checked_year()
    }

    fn duration_sum(&self, rhs: &Self, sign: i64) -> Result<Self, Diagnostic> {
        let overflow = || Diagnostic::new("FODT0002", "duration overflow", "run");
        Ok(Self {
            months: self
                .months
                .checked_add(sign.checked_mul(rhs.months).ok_or_else(overflow)?)
                .ok_or_else(overflow)?,
            nanos: self.nanos + i128::from(sign) * rhs.nanos,
            ..self.clone()
        })
    }

    pub fn add(&self, rhs: &Self) -> Result<Self, Diagnostic> {
        use TemporalType::*;
        match (self.kind, rhs.kind) {
            (YearMonthDuration, YearMonthDuration) | (DayTimeDuration, DayTimeDuration) => {
                self.duration_sum(rhs, 1)
            }
            (DateTime | Date, YearMonthDuration) => self.add_months(rhs.months),
            (DateTime | Date | Time, DayTimeDuration) => self.add_nanos(rhs.nanos),
            (YearMonthDuration | DayTimeDuration, DateTime | Date | Time) => rhs.add(self),
            _ => Err(Self::type_error()),
        }
    }

    pub fn sub(&self, rhs: &Self) -> Result<Self, Diagnostic> {
        use TemporalType::*;
        match (self.kind, rhs.kind) {
            (YearMonthDuration, YearMonthDuration) | (DayTimeDuration, DayTimeDuration) => {
                self.duration_sum(rhs, -1)
            }
            (DateTime | Date, YearMonthDuration) => self.add_months(-rhs.months),
            (DateTime | Date | Time, DayTimeDuration) => self.add_nanos(-rhs.nanos),
            (a, b) if a == b && matches!(a, DateTime | Date | Time) => Ok(Self {
                kind: DayTimeDuration,
                nanos: self.instant() - rhs.instant(),
                ..Self::moment(DayTimeDuration)
            }),
            _ => Err(Self::type_error()),
        }
    }

    /// Multiplies (or divides) a yearMonthDuration or dayTimeDuration by a number.
    pub fn scale(&self, factor: f64, divide: bool) -> Result<Self, Diagnostic> {
        use TemporalType::*;
        if !matches!(self.kind, YearMonthDuration | DayTimeDuration) {
            return Err(Self::type_error());
        }
        if factor.is_nan() {
            return Err(Diagnostic::new(
                "FOCA0005",
                "NaN supplied as duration factor",
                "run",
            ));
        }
        let factor = if divide { 1.0 / factor } else { factor };
        let overflow = || Diagnostic::new("FODT0002", "duration overflow", "run");
        if divide && factor.is_infinite() {
            return Err(Diagnostic::new(
                "FODT0002",
                "division of a duration by zero",
                "run",
            ));
        }
        // Rounds half toward positive infinity, like fn:round.
        let months = (self.months as f64 * factor + 0.5).floor();
        let nanos = (self.nanos as f64 * factor + 0.5).floor();
        if !months.is_finite()
            || !nanos.is_finite()
            || months.abs() > 9.0e18
            || nanos.abs() > 1.0e38
        {
            return Err(overflow());
        }
        Ok(Self {
            months: months as i64,
            nanos: nanos as i128,
            ..self.clone()
        })
    }

    /// Ratio of two durations of the same subtype, as decimal text.
    pub fn ratio(&self, rhs: &Self) -> Result<(i128, i128), Diagnostic> {
        use TemporalType::*;
        let (a, b) = match (self.kind, rhs.kind) {
            (YearMonthDuration, YearMonthDuration) => {
                (i128::from(self.months), i128::from(rhs.months))
            }
            (DayTimeDuration, DayTimeDuration) => (self.nanos, rhs.nanos),
            _ => return Err(Self::type_error()),
        };
        if b == 0 {
            return Err(Diagnostic::new(
                "FOAR0001",
                "division of a duration by zero",
                "run",
            ));
        }
        Ok((a, b))
    }

    /// Moves to the given time zone: values without a zone keep their local
    /// fields and gain the zone, values with a zone are shifted.
    pub fn adjust(&self, tz: Option<i32>) -> Result<Self, Diagnostic> {
        let mut value = self.clone();
        value.tz = tz;
        match (self.tz, tz) {
            (Some(old), Some(new)) => {
                let delta = i128::from(new - old) * NANOS_PER_MINUTE;
                let mut shifted = self.add_nanos(delta)?;
                shifted.tz = tz;
                Ok(shifted)
            }
            _ => Ok(value),
        }
    }

    /// Combines a date and a time, rejecting conflicting time zones.
    pub fn combine(date: &Self, time: &Self) -> Result<Self, Diagnostic> {
        let tz = match (date.tz, time.tz) {
            (Some(a), Some(b)) if a != b => {
                return Err(Diagnostic::new(
                    "FORG0008",
                    "date and time have different time zones",
                    "run",
                ));
            }
            (a, b) => a.or(b),
        };
        Ok(Self {
            kind: TemporalType::DateTime,
            nanos: time.nanos,
            tz,
            ..date.clone()
        })
    }

    /// Duration with a whole-minute zone offset, if it is a valid zone.
    pub fn as_timezone(&self) -> Result<i32, Diagnostic> {
        let bad = || Diagnostic::new("FODT0003", "invalid time zone duration", "run");
        if self.kind != TemporalType::DayTimeDuration
            || self.nanos % NANOS_PER_MINUTE != 0
            || self.nanos.abs() > 14 * NANOS_PER_HOUR
        {
            return Err(bad());
        }
        Ok((self.nanos / NANOS_PER_MINUTE) as i32)
    }
}
