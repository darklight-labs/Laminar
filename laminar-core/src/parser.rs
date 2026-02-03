//! ZEC decimal parsing into zatoshis with strict integer arithmetic.

use thiserror::Error;

/// Zatoshi conversion constant: 1 ZEC = 100,000,000 zatoshis.
pub const ZAT_PER_ZEC: u64 = 100_000_000;
/// Maximum supported supply in zatoshis.
pub const MAX_SUPPLY_ZAT: u64 = 21_000_000_u64 * ZAT_PER_ZEC;

#[derive(Debug, Error, Clone)]
pub enum ZecParseError {
    #[error("amount is empty")]
    Empty,
    #[error("amount contains a sign; negative/positive signs are not allowed")]
    SignNotAllowed,
    #[error("amount contains invalid characters")]
    InvalidCharacters,
    #[error("amount has more than one decimal point")]
    MultipleDecimalPoints,
    #[error("amount has more than 8 decimal places")]
    TooManyDecimals,
    #[error("amount has invalid digits")]
    InvalidDigits,
    #[error("amount exceeds maximum supply")]
    ExceedsMaximum,
    #[error("amount arithmetic overflow")]
    Overflow,
}

fn all_digits(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit())
}

fn parse_u64_digits(s: &str) -> Result<u64, ZecParseError> {
    if s.is_empty() {
        return Ok(0);
    }
    if !all_digits(s) {
        return Err(ZecParseError::InvalidDigits);
    }
    s.parse::<u64>().map_err(|_| ZecParseError::Overflow)
}

/// Parse a decimal ZEC string into zatoshis with no floating-point math.
pub fn parse_zec_to_zat(input: &str) -> Result<u64, ZecParseError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(ZecParseError::Empty);
    }

    if s.starts_with('-') || s.starts_with('+') {
        return Err(ZecParseError::SignNotAllowed);
    }

    for c in s.chars() {
        if !(c.is_ascii_digit() || c == '.') {
            return Err(ZecParseError::InvalidCharacters);
        }
    }

    if s.chars().filter(|c| *c == '.').count() > 1 {
        return Err(ZecParseError::MultipleDecimalPoints);
    }

    let mut iter = s.splitn(2, '.');
    let whole_str = iter.next().ok_or(ZecParseError::InvalidDigits)?;
    let frac_opt = iter.next();

    let whole = parse_u64_digits(whole_str)?;
    let whole_zat = whole
        .checked_mul(ZAT_PER_ZEC)
        .ok_or(ZecParseError::Overflow)?;

    let frac_zat = match frac_opt {
        None => 0_u64,
        Some(frac_str) => {
            if frac_str.len() > 8 {
                return Err(ZecParseError::TooManyDecimals);
            }
            if !frac_str.is_empty() && !all_digits(frac_str) {
                return Err(ZecParseError::InvalidDigits);
            }
            let mut padded = frac_str.to_string();
            while padded.len() < 8 {
                padded.push('0');
            }
            parse_u64_digits(&padded)?
        }
    };

    let total = whole_zat
        .checked_add(frac_zat)
        .ok_or(ZecParseError::Overflow)?;

    if total > MAX_SUPPLY_ZAT {
        return Err(ZecParseError::ExceedsMaximum);
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer_amount() {
        assert_eq!(parse_zec_to_zat("10").unwrap(), 1_000_000_000);
    }

    #[test]
    fn parses_decimal_amount() {
        assert_eq!(parse_zec_to_zat("1.5").unwrap(), 150_000_000);
    }

    #[test]
    fn parses_min_unit() {
        assert_eq!(parse_zec_to_zat("0.00000001").unwrap(), 1);
    }

    #[test]
    fn parses_leading_dot() {
        assert_eq!(parse_zec_to_zat(".5").unwrap(), 50_000_000);
    }

    #[test]
    fn parses_trailing_dot() {
        assert_eq!(parse_zec_to_zat("1.").unwrap(), 100_000_000);
    }

    #[test]
    fn rejects_too_many_decimals() {
        assert!(matches!(
            parse_zec_to_zat("0.000000001"),
            Err(ZecParseError::TooManyDecimals)
        ));
    }

    #[test]
    fn rejects_signs() {
        assert!(parse_zec_to_zat("-1").is_err());
        assert!(parse_zec_to_zat("+1").is_err());
    }

    #[test]
    fn rejects_invalid_chars() {
        assert!(parse_zec_to_zat("1,0").is_err());
        assert!(parse_zec_to_zat("abc").is_err());
    }

    #[test]
    fn accepts_max_supply() {
        assert_eq!(parse_zec_to_zat("21000000").unwrap(), MAX_SUPPLY_ZAT);
    }

    #[test]
    fn rejects_above_max_supply() {
        assert!(matches!(
            parse_zec_to_zat("21000001"),
            Err(ZecParseError::ExceedsMaximum)
        ));
    }
}
