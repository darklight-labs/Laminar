//! Data types shared between CLI and core.

use serde::{Deserialize, Serialize};

/// Supported network selectors.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    /// Stable string identifier used in JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
        }
    }
}

/// A single payment recipient in zatoshis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub address: String,
    pub amount_zat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// The constructed intent emitted by the CLI in agent mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionIntent {
    pub schema_version: String,
    pub network: String,
    pub recipient_count: u64,
    pub total_zat: u64,
    pub recipients: Vec<Recipient>,
}
