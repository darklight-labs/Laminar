//! Minimal address validation for the tracer bullet.

use crate::types::Network;
use thiserror::Error;

/// Max allowed memo length in bytes (UTF-8).
pub const MAX_MEMO_BYTES: usize = 512;

const MAINNET_PREFIXES: [&str; 2] = ["u1", "t1"];
const TESTNET_PREFIXES: [&str; 2] = ["utest1", "tm"];

/// Validation errors for recipient addresses.
#[derive(Debug, Error, Clone)]
pub enum AddressValidationError {
    #[error("address is empty")]
    Empty,
    #[error("address contains invalid characters (ASCII letters and digits only)")]
    InvalidCharacters,
    #[error(
        "address does not match allowed prefixes (mainnet: 'u1'/'t1'; testnet: 'utest1'/'tm')"
    )]
    InvalidPrefix,
    #[error("address does not match selected network '{expected}'")]
    NetworkMismatch { expected: &'static str },
}

/// Validation errors for memo fields.
#[derive(Debug, Error, Clone)]
pub enum MemoValidationError {
    #[error("E1004 MEMO_TOO_LONG: memo exceeds {limit} bytes (got {actual})")]
    TooLong { limit: usize, actual: usize },
}

fn has_any_prefix(addr: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| addr.starts_with(prefix))
}

/// Stub validation: ensures the address is present and uses known prefixes for the selected network.
pub fn validate_address(addr: &str, network: Network) -> Result<(), AddressValidationError> {
    let s = addr.trim();
    if s.is_empty() {
        return Err(AddressValidationError::Empty);
    }

    if !s.is_ascii() || !s.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(AddressValidationError::InvalidCharacters);
    }

    let is_mainnet = has_any_prefix(s, &MAINNET_PREFIXES);
    let is_testnet = has_any_prefix(s, &TESTNET_PREFIXES);

    if !is_mainnet && !is_testnet {
        return Err(AddressValidationError::InvalidPrefix);
    }

    match network {
        Network::Mainnet if is_mainnet => Ok(()),
        Network::Testnet if is_testnet => Ok(()),
        Network::Mainnet => Err(AddressValidationError::NetworkMismatch {
            expected: "mainnet",
        }),
        Network::Testnet => Err(AddressValidationError::NetworkMismatch {
            expected: "testnet",
        }),
    }
}

/// Enforce memo length limits (UTF-8 byte count).
pub fn validate_memo(memo: &str) -> Result<(), MemoValidationError> {
    let len = memo.len();
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
        assert!(validate_address("u1abc", Network::Mainnet).is_ok());
    }

    #[test]
    fn address_prefix_accepts_t1() {
        assert!(validate_address("t1abc", Network::Mainnet).is_ok());
    }

    #[test]
    fn address_prefix_accepts_utest1_on_testnet() {
        assert!(validate_address("utest1abc", Network::Testnet).is_ok());
    }

    #[test]
    fn address_prefix_accepts_tm_on_testnet() {
        assert!(validate_address("tmabc", Network::Testnet).is_ok());
    }

    #[test]
    fn address_rejects_other_prefix() {
        assert!(matches!(
            validate_address("x1abc", Network::Mainnet),
            Err(AddressValidationError::InvalidPrefix)
        ));
    }

    #[test]
    fn address_rejects_network_mismatch() {
        assert!(matches!(
            validate_address("u1abc", Network::Testnet),
            Err(AddressValidationError::NetworkMismatch { .. })
        ));
    }

    #[test]
    fn address_rejects_non_ascii_characters() {
        let han = "\u{4F60}";
        assert!(matches!(
            validate_address(&format!("u1{han}{han}{han}"), Network::Mainnet),
            Err(AddressValidationError::InvalidCharacters)
        ));
    }

    #[test]
    fn address_rejects_empty() {
        assert!(matches!(
            validate_address("   ", Network::Mainnet),
            Err(AddressValidationError::Empty)
        ));
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
