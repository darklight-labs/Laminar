//! Core library for Laminar: parsing, validation, and shared types.

pub mod output;
pub mod parser;
pub mod types;
pub mod validation;

pub use output::{format_zat_as_zec, truncate_address, AgentError, OutputMode, RowIssue};
pub use parser::{parse_zec_to_zat, ZecParseError, ZAT_PER_ZEC, MAX_SUPPLY_ZAT};
pub use types::{Network, Recipient, TransactionIntent};
pub use validation::{validate_address, validate_memo, AddressValidationError, MemoValidationError, MAX_MEMO_BYTES};
