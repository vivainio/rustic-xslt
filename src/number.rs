//! Exact integer/decimal arithmetic, with double promotion only when required.
use crate::Diagnostic;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};
use std::{cmp::Ordering, str::FromStr};
const MAX_DIGITS: usize = 10_000;
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Number {
    Integer(BigInt),
    DerivedInteger(crate::integer_type::IntegerType, BigInt),
    Decimal(BigDecimal),
    Double(f64),
    Float(f32),
}
fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(code, message, "run")
}
fn floating_string(value: impl std::fmt::Display + std::fmt::LowerExp, number: f64) -> String {
    if number.is_nan() {
        return "NaN".into();
    }
    if number == f64::INFINITY {
        return "INF".into();
    }
    if number == f64::NEG_INFINITY {
        return "-INF".into();
    }
    if number == 0.0 {
        return if number.is_sign_negative() { "-0" } else { "0" }.into();
    }
    if (0.000001..1000000.0).contains(&number.abs()) {
        return value.to_string();
    }
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap();
    format!(
        "{mantissa}{}E{exponent}",
        if mantissa.contains('.') { "" } else { ".0" }
    )
}
impl From<usize> for Number {
    fn from(n: usize) -> Self {
        Self::Integer(n.into())
    }
}
impl Number {
    pub fn literal(s: &str) -> Result<Self, Diagnostic> {
        if s.contains(['e', 'E']) {
            return Ok(Self::Double(
                s.parse()
                    .map_err(|_| error("XPST0003", "invalid double literal"))?,
            ));
        }
        Self::cast(
            if s.contains('.') {
                "decimal"
            } else {
                "integer"
            },
            s,
        )
    }
    pub fn cast(kind: &str, s: &str) -> Result<Self, Diagnostic> {
        let s = s.trim_matches([' ', '\t', '\r', '\n']);
        if s.len() > MAX_DIGITS {
            return Err(error("XPDY0130", "numeric digit limit exceeded"));
        }
        let invalid = || error("FORG0001", "invalid numeric lexical value");
        if let Some(kind) = crate::integer_type::IntegerType::parse(kind) {
            let Self::Integer(value) = Self::cast("integer", s)? else {
                unreachable!()
            };
            return if kind.accepts(&value) {
                Ok(Self::DerivedInteger(kind, value))
            } else {
                Err(invalid())
            };
        }
        match kind {
            "double" | "float" => {
                let n = match s {
                    "INF" | "+INF" => f64::INFINITY,
                    "-INF" => f64::NEG_INFINITY,
                    "NaN" => f64::NAN,
                    _ => {
                        if s.is_empty()
                            || !s.chars().all(|c| c.is_ascii_digit() || "+-.eE".contains(c))
                        {
                            return Err(invalid());
                        }
                        s.parse().map_err(|_| invalid())?
                    }
                };
                Ok(if kind == "float" {
                    Self::Float(n as f32)
                } else {
                    Self::Double(n)
                })
            }
            "integer" => BigInt::from_str(s)
                .map(Self::Integer)
                .map_err(|_| invalid()),
            "decimal" => {
                if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit() || "+-.".contains(c)) {
                    return Err(invalid());
                }
                BigDecimal::from_str(s)
                    .map(Self::Decimal)
                    .map_err(|_| invalid())
            }
            _ => Err(error("XPTY0004", "unsupported numeric cast target")),
        }
    }
    pub fn string(&self) -> String {
        match self {
            Self::Integer(n) | Self::DerivedInteger(_, n) => n.to_string(),
            Self::Decimal(n) => n.normalized().to_plain_string(),
            Self::Float(n) => floating_string(n, *n as f64),
            Self::Double(n) => floating_string(n, *n),
        }
    }
    pub fn legacy_string(&self) -> String {
        let value = match self {
            Self::Double(n) => *n,
            Self::Float(n) => *n as f64,
            _ => return self.string(),
        };
        if value == 0.0 {
            "0".into()
        } else if value == f64::INFINITY {
            "Infinity".into()
        } else if value == f64::NEG_INFINITY {
            "-Infinity".into()
        } else {
            match self {
                Self::Float(n) => n.to_string(),
                _ => value.to_string(),
            }
        }
    }
    pub fn double(&self) -> f64 {
        match self {
            Self::Double(n) => *n,
            Self::Float(n) => *n as f64,
            Self::Integer(n) | Self::DerivedInteger(_, n) => n.to_f64().unwrap_or_else(|| {
                if n < &BigInt::zero() {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            }),
            Self::Decimal(n) => n.to_f64().unwrap_or_else(|| {
                if n < &BigDecimal::zero() {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            }),
        }
    }
    pub fn boolean(&self) -> bool {
        match self {
            Self::Integer(n) | Self::DerivedInteger(_, n) => !n.is_zero(),
            Self::Decimal(n) => !n.is_zero(),
            Self::Double(n) => *n != 0.0 && !n.is_nan(),
            Self::Float(n) => *n != 0.0 && !n.is_nan(),
        }
    }
    pub fn negate(&self) -> Self {
        match self {
            Self::Integer(n) | Self::DerivedInteger(_, n) => Self::Integer(-n),
            Self::Decimal(n) => Self::Decimal(-n),
            Self::Double(n) => Self::Double(-n),
            Self::Float(n) => Self::Float(-n),
        }
    }
    /// The shortest decimal expansion of a finite number, for formatting.
    pub fn shortest_decimal(&self) -> Option<BigDecimal> {
        match self {
            Self::Double(n) if n.is_finite() => BigDecimal::from_str(&format!("{n}")).ok(),
            Self::Float(n) if n.is_finite() => BigDecimal::from_str(&format!("{n}")).ok(),
            Self::Double(_) | Self::Float(_) => None,
            other => other.decimal().ok(),
        }
    }
    fn decimal(&self) -> Result<BigDecimal, Diagnostic> {
        match self {
            Self::Integer(n) | Self::DerivedInteger(_, n) => Ok(BigDecimal::from(n.clone())),
            Self::Decimal(n) => Ok(n.clone()),
            Self::Float(n) => BigDecimal::from_f32(*n)
                .ok_or_else(|| error("FOCA0002", "non-finite number cannot become decimal")),
            Self::Double(n) => BigDecimal::from_f64(*n)
                .ok_or_else(|| error("FOCA0002", "non-finite number cannot become decimal")),
        }
    }
    pub fn convert(&self, kind: &str) -> Result<Self, Diagnostic> {
        if let Some(kind) = crate::integer_type::IntegerType::parse(kind) {
            let Self::Integer(value) = self.convert("integer")? else {
                unreachable!()
            };
            return if kind.accepts(&value) {
                Ok(Self::DerivedInteger(kind, value))
            } else {
                Err(error(
                    "FORG0001",
                    "numeric value is outside the integer subtype bounds",
                ))
            };
        }
        match kind {
            "integer" => {
                let d = self.decimal()?;
                Ok(Self::Integer(d.with_scale(0).into_bigint_and_scale().0))
            }
            "decimal" => Ok(Self::Decimal(self.decimal()?)),
            "double" => Ok(Self::Double(self.double())),
            "float" => Ok(Self::Float(self.double() as f32)),
            _ => unreachable!(),
        }
    }
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        if matches!(self, Self::Double(_)) || matches!(other, Self::Double(_)) {
            self.double().partial_cmp(&other.double())
        } else if matches!(self, Self::Float(_)) || matches!(other, Self::Float(_)) {
            (self.double() as f32).partial_cmp(&(other.double() as f32))
        } else {
            Some(self.decimal().ok()?.cmp(&other.decimal().ok()?))
        }
    }
    pub fn unary_function(&self, name: &str) -> Self {
        use bigdecimal::RoundingMode;
        match self {
            Self::Integer(n) | Self::DerivedInteger(_, n) => {
                Self::Integer(if name == "abs" { n.abs() } else { n.clone() })
            }
            Self::Decimal(n) => Self::Decimal(match name {
                "abs" => n.abs(),
                "floor" => n.with_scale_round(0, RoundingMode::Floor),
                "ceiling" => n.with_scale_round(0, RoundingMode::Ceiling),
                _ => unreachable!(),
            }),
            Self::Float(n) => Self::Float(match name {
                "abs" => n.abs(),
                "floor" => n.floor(),
                "ceiling" => n.ceil(),
                _ => unreachable!(),
            }),
            Self::Double(n) => Self::Double(match name {
                "abs" => n.abs(),
                "floor" => n.floor(),
                "ceiling" => n.ceil(),
                _ => unreachable!(),
            }),
        }
    }
    pub fn round(&self, precision: &BigInt, even: bool) -> Result<Self, Diagnostic> {
        if matches!(self, Self::Float(n) if !n.is_finite() || *n == 0.0)
            || matches!(self, Self::Double(n) if !n.is_finite() || *n == 0.0)
        {
            return Ok(self.clone());
        }
        let zero = || match self {
            Self::Integer(_) | Self::DerivedInteger(_, _) => Self::Integer(BigInt::from(0)),
            Self::Decimal(_) => Self::Decimal(BigDecimal::from(0)),
            Self::Float(n) => Self::Float(0.0f32.copysign(*n)),
            Self::Double(n) => Self::Double(0.0f64.copysign(*n)),
        };
        let Some(precision) = precision.to_i64() else {
            return Ok(if precision.is_negative() {
                zero()
            } else {
                self.clone()
            });
        };
        let value = self.decimal()?;
        if precision >= value.fractional_digit_count() {
            return Ok(self.clone());
        }
        if precision < -(MAX_DIGITS as i64) {
            return Ok(zero());
        }
        let mode = if even {
            bigdecimal::RoundingMode::HalfEven
        } else if value.is_negative() {
            bigdecimal::RoundingMode::HalfDown
        } else {
            bigdecimal::RoundingMode::HalfUp
        };
        let rounded = value.with_scale_round(precision, mode);
        let result = match self {
            Self::Integer(_) | Self::DerivedInteger(_, _) => {
                Self::Integer(rounded.with_scale(0).into_bigint_and_scale().0)
            }
            Self::Decimal(_) => Self::Decimal(rounded),
            Self::Float(n) => {
                let v = rounded.to_f32().unwrap_or_else(|| {
                    if rounded.is_negative() {
                        f32::NEG_INFINITY
                    } else {
                        f32::INFINITY
                    }
                });
                Self::Float(if v == 0.0 { 0.0f32.copysign(*n) } else { v })
            }
            Self::Double(n) => {
                let v = rounded.to_f64().unwrap_or_else(|| {
                    if rounded.is_negative() {
                        f64::NEG_INFINITY
                    } else {
                        f64::INFINITY
                    }
                });
                Self::Double(if v == 0.0 { 0.0f64.copysign(*n) } else { v })
            }
        };
        if result.string().len() > MAX_DIGITS {
            return Err(error("XPDY0130", "numeric digit limit exceeded"));
        }
        Ok(result)
    }
    fn integer_division(&self, rhs: &Self) -> Result<Self, Diagnostic> {
        let limit = || error("XPDY0130", "numeric digit limit exceeded");
        let quotient = if matches!(self, Self::Double(_) | Self::Float(_))
            || matches!(rhs, Self::Double(_) | Self::Float(_))
        {
            let double = matches!(self, Self::Double(_)) || matches!(rhs, Self::Double(_));
            let (a, b) = if double {
                (self.double(), rhs.double())
            } else {
                (self.double() as f32 as f64, rhs.double() as f32 as f64)
            };
            if b == 0.0 {
                return Err(error("FOAR0001", "integer division by zero"));
            }
            if a.is_nan() || b.is_nan() || a.is_infinite() {
                return Err(error("FOAR0002", "invalid integer division operand"));
            }
            let value = if double {
                a / b
            } else {
                ((a as f32) / (b as f32)) as f64
            };
            BigInt::from_f64(value.trunc())
                .ok_or_else(|| error("FOAR0002", "integer division overflow"))?
        } else {
            let (mut a, a_scale) = self.decimal()?.into_bigint_and_scale();
            let (mut b, b_scale) = rhs.decimal()?.into_bigint_and_scale();
            if b.is_zero() {
                return Err(error("FOAR0001", "integer division by zero"));
            }
            let scale = b_scale.checked_sub(a_scale).ok_or_else(limit)?;
            let magnitude = scale.unsigned_abs();
            if magnitude > MAX_DIGITS as u64 {
                return Err(limit());
            }
            let factor = BigInt::from(10).pow(magnitude as u32);
            if scale > 0 {
                a *= factor;
            } else {
                b *= factor;
            }
            a / b
        };
        if quotient.to_string().len() > MAX_DIGITS {
            return Err(limit());
        }
        Ok(Self::Integer(quotient))
    }
    pub fn arithmetic(&self, rhs: &Self, op: &str) -> Result<Self, Diagnostic> {
        if op == "idiv" {
            return self.integer_division(rhs);
        }
        if matches!(self, Self::Double(_)) || matches!(rhs, Self::Double(_)) {
            let (a, b) = (self.double(), rhs.double());
            return Ok(Self::Double(match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "div" => a / b,
                "mod" => a % b,
                _ => unreachable!(),
            }));
        }
        if matches!(self, Self::Float(_)) || matches!(rhs, Self::Float(_)) {
            let (a, b) = (self.double() as f32, rhs.double() as f32);
            return Ok(Self::Float(match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "div" => a / b,
                "mod" => a % b,
                _ => unreachable!(),
            }));
        }
        let zero = !rhs.boolean();
        if zero && matches!(op, "div" | "mod") {
            return Err(error("FOAR0001", "division by zero"));
        }
        let result = match (self, rhs) {
            (
                Self::Integer(a) | Self::DerivedInteger(_, a),
                Self::Integer(b) | Self::DerivedInteger(_, b),
            ) if op != "div" => Self::Integer(match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "mod" => a % b,
                _ => unreachable!(),
            }),
            _ => {
                let (a, b) = (self.decimal()?, rhs.decimal()?);
                Self::Decimal(match op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "div" => a / b,
                    "mod" => a % b,
                    _ => unreachable!(),
                })
            }
        };
        if result.string().len() > MAX_DIGITS {
            return Err(error("XPDY0130", "numeric digit limit exceeded"));
        }
        Ok(result)
    }
}
