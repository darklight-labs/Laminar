use std::fmt::{self, Display};
use std::iter::Sum;
use std::ops::Add;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AmountError;

pub const ZATOSHI_PER_ZEC: u64 = 100_000_000;
pub const ZATOSHI_MIN: u64 = 1;
pub const ZATOSHI_MAX: u64 = 2_100_000_000_000_000;
pub const DUST_THRESHOLD: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct Zatoshi(u64);

impl Zatoshi {
    pub fn new(value: u64) -> Result<Self, AmountError> {
        if value < ZATOSHI_MIN {
            return Err(AmountError::BelowMinimum);
        }
        if value > ZATOSHI_MAX {
            return Err(AmountError::AboveMaximum { value });
        }

        Ok(Self(value))
    }

    pub fn from_zec_str(input: &str) -> Result<Self, AmountError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AmountError::EmptyInput);
        }
        if trimmed.starts_with('-') {
            return Err(AmountError::NegativeNotAllowed);
        }

        let mut parts = trimmed.split('.');
        let whole_str = parts.next().ok_or(AmountError::InvalidFormat)?;
        let frac_str = parts.next();
        if parts.next().is_some() {
            return Err(AmountError::InvalidFormat);
        }
        if whole_str.is_empty() {
            return Err(AmountError::InvalidFormat);
        }
        if !whole_str.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(AmountError::InvalidNumeric);
        }

        let whole = parse_u64_digits(whole_str)?;
        let whole_zat = whole
            .checked_mul(ZATOSHI_PER_ZEC)
            .ok_or(AmountError::Overflow)?;

        let frac_zat = match frac_str {
            None => 0,
            Some(fraction) => {
                if fraction.len() > 8 {
                    return Err(AmountError::TooManyDecimals {
                        decimals: fraction.len(),
                    });
                }
                if !fraction.is_empty() && !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(AmountError::InvalidNumeric);
                }

                let mut padded = fraction.to_string();
                while padded.len() < 8 {
                    padded.push('0');
                }
                if padded.is_empty() {
                    padded.push('0');
                }
                parse_u64_digits(&padded)?
            }
        };

        let combined = whole_zat
            .checked_add(frac_zat)
            .ok_or(AmountError::Overflow)?;
        Self::new(combined)
    }

    pub fn to_zec_string(&self) -> String {
        let whole = self.0 / ZATOSHI_PER_ZEC;
        let frac = self.0 % ZATOSHI_PER_ZEC;
        if frac == 0 {
            return whole.to_string();
        }

        let mut frac_str = format!("{frac:08}");
        while frac_str.ends_with('0') {
            frac_str.pop();
        }
        format!("{whole}.{frac_str}")
    }

    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn checked_add(self, other: Zatoshi) -> Result<Zatoshi, AmountError> {
        let sum = self.0.checked_add(other.0).ok_or(AmountError::Overflow)?;
        Zatoshi::new(sum)
    }
}

impl TryFrom<u64> for Zatoshi {
    type Error = AmountError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Zatoshi> for u64 {
    fn from(value: Zatoshi) -> Self {
        value.0
    }
}

impl Display for Zatoshi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_zec_string())
    }
}

impl Add for Zatoshi {
    type Output = Zatoshi;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
            .expect("zatoshi addition overflowed or exceeded maximum supply")
    }
}

impl Sum for Zatoshi {
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let first = iter
            .next()
            .expect("cannot sum an empty iterator of Zatoshi values");
        iter.fold(first, |acc, value| acc + value)
    }
}

impl<'a> Sum<&'a Zatoshi> for Zatoshi {
    fn sum<I: Iterator<Item = &'a Zatoshi>>(mut iter: I) -> Self {
        let first = *iter
            .next()
            .expect("cannot sum an empty iterator of Zatoshi values");
        iter.fold(first, |acc, value| acc + *value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ZatoshiString(String);

impl ZatoshiString {
    pub fn new(value: impl Into<String>) -> Result<Self, AmountError> {
        let value = value.into();
        if is_valid_zatoshi_integer_string(&value) {
            Ok(Self(value))
        } else {
            Err(AmountError::InvalidZatoshiString { value })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ZatoshiString {
    type Error = AmountError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ZatoshiString> for String {
    fn from(value: ZatoshiString) -> Self {
        value.0
    }
}

impl From<Zatoshi> for ZatoshiString {
    fn from(value: Zatoshi) -> Self {
        Self(value.as_u64().to_string())
    }
}

impl TryFrom<ZatoshiString> for Zatoshi {
    type Error = AmountError;

    fn try_from(value: ZatoshiString) -> Result<Self, Self::Error> {
        let parsed = parse_u64_digits(value.as_str())?;
        Zatoshi::new(parsed)
    }
}

impl TryFrom<&ZatoshiString> for Zatoshi {
    type Error = AmountError;

    fn try_from(value: &ZatoshiString) -> Result<Self, Self::Error> {
        let parsed = parse_u64_digits(value.as_str())?;
        Zatoshi::new(parsed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipient {
    pub address: String,
    pub amount: Zatoshi,
    pub memo: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionIntent {
    pub schema_version: String,
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub network: Network,
    pub recipients: Vec<Recipient>,
    pub total_zat: Zatoshi,
    pub zip321_uri: String,
    pub payload_bytes: usize,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchConfig {
    pub network: Network,
    pub max_recipients: usize,
    pub source_file: String,
}

fn parse_u64_digits(input: &str) -> Result<u64, AmountError> {
    if !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AmountError::InvalidNumeric);
    }
    input.parse::<u64>().map_err(|_| AmountError::Overflow)
}

fn is_valid_zatoshi_integer_string(input: &str) -> bool {
    if input.is_empty() || !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    if input == "0" {
        return true;
    }
    !input.starts_with('0')
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{AmountError, Zatoshi, ZatoshiString, ZATOSHI_MAX, ZATOSHI_MIN, ZATOSHI_PER_ZEC};

    #[test]
    fn zatoshi_creation_valid_values() {
        assert_eq!(Zatoshi::new(1).unwrap().as_u64(), 1);
        assert_eq!(Zatoshi::new(100_000_000).unwrap().as_u64(), 100_000_000);
        assert_eq!(Zatoshi::new(ZATOSHI_MAX).unwrap().as_u64(), ZATOSHI_MAX);
    }

    #[test]
    fn zatoshi_creation_invalid_values() {
        assert!(matches!(Zatoshi::new(0), Err(AmountError::BelowMinimum)));
        assert!(matches!(
            Zatoshi::new(ZATOSHI_MAX + 1),
            Err(AmountError::AboveMaximum { value }) if value == ZATOSHI_MAX + 1
        ));
    }

    #[test]
    fn parses_valid_zec_strings() {
        assert_eq!(Zatoshi::from_zec_str("1").unwrap().as_u64(), 100_000_000);
        assert_eq!(Zatoshi::from_zec_str("1.0").unwrap().as_u64(), 100_000_000);
        assert_eq!(Zatoshi::from_zec_str("1.5").unwrap().as_u64(), 150_000_000);
        assert_eq!(Zatoshi::from_zec_str("0.00000001").unwrap().as_u64(), 1);
        assert_eq!(
            Zatoshi::from_zec_str("1.12345678").unwrap().as_u64(),
            112_345_678
        );
        assert_eq!(
            Zatoshi::from_zec_str("21000000").unwrap().as_u64(),
            ZATOSHI_MAX
        );
    }

    #[test]
    fn rejects_invalid_zec_strings() {
        assert!(matches!(
            Zatoshi::from_zec_str("0.123456789"),
            Err(AmountError::TooManyDecimals { .. })
        ));
        assert!(matches!(
            Zatoshi::from_zec_str("-1"),
            Err(AmountError::NegativeNotAllowed)
        ));
        assert!(matches!(
            Zatoshi::from_zec_str(""),
            Err(AmountError::EmptyInput)
        ));
        assert!(matches!(
            Zatoshi::from_zec_str("abc"),
            Err(AmountError::InvalidNumeric)
        ));
        assert!(matches!(
            Zatoshi::from_zec_str("21000000.00000001"),
            Err(AmountError::AboveMaximum { .. })
        ));
    }

    #[test]
    fn formats_zec_strings_without_trailing_zeros() {
        let one = Zatoshi::new(ZATOSHI_PER_ZEC).unwrap();
        let one_and_half = Zatoshi::new(150_000_000).unwrap();
        let min = Zatoshi::new(ZATOSHI_MIN).unwrap();

        assert_eq!(one.to_zec_string(), "1");
        assert_eq!(one_and_half.to_zec_string(), "1.5");
        assert_eq!(min.to_zec_string(), "0.00000001");
    }

    #[test]
    fn roundtrip_from_zec_string_to_zatoshi() {
        for value in 1_u64..=200_000_u64 {
            let z = Zatoshi::new(value).unwrap();
            let reparsed = Zatoshi::from_zec_str(&z.to_zec_string()).unwrap();
            assert_eq!(reparsed, z);
        }

        let near_max_start = ZATOSHI_MAX - 200_000;
        for value in near_max_start..=ZATOSHI_MAX {
            let z = Zatoshi::new(value).unwrap();
            let reparsed = Zatoshi::from_zec_str(&z.to_zec_string()).unwrap();
            assert_eq!(reparsed, z);
        }
    }

    proptest! {
        #[test]
        fn property_roundtrip_for_all_valid_zatoshi_values(value in ZATOSHI_MIN..=ZATOSHI_MAX) {
            let zatoshi = Zatoshi::new(value).expect("generated zatoshi must be valid");
            let reparsed = Zatoshi::from_zec_str(&zatoshi.to_zec_string())
                .expect("roundtrip parse should succeed");
            prop_assert_eq!(reparsed, zatoshi);
        }
    }

    #[test]
    fn checked_add_reports_overflow_or_range_error() {
        let max = Zatoshi::new(ZATOSHI_MAX).unwrap();
        let one = Zatoshi::new(1).unwrap();
        assert!(matches!(
            max.checked_add(one),
            Err(AmountError::AboveMaximum { value }) if value == ZATOSHI_MAX + 1
        ));
    }

    #[test]
    fn zatoshi_string_validates_integer_pattern() {
        assert!(ZatoshiString::new("0").is_ok());
        assert!(ZatoshiString::new("1").is_ok());
        assert!(ZatoshiString::new("123456").is_ok());
        assert!(ZatoshiString::new("01").is_err());
        assert!(ZatoshiString::new("-1").is_err());
        assert!(ZatoshiString::new("1.0").is_err());
        assert!(ZatoshiString::new("").is_err());
    }

    #[test]
    fn zatoshi_string_converts_to_and_from_zatoshi() {
        let z = Zatoshi::new(150_000_000).unwrap();
        let z_str = ZatoshiString::from(z);
        assert_eq!(z_str.as_str(), "150000000");

        let parsed = Zatoshi::try_from(z_str).unwrap();
        assert_eq!(parsed, z);
    }
}
