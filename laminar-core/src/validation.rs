//! Minimal address validation for the tracer bullet.

use thiserror::Error;

/// Max allowed memo length in bytes (UTF-8).
pub const MAX_MEMO_BYTES: usize = 512;

/// Validation errors for recipient addresses.
#[derive(Debug, Error, Clone)]
pub enum AddressValidationError {
    #[error("address is empty")]
    Empty,
    #[error("address does not match allowed prefixes (expected 'u1' or 't1')")]
    InvalidPrefix,
}

/// Validation errors for memo fields.
#[derive(Debug, Error, Clone)]
pub enum MemoValidationError {
    #[error("E1004 MEMO_TOO_LONG: memo exceeds {limit} bytes (got {actual})")]
    TooLong { limit: usize, actual: usize },
}

/// Stub validation: ensures the address is present and uses known prefixes.
pub fn validate_address(addr: &str) -> Result<(), AddressValidationError> {
    let s = addr.trim();
    if s.is_empty() {
        return Err(AddressValidationError::Empty);
    }
    if s.starts_with("u1") || s.starts_with("t1") {
        Ok(())
    } else {
        Err(AddressValidationError::InvalidPrefix)
    }
}

/// Enforce memo length limits (UTF-8 byte count).
pub fn validate_memo(memo: &str) -> Result<(), MemoValidationError> {
    let len = memo.as_bytes().len();
    if len > MAX_MEMO_BYTES {
        Err(MemoValidationError::TooLong {
            limit: MAX_MEMO_BYTES,
            actual: len,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_prefix_accepts_u1() {
        assert!(validate_address("u1abc").is_ok());
    }

    #[test]
    fn address_prefix_accepts_t1() {
        assert!(validate_address("t1abc").is_ok());
    }

    #[test]
    fn address_rejects_other_prefix() {
        assert!(validate_address("x1abc").is_err());
    }

    #[test]
    fn address_rejects_empty() {
        assert!(validate_address("   ").is_err());
    }

    #[test]
    fn memo_allows_empty() {
        assert!(validate_memo("").is_ok());
    }

    #[test]
    fn memo_allows_512_bytes_ascii() {
        let memo = "a".repeat(MAX_MEMO_BYTES);
        assert!(validate_memo(&memo).is_ok());
    }

    #[test]
    fn memo_rejects_513_bytes_ascii() {
        let memo = "a".repeat(MAX_MEMO_BYTES + 1);
        assert!(validate_memo(&memo).is_err());
    }

    #[test]
    fn memo_allows_512_bytes_utf8() {
        let memo = "\u{1F600}".repeat(128);
        assert_eq!(memo.as_bytes().len(), MAX_MEMO_BYTES);
        assert!(validate_memo(&memo).is_ok());
    }

    #[test]
    fn memo_rejects_513_bytes_utf8() {
        let memo = "\u{1F600}".repeat(129);
        assert!(memo.as_bytes().len() > MAX_MEMO_BYTES);
        assert!(validate_memo(&memo).is_err());
    }
}
