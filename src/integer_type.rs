//! XML Schema integer restriction hierarchy and value-space checks.
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegerType {
    NonPositive,
    Negative,
    Long,
    Int,
    Short,
    Byte,
    NonNegative,
    UnsignedLong,
    UnsignedInt,
    UnsignedShort,
    UnsignedByte,
    Positive,
}
impl IntegerType {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "nonPositiveInteger" => Self::NonPositive,
            "negativeInteger" => Self::Negative,
            "long" => Self::Long,
            "int" => Self::Int,
            "short" => Self::Short,
            "byte" => Self::Byte,
            "nonNegativeInteger" => Self::NonNegative,
            "positiveInteger" => Self::Positive,
            "unsignedLong" => Self::UnsignedLong,
            "unsignedInt" => Self::UnsignedInt,
            "unsignedShort" => Self::UnsignedShort,
            "unsignedByte" => Self::UnsignedByte,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::NonPositive => "nonPositiveInteger",
            Self::Negative => "negativeInteger",
            Self::Long => "long",
            Self::Int => "int",
            Self::Short => "short",
            Self::Byte => "byte",
            Self::NonNegative => "nonNegativeInteger",
            Self::Positive => "positiveInteger",
            Self::UnsignedLong => "unsignedLong",
            Self::UnsignedInt => "unsignedInt",
            Self::UnsignedShort => "unsignedShort",
            Self::UnsignedByte => "unsignedByte",
        }
    }
    pub fn subtype_of(self, target: Self) -> bool {
        let mut current = Some(self);
        while let Some(kind) = current {
            if kind == target {
                return true;
            }
            current = match kind {
                Self::Negative => Some(Self::NonPositive),
                Self::Int => Some(Self::Long),
                Self::Short => Some(Self::Int),
                Self::Byte => Some(Self::Short),
                Self::UnsignedLong | Self::Positive => Some(Self::NonNegative),
                Self::UnsignedInt => Some(Self::UnsignedLong),
                Self::UnsignedShort => Some(Self::UnsignedInt),
                Self::UnsignedByte => Some(Self::UnsignedShort),
                _ => None,
            };
        }
        false
    }
    pub fn accepts(self, value: &BigInt) -> bool {
        match self {
            Self::NonPositive => !value.is_positive(),
            Self::Negative => value.is_negative(),
            Self::NonNegative => !value.is_negative(),
            Self::Positive => value.is_positive(),
            Self::Long => value.to_i64().is_some(),
            Self::Int => value.to_i32().is_some(),
            Self::Short => value.to_i16().is_some(),
            Self::Byte => value.to_i8().is_some(),
            Self::UnsignedLong => value.to_u64().is_some(),
            Self::UnsignedInt => value.to_u32().is_some(),
            Self::UnsignedShort => value.to_u16().is_some(),
            Self::UnsignedByte => value.to_u8().is_some(),
        }
    }
}
