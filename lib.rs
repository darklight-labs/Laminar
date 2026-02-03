//! # Laminar Core
//!
//! Stateless transaction construction engine for Zcash batch operations.
//!
//! ## Overview
//!
//! Laminar Core provides the foundational library for constructing Zcash payment
//! requests from batch data (CSV/JSON). It enforces strict invariants around
//! monetary precision, address validation, and deterministic output.
//!
//! ## Key Types
//!
//! - [`Zatoshi`]: Integer monetary representation (1 ZEC = 100,000,000 zatoshis)
//! - [`Recipient`]: Validated payment recipient with address, amount, and optional memo
//! - [`TransactionIntent`]: Complete payment request ready for encoding
//! - [`CliOutput`]: Schema-compliant output for Agent mode
//!
//! ## Invariants
//!
//! This crate enforces the following invariants:
//!
//! - **INV-01**: Never handles spending keys or seed phrases
//! - **INV-02**: Never signs transactions
//! - **INV-03**: All monetary math uses integer zatoshis
//! - **INV-04**: Deterministic output for identical input
//! - **INV-05**: Entire batch rejected if any row fails validation
//! - **INV-06**: Modal determinism (CLI modes behave identically)
//! - **INV-07**: Non-blocking agent mode
//!
//! ## Example
//!
//! ```ignore
//! use laminar_core::{parse_csv, construct_intent, encode_zip321};
//!
//! let csv_data = r#"address,amount,memo
//! u1abc...,10.5,January payment
//! u1def...,25.0,January payment"#;
//!
//! let recipients = parse_csv(csv_data, Network::Mainnet)?;
//! let intent = construct_intent(recipients, Network::Mainnet)?;
//! let uri = encode_zip321(&intent)?;
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::float_arithmetic)]

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// CONSTANTS (INV-03: Zatoshi Standard)
// ============================================================================

/// Conversion factor: 1 ZEC = 100,000,000 zatoshis
pub const ZATOSHI_PER_ZEC: u64 = 100_000_000;

/// Minimum valid amount (1 zatoshi)
pub const ZATOSHI_MIN: u64 = 1;

/// Maximum valid amount (21 million ZEC total supply cap)
pub const ZATOSHI_MAX: u64 = 2_100_000_000_000_000;

/// Dust threshold (0.0001 ZEC = 10,000 zatoshis)
pub const DUST_THRESHOLD_ZAT: u64 = 10_000;

/// Maximum memo size in bytes (Zcash protocol limit)
pub const MEMO_MAX_BYTES: usize = 512;

/// Maximum recipients per batch
pub const MAX_RECIPIENTS: usize = 1000;

/// Maximum CSV file size (10MB)
pub const MAX_CSV_FILE_SIZE: usize = 10_485_760;

/// Maximum payload for static QR code (with 15% safety margin)
pub const PAYLOAD_LIMIT_QR_STATIC: usize = 2510;

/// Maximum payload for animated UR sequence (with 10% safety margin)
pub const PAYLOAD_LIMIT_QR_ANIMATED: usize = 29_000;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Integer representation of Zcash monetary value.
///
/// All monetary arithmetic in Laminar uses this type to prevent
/// floating-point precision errors (INV-03).
///
/// # Conversion
///
/// - 1 ZEC = 100,000,000 zatoshis
/// - 0.00000001 ZEC = 1 zatoshi
pub type Zatoshi = u64;

/// Network identifier for address validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    /// Zcash mainnet
    Mainnet,
    /// Zcash testnet
    Testnet,
}

/// CLI execution mode (INV-06: Modal Determinism)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Human-centric: spinners, tables, colors, confirmations
    Operator,
    /// Machine-centric: silent, strict JSON, non-interactive
    Agent,
}

/// A validated recipient within a batch.
///
/// All fields have been validated against protocol constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    /// Validated Zcash address (Unified, Sapling, or Transparent)
    pub address: String,

    /// Payment amount in zatoshis
    pub amount: Zatoshi,

    /// Optional encrypted memo (max 512 bytes, base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Optional display label (not transmitted, for UI only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A complete payment intent ready for encoding.
///
/// This is the primary artifact Laminar produces. It contains all
/// information needed to construct a ZIP-321 payment request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionIntent {
    /// Schema version for forward compatibility
    pub schema_version: String,

    /// Unique identifier for this intent
    pub id: String,

    /// ISO 8601 creation timestamp
    pub created_at: String,

    /// Target network
    pub network: Network,

    /// Validated recipients
    pub recipients: Vec<Recipient>,

    /// Sum of all recipient amounts
    pub total_zat: Zatoshi,

    /// Constructed ZIP-321 URI
    pub zip321_uri: String,

    /// Byte length of the ZIP-321 payload
    pub payload_bytes: usize,
}

impl TransactionIntent {
    /// Create a new transaction intent.
    ///
    /// Automatically generates UUID, timestamp, and calculates totals.
    pub fn new(recipients: Vec<Recipient>, network: Network, zip321_uri: String) -> Self {
        let total_zat = recipients.iter().map(|r| r.amount).sum();
        let payload_bytes = zip321_uri.len();

        Self {
            schema_version: "1.0".to_string(),
            id: Uuid::new_v4().to_string(),
            created_at: chrono_now_iso8601(),
            network,
            recipients,
            total_zat,
            zip321_uri,
            payload_bytes,
        }
    }
}

// ============================================================================
// CLI OUTPUT SCHEMA (Agent Mode)
// ============================================================================

/// CLI output envelope for Agent mode (INV-06).
///
/// All Agent mode output conforms to this schema for reliable machine parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliOutput {
    /// True if operation completed without errors
    pub success: bool,

    /// Semantic version of laminar-cli
    pub laminar_version: String,

    /// Execution mode (always "agent" in Agent mode)
    pub mode: Mode,

    /// Operation performed: validate, construct, generate
    pub operation: String,

    /// ISO 8601 timestamp of operation completion
    pub timestamp: String,

    /// Operation result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<CliResult>,

    /// Error details (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CliError>,
}

/// Successful operation result in Agent mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResult {
    /// Unique batch identifier
    pub batch_id: String,

    /// Target network
    pub network: Network,

    /// Number of recipients in batch
    pub recipient_count: usize,

    /// Total amount in zatoshis
    pub total_zatoshis: Zatoshi,

    /// Total amount in ZEC (string for precision)
    pub total_zec: String,

    /// Number of QR segments required
    pub segments: usize,

    /// Constructed ZIP-321 URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip321_uri: Option<String>,

    /// UR-encoded fragments for animated QR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ur_encoded: Option<Vec<String>>,

    /// Non-fatal warnings
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

/// Error details in Agent mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliError {
    /// Error code (e.g., "E001")
    pub code: String,

    /// Error name (e.g., "INVALID_ADDRESS_FORMAT")
    pub name: String,

    /// Human-readable error message
    pub message: String,

    /// Additional error context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<CliErrorDetails>,
}

/// Additional context for errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliErrorDetails {
    /// Row number where error occurred (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row: Option<usize>,

    /// Column name where error occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,

    /// Invalid value that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Expected formats (for address errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_formats: Option<Vec<String>>,
}

// ============================================================================
// CLI EXIT CODES
// ============================================================================

/// CLI exit codes for Agent mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// Operation completed successfully
    Success = 0,
    /// Input data invalid
    ValidationError = 1,
    /// Invalid arguments or flags
    ConfigError = 2,
    /// File read/write failure
    IoError = 3,
    /// Unexpected failure
    InternalError = 4,
}

// ============================================================================
// ERROR TAXONOMY
// ============================================================================

/// Error codes for Laminar operations.
///
/// All errors must use codes from this enum. No string literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ErrorCode {
    // Validation errors (E001-E009)
    /// Address does not match any valid Zcash encoding
    InvalidAddressFormat = 1,
    /// Address network does not match configured network
    NetworkMismatch = 2,
    /// Amount is zero, negative, or exceeds maximum
    AmountOutOfRange = 3,
    /// Amount cannot be represented as integer zatoshis
    AmountPrecisionLoss = 4,
    /// Memo exceeds 512 bytes when UTF-8 encoded
    MemoTooLong = 5,
    /// Memo contains invalid UTF-8 sequences
    MemoInvalidUtf8 = 6,
    /// Sum of all amounts exceeds u64 maximum
    BatchTotalOverflow = 7,
    /// CSV file is malformed or uses unsupported encoding
    CsvParseError = 8,
    /// Required column (address or amount) not found
    MissingRequiredColumn = 9,

    // CLI errors (E010-E011) — NEW in v2.0
    /// Required CLI argument not provided (Agent mode)
    MissingRequiredArgument = 10,
    /// Operation requires --force flag in non-interactive mode
    ConfirmationRequired = 11,

    // Internal errors
    /// Unknown error
    Unknown = 99,
}

impl ErrorCode {
    /// Returns the error code string (e.g., "E001")
    pub fn code_string(&self) -> String {
        format!("E{:03}", *self as u16)
    }

    /// Returns the error name (e.g., "INVALID_ADDRESS_FORMAT")
    pub fn name(&self) -> &'static str {
        match self {
            Self::InvalidAddressFormat => "INVALID_ADDRESS_FORMAT",
            Self::NetworkMismatch => "NETWORK_MISMATCH",
            Self::AmountOutOfRange => "AMOUNT_OUT_OF_RANGE",
            Self::AmountPrecisionLoss => "AMOUNT_PRECISION_LOSS",
            Self::MemoTooLong => "MEMO_TOO_LONG",
            Self::MemoInvalidUtf8 => "MEMO_INVALID_UTF8",
            Self::BatchTotalOverflow => "BATCH_TOTAL_OVERFLOW",
            Self::CsvParseError => "CSV_PARSE_ERROR",
            Self::MissingRequiredColumn => "MISSING_REQUIRED_COLUMN",
            Self::MissingRequiredArgument => "MISSING_REQUIRED_ARGUMENT",
            Self::ConfirmationRequired => "CONFIRMATION_REQUIRED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// Warning codes (non-fatal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum WarningCode {
    /// Same address appears multiple times in batch
    DuplicateAddress = 1,
}

impl WarningCode {
    /// Returns the warning code string (e.g., "W001")
    pub fn code_string(&self) -> String {
        format!("W{:03}", *self as u16)
    }
}

/// Errors that can occur during Laminar operations.
#[derive(Error, Debug)]
pub enum LaminarError {
    /// Address validation failed
    #[error("Invalid address at row {row}: {message}")]
    InvalidAddress {
        /// Row number (1-indexed)
        row: usize,
        /// Error details
        message: String,
        /// Error code
        code: ErrorCode,
    },

    /// Amount validation failed
    #[error("Invalid amount at row {row}: {message}")]
    InvalidAmount {
        /// Row number (1-indexed)
        row: usize,
        /// Error details
        message: String,
        /// Error code
        code: ErrorCode,
    },

    /// Memo validation failed
    #[error("Invalid memo at row {row}: {message}")]
    InvalidMemo {
        /// Row number (1-indexed)
        row: usize,
        /// Error details
        message: String,
        /// Error code
        code: ErrorCode,
    },

    /// CSV parsing failed
    #[error("CSV parse error: {message}")]
    CsvError {
        /// Error details
        message: String,
        /// Error code
        code: ErrorCode,
    },

    /// Network mismatch
    #[error("Network mismatch at row {row}: expected {expected:?}, got {actual:?}")]
    NetworkMismatch {
        /// Row number (1-indexed)
        row: usize,
        /// Expected network
        expected: Network,
        /// Actual network detected
        actual: Network,
        /// Error code
        code: ErrorCode,
    },

    /// Batch total overflow
    #[error("Batch total overflow: sum exceeds maximum")]
    BatchOverflow {
        /// Error code
        code: ErrorCode,
    },

    /// Missing required CLI argument (Agent mode)
    #[error("Missing required argument: {argument}")]
    MissingArgument {
        /// Missing argument name
        argument: String,
        /// Error code
        code: ErrorCode,
    },

    /// Confirmation required but --force not provided
    #[error("Confirmation required: use --force to proceed in non-interactive mode")]
    ConfirmationRequired {
        /// Error code
        code: ErrorCode,
    },

    /// Payload too large for encoding mode
    #[error("Payload too large: {size} bytes exceeds {limit} byte limit")]
    PayloadTooLarge {
        /// Actual payload size
        size: usize,
        /// Maximum allowed size
        limit: usize,
        /// Error code
        code: ErrorCode,
    },
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Returns current time as ISO 8601 string.
fn chrono_now_iso8601() -> String {
    // Placeholder - implement with chrono or time crate
    "2025-01-01T00:00:00Z".to_string()
}

// ============================================================================
// MODULE DECLARATIONS (TO BE IMPLEMENTED)
// ============================================================================

// TODO: Implement these modules during Milestone 1
//
// pub mod zatoshi;     // Monetary arithmetic (INV-03)
// pub mod address;     // Zcash address validation
// pub mod memo;        // Memo encoding
// pub mod csv;         // CSV parsing and sanitization
// pub mod validation;  // Batch validation (INV-05)
// pub mod zip321;      // Payment request construction
// pub mod ur;          // Uniform Resources encoding
// pub mod receipt;     // JSON receipt generation

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zatoshi_constants_are_correct() {
        assert_eq!(ZATOSHI_PER_ZEC, 100_000_000);
        assert_eq!(ZATOSHI_MAX, 21_000_000 * ZATOSHI_PER_ZEC);
    }

    #[test]
    fn recipient_serialization_roundtrip() {
        let recipient = Recipient {
            address: "u1test...".to_string(),
            amount: 150_000_000, // 1.5 ZEC
            memo: Some("dGVzdA==".to_string()),
            label: Some("Alice".to_string()),
        };

        let json = serde_json::to_string(&recipient).unwrap();
        let parsed: Recipient = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.address, recipient.address);
        assert_eq!(parsed.amount, recipient.amount);
    }

    #[test]
    fn error_code_formatting() {
        assert_eq!(ErrorCode::InvalidAddressFormat.code_string(), "E001");
        assert_eq!(ErrorCode::MissingRequiredArgument.code_string(), "E010");
        assert_eq!(ErrorCode::ConfirmationRequired.code_string(), "E011");
    }

    #[test]
    fn cli_output_success_serialization() {
        let output = CliOutput {
            success: true,
            laminar_version: "1.0.0".to_string(),
            mode: Mode::Agent,
            operation: "construct".to_string(),
            timestamp: "2025-01-28T12:00:00Z".to_string(),
            result: Some(CliResult {
                batch_id: "abc123".to_string(),
                network: Network::Mainnet,
                recipient_count: 10,
                total_zatoshis: 500_000_000,
                total_zec: "5.0".to_string(),
                segments: 1,
                zip321_uri: Some("zcash:?...".to_string()),
                ur_encoded: None,
                warnings: vec![],
            }),
            error: None,
        };

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"mode\": \"agent\""));
    }

    #[test]
    fn cli_output_error_serialization() {
        let output = CliOutput {
            success: false,
            laminar_version: "1.0.0".to_string(),
            mode: Mode::Agent,
            operation: "validate".to_string(),
            timestamp: "2025-01-28T12:00:00Z".to_string(),
            result: None,
            error: Some(CliError {
                code: "E001".to_string(),
                name: "INVALID_ADDRESS_FORMAT".to_string(),
                message: "Address at row 5 is invalid".to_string(),
                details: Some(CliErrorDetails {
                    row: Some(5),
                    column: Some("address".to_string()),
                    value: Some("invalid_addr".to_string()),
                    expected_formats: Some(vec!["Unified (u1...)".to_string()]),
                }),
            }),
        };

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("\"success\": false"));
        assert!(json.contains("\"code\": \"E001\""));
    }
}
