//! Minimal address validation for the tracer bullet.

use thiserror::Error;

/// Validation errors for recipient addresses.
#[derive(Debug, Error, Clone)]
pub enum AddressValidationError {
    #[error("address is empty")]
    Empty,
    #[error("address does not match allowed prefixes (expected 'u1' or 't1')")]
    InvalidPrefix,
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
}
