use std::fmt;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxonomyCode {
    Validation1001,
    Validation1002,
    Validation1003,
    Validation1004,
    Validation1005,
    Validation1006,
    Validation1007,
    Validation1008,
    Validation1009,
    Validation1010,
    Validation1011,
    Validation1012,
    Validation1013,
    Cli2001,
    Cli2002,
    Cli2003,
    Cli2004,
    Cli2005,
    Storage3001,
    Storage3002,
    Storage3003,
    Handoff5003,
    Handoff5004,
    Handoff5005,
    Handoff5006,
    Handoff5007,
    Handoff5008,
    Generic9999,
}

impl TaxonomyCode {
    pub const ALL: [Self; 28] = [
        Self::Validation1001,
        Self::Validation1002,
        Self::Validation1003,
        Self::Validation1004,
        Self::Validation1005,
        Self::Validation1006,
        Self::Validation1007,
        Self::Validation1008,
        Self::Validation1009,
        Self::Validation1010,
        Self::Validation1011,
        Self::Validation1012,
        Self::Validation1013,
        Self::Cli2001,
        Self::Cli2002,
        Self::Cli2003,
        Self::Cli2004,
        Self::Cli2005,
        Self::Storage3001,
        Self::Storage3002,
        Self::Storage3003,
        Self::Handoff5003,
        Self::Handoff5004,
        Self::Handoff5005,
        Self::Handoff5006,
        Self::Handoff5007,
        Self::Handoff5008,
        Self::Generic9999,
    ];

    pub const fn code(self) -> u16 {
        match self {
            Self::Validation1001 => 1001,
            Self::Validation1002 => 1002,
            Self::Validation1003 => 1003,
            Self::Validation1004 => 1004,
            Self::Validation1005 => 1005,
            Self::Validation1006 => 1006,
            Self::Validation1007 => 1007,
            Self::Validation1008 => 1008,
            Self::Validation1009 => 1009,
            Self::Validation1010 => 1010,
            Self::Validation1011 => 1011,
            Self::Validation1012 => 1012,
            Self::Validation1013 => 1013,
            Self::Cli2001 => 2001,
            Self::Cli2002 => 2002,
            Self::Cli2003 => 2003,
            Self::Cli2004 => 2004,
            Self::Cli2005 => 2005,
            Self::Storage3001 => 3001,
            Self::Storage3002 => 3002,
            Self::Storage3003 => 3003,
            Self::Handoff5003 => 5003,
            Self::Handoff5004 => 5004,
            Self::Handoff5005 => 5005,
            Self::Handoff5006 => 5006,
            Self::Handoff5007 => 5007,
            Self::Handoff5008 => 5008,
            Self::Generic9999 => 9999,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Validation1001 => "VALIDATION_1001",
            Self::Validation1002 => "VALIDATION_1002",
            Self::Validation1003 => "VALIDATION_1003",
            Self::Validation1004 => "VALIDATION_1004",
            Self::Validation1005 => "VALIDATION_1005",
            Self::Validation1006 => "VALIDATION_1006",
            Self::Validation1007 => "VALIDATION_1007",
            Self::Validation1008 => "VALIDATION_1008",
            Self::Validation1009 => "VALIDATION_1009",
            Self::Validation1010 => "VALIDATION_1010",
            Self::Validation1011 => "VALIDATION_1011",
            Self::Validation1012 => "VALIDATION_1012",
            Self::Validation1013 => "VALIDATION_1013",
            Self::Cli2001 => "CLI_2001",
            Self::Cli2002 => "CLI_2002",
            Self::Cli2003 => "CLI_2003",
            Self::Cli2004 => "CLI_2004",
            Self::Cli2005 => "CLI_2005",
            Self::Storage3001 => "STORAGE_3001",
            Self::Storage3002 => "STORAGE_3002",
            Self::Storage3003 => "STORAGE_3003",
            Self::Handoff5003 => "HANDOFF_5003",
            Self::Handoff5004 => "HANDOFF_5004",
            Self::Handoff5005 => "HANDOFF_5005",
            Self::Handoff5006 => "HANDOFF_5006",
            Self::Handoff5007 => "HANDOFF_5007",
            Self::Handoff5008 => "HANDOFF_5008",
            Self::Generic9999 => "GENERIC_9999",
        }
    }
}

impl fmt::Display for TaxonomyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name(), self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AmountError {
    #[error("amount must be greater than zero")]
    BelowMinimum,

    #[error("amount exceeds maximum supply: {value}")]
    AboveMaximum { value: u64 },

    #[error("amount input is empty")]
    EmptyInput,

    #[error("negative amounts are not allowed")]
    NegativeNotAllowed,

    #[error("amount format is invalid")]
    InvalidFormat,

    #[error("amount contains non-numeric characters")]
    InvalidNumeric,

    #[error("amount has too many decimal places: {decimals} (max 8)")]
    TooManyDecimals { decimals: usize },

    #[error("amount arithmetic overflow")]
    Overflow,

    #[error("invalid zatoshi string: {value}")]
    InvalidZatoshiString { value: String },
}

impl AmountError {
    pub const fn code(&self) -> u16 {
        match self {
            Self::BelowMinimum => 1001,
            Self::AboveMaximum { .. } => 1002,
            Self::EmptyInput => 1003,
            Self::NegativeNotAllowed => 1004,
            Self::InvalidFormat => 1005,
            Self::InvalidNumeric => 1006,
            Self::TooManyDecimals { .. } => 1007,
            Self::Overflow => 1008,
            Self::InvalidZatoshiString { .. } => 1009,
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::BelowMinimum => "AMOUNT_BELOW_MINIMUM",
            Self::AboveMaximum { .. } => "AMOUNT_ABOVE_MAXIMUM",
            Self::EmptyInput => "AMOUNT_EMPTY_INPUT",
            Self::NegativeNotAllowed => "AMOUNT_NEGATIVE_NOT_ALLOWED",
            Self::InvalidFormat => "AMOUNT_INVALID_FORMAT",
            Self::InvalidNumeric => "AMOUNT_INVALID_NUMERIC",
            Self::TooManyDecimals { .. } => "AMOUNT_TOO_MANY_DECIMALS",
            Self::Overflow => "AMOUNT_OVERFLOW",
            Self::InvalidZatoshiString { .. } => "AMOUNT_INVALID_ZATOSHI_STRING",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxonomyError {
    pub code: TaxonomyCode,
    pub message: String,
}

impl TaxonomyError {
    pub fn new(code: TaxonomyCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub const fn code(&self) -> u16 {
        self.code.code()
    }

    pub const fn name(&self) -> &'static str {
        self.code.name()
    }
}

impl fmt::Display for TaxonomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}] {}", self.code(), self.name(), self.message)
    }
}

impl std::error::Error for TaxonomyError {}

#[derive(Debug, Error)]
pub enum LaminarError {
    #[error("{0}")]
    Taxonomy(#[from] TaxonomyError),

    #[error("{0}")]
    Amount(#[from] AmountError),

    #[error("{0}")]
    BatchValidation(#[from] crate::validation::BatchValidationError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unimplemented: {0}")]
    Unimplemented(&'static str),
}

impl LaminarError {
    pub fn taxonomy(code: TaxonomyCode, message: impl Into<String>) -> Self {
        Self::Taxonomy(TaxonomyError::new(code, message))
    }
}

pub type Result<T> = std::result::Result<T, LaminarError>;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{AmountError, LaminarError, TaxonomyCode, TaxonomyError};

    #[test]
    fn taxonomy_codes_cover_required_ranges_and_are_unique() {
        assert_eq!(TaxonomyCode::ALL.len(), 28);

        let unique: HashSet<u16> = TaxonomyCode::ALL.iter().map(|code| code.code()).collect();
        assert_eq!(unique.len(), TaxonomyCode::ALL.len());

        for code in TaxonomyCode::ALL {
            let numeric = code.code();
            let in_expected_range = (1001..=1013).contains(&numeric)
                || (2001..=2005).contains(&numeric)
                || (3001..=3003).contains(&numeric)
                || (5003..=5008).contains(&numeric)
                || numeric == 9999;
            assert!(in_expected_range, "unexpected taxonomy code: {numeric}");
            assert!(!code.name().is_empty());
        }
    }

    #[test]
    fn taxonomy_error_display_contains_code_and_name() {
        let err = TaxonomyError::new(TaxonomyCode::Validation1001, "example");
        let rendered = err.to_string();
        assert!(rendered.contains("1001"));
        assert!(rendered.contains("VALIDATION_1001"));
        assert!(rendered.contains("example"));
    }

    #[test]
    fn amount_error_codes_and_names_cover_all_variants() {
        let variants = vec![
            AmountError::BelowMinimum,
            AmountError::AboveMaximum { value: 42 },
            AmountError::EmptyInput,
            AmountError::NegativeNotAllowed,
            AmountError::InvalidFormat,
            AmountError::InvalidNumeric,
            AmountError::TooManyDecimals { decimals: 9 },
            AmountError::Overflow,
            AmountError::InvalidZatoshiString {
                value: "abc".to_string(),
            },
        ];

        for variant in variants {
            assert!((1001..=1009).contains(&variant.code()));
            assert!(!variant.name().is_empty());
            assert!(!variant.to_string().is_empty());
        }
    }

    #[test]
    fn laminar_error_taxonomy_helper_preserves_code_and_message() {
        let err = LaminarError::taxonomy(TaxonomyCode::Cli2001, "bad config");
        match err {
            LaminarError::Taxonomy(taxonomy) => {
                assert_eq!(taxonomy.code(), 2001);
                assert_eq!(taxonomy.name(), "CLI_2001");
                assert_eq!(taxonomy.message, "bad config");
            }
            other => panic!("expected taxonomy error, got {other}"),
        }
    }
}
