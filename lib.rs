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
//! - [`HandoffResult`]: Status of wallet handoff operation
//!
//! ## Invariants
//!
//! This crate enforces the following invariants:
//!
//! - **INV-01**: Never handles spending keys or seed phrases
//! - **INV-02**: Never signs transactions
//! - **INV-03**: Never broadcasts to the network
//! - **INV-04**: All monetary math uses integer zatoshis
//! - **INV-10**: Entire batch rejected if any row fails validation
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
// CONSTANTS (INV-04: Zatoshi Standard)
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
pub const MAX_RECIPIENTS: usize = 500;

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
/// floating-point precision errors (INV-04).
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

/// Status of a wallet handoff operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandoffStatus {
    /// Intent displayed, awaiting wallet scan
    Pending,
    /// User reported submission to wallet
    Submitted,
    /// Transaction confirmed on chain
    Confirmed,
    /// User cancelled the operation
    Cancelled,
    /// Handoff failed
    Failed,
    /// Status unknown
    Unknown,
}

/// Method used for wallet handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffMode {
    /// Static QR code
    QrStatic,
    /// Animated UR sequence
    QrAnimated,
    /// Deep link (zcash: URI)
    Deeplink,
    /// Manual copy/paste
    Manual,
}

/// Result of a wallet handoff operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffResult {
    /// Schema version for forward compatibility
    pub schema_version: String,

    /// Current status
    pub status: HandoffStatus,

    /// Method used for handoff
    pub mode: HandoffMode,

    /// ISO 8601 timestamp
    pub timestamp: String,

    /// Reference to the original intent
    pub intent_id: String,

    /// Transaction ID (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
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
    // Validation errors (1xxx)
    /// Address does not match any valid Zcash encoding
    InvalidAddress = 1001,
    /// Amount is invalid (non-numeric, negative)
    InvalidAmount = 1002,
    /// Amount is zero, negative, or exceeds maximum
    AmountOutOfRange = 1003,
    /// Memo exceeds 512 bytes when UTF-8 encoded
    MemoTooLong = 1004,
    /// Memo contains invalid UTF-8 sequences
    MemoInvalidUtf8 = 1005,
    /// CSV file is malformed or uses unsupported encoding
    CsvParseError = 1008,
    /// CSV contains potential formula injection
    CsvFormulaInjection = 1009,
    /// Address network does not match configured network
    NetworkMismatch = 1010,
    /// Required column (address or amount) not found
    MissingRequiredColumn = 1012,
    /// Sum of all amounts exceeds u64 maximum
    BatchTotalOverflow = 1013,

    // Storage errors (3xxx)
    /// Database unavailable
    DbUnavailable = 3001,
    /// Encryption operation failed
    EncryptionFailed = 3003,

    // Handoff errors (5xxx)
    /// Payload exceeds maximum size for encoding mode
    PayloadTooLarge = 5003,
    /// UR encoding failed
    UrEncodingFailed = 5008,

    // Generic errors (9xxx)
    /// Unknown error
    Unknown = 9999,
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

    /// UR encoding failed
    #[error("UR encoding failed: {message}")]
    UrEncodingFailed {
        /// Error details
        message: String,
        /// Error code
        code: ErrorCode,
    },
}

// ============================================================================
// UTILITY FUNCTIONS (STUBS)
// ============================================================================

/// Returns current time as ISO 8601 string.
///
/// Note: In the actual implementation, use a proper datetime library.
fn chrono_now_iso8601() -> String {
    // Placeholder - implement with chrono or time crate
    "2025-01-01T00:00:00Z".to_string()
}

// ============================================================================
// MODULE DECLARATIONS (TO BE IMPLEMENTED)
// ============================================================================

// TODO: Implement these modules during Milestone 1
//
// pub mod zatoshi;     // Monetary arithmetic (INV-04)
// pub mod address;     // Zcash address validation
// pub mod memo;        // Memo encoding (INV-07)
// pub mod csv;         // CSV parsing and sanitization
// pub mod validation;  // Batch validation (INV-10)
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
            memo: Some("dGVzdA==".to_string()), // "test" in base64
            label: Some("Alice".to_string()),
        };

        let json = serde_json::to_string(&recipient).unwrap();
        let parsed: Recipient = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.address, recipient.address);
        assert_eq!(parsed.amount, recipient.amount);
    }

    #[test]
    fn network_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Network::Mainnet).unwrap(),
            "\"mainnet\""
        );
        assert_eq!(
            serde_json::to_string(&Network::Testnet).unwrap(),
            "\"testnet\""
        );
    }
}
