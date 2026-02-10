use crate::error::{LaminarError, Result};
use crate::types::Zatoshi;

pub fn validate_address(address: &str) -> Result<()> {
    // TODO: use zcash_address parsing and network constraints.
    if address.trim().is_empty() {
        return Err(LaminarError::Validation {
            code: "E_ADDR_EMPTY",
            message: "address cannot be empty".to_string(),
        });
    }

    Ok(())
}

pub fn validate_amount(amount: Zatoshi) -> Result<()> {
    // TODO: enforce configurable min/max amount thresholds.
    if amount.0 == 0 {
        return Err(LaminarError::Validation {
            code: "E_AMOUNT_ZERO",
            message: "amount must be > 0".to_string(),
        });
    }

    Ok(())
}

pub fn validate_memo(memo: Option<&str>) -> Result<()> {
    // TODO: validate UTF-8 byte-length against memo protocol limits.
    if let Some(value) = memo {
        if value.len() > 512 {
            return Err(LaminarError::Validation {
                code: "E_MEMO_TOO_LONG",
                message: "memo exceeds 512 bytes".to_string(),
            });
        }
    }

    Ok(())
}
