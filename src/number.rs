//! Exact integer/decimal arithmetic, with double promotion only when required.
use crate::Diagnostic;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use std::{cmp::Ordering, str::FromStr};
const MAX_DIGITS: usize = 10_000;
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Number {
    Integer(BigInt),
    Decimal(BigDecimal),
    Double(f64),
}
fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(code, message, "run")
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
                Ok(Self::Double(if kind == "float" {
                    (n as f32) as f64
                } else {
                    n
                }))
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
            _ => unreachable!(),
        }
    }
    pub fn string(&self) -> String {
        match self {
            Self::Integer(n) => n.to_string(),
            Self::Decimal(n) => n.normalized().to_plain_string(),
            Self::Double(n) => {
                if n.is_nan() {
                    "NaN".into()
                } else if *n == f64::INFINITY {
                    "INF".into()
                } else if *n == f64::NEG_INFINITY {
                    "-INF".into()
                } else if *n == 0.0 {
                    "0".into()
                } else {
                    n.to_string()
                }
            }
        }
    }
    pub fn double(&self) -> f64 {
        match self {
            Self::Double(n) => *n,
            Self::Integer(n) => n.to_f64().unwrap_or_else(|| {
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
            Self::Integer(n) => !n.is_zero(),
            Self::Decimal(n) => !n.is_zero(),
            Self::Double(n) => *n != 0.0 && !n.is_nan(),
        }
    }
    pub fn negate(&self) -> Self {
        match self {
            Self::Integer(n) => Self::Integer(-n),
            Self::Decimal(n) => Self::Decimal(-n),
            Self::Double(n) => Self::Double(-n),
        }
    }
    fn decimal(&self) -> Result<BigDecimal, Diagnostic> {
        match self {
            Self::Integer(n) => Ok(BigDecimal::from(n.clone())),
            Self::Decimal(n) => Ok(n.clone()),
            Self::Double(n) => BigDecimal::from_f64(*n)
                .ok_or_else(|| error("FOCA0002", "non-finite number cannot become decimal")),
        }
    }
    pub fn convert(&self, kind: &str) -> Result<Self, Diagnostic> {
        match kind {
            "integer" => {
                let d = self.decimal()?;
                Ok(Self::Integer(d.with_scale(0).into_bigint_and_scale().0))
            }
            "decimal" => Ok(Self::Decimal(self.decimal()?)),
            "double" => Ok(Self::Double(self.double())),
            "float" => Ok(Self::Double((self.double() as f32) as f64)),
            _ => unreachable!(),
        }
    }
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        if matches!(self, Self::Double(_)) || matches!(other, Self::Double(_)) {
            self.double().partial_cmp(&other.double())
        } else {
            Some(self.decimal().ok()?.cmp(&other.decimal().ok()?))
        }
    }
    pub fn arithmetic(&self, rhs: &Self, op: &str) -> Result<Self, Diagnostic> {
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
        let zero = !rhs.boolean();
        if zero && matches!(op, "div" | "mod") {
            return Err(error("FOAR0001", "division by zero"));
        }
        let result = match (self, rhs) {
            (Self::Integer(a), Self::Integer(b)) if op != "div" => Self::Integer(match op {
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
