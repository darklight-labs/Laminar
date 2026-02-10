use serde::{Deserialize, Serialize};

/// TODO: enforce protocol-level supply limits for zatoshi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Zatoshi(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipient {
    pub address: String,
    pub amount: Zatoshi,
    pub memo: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionIntent {
    pub recipients: Vec<Recipient>,
    pub network: String,
    pub reference: Option<String>,
}

impl TransactionIntent {
    pub fn empty(network: impl Into<String>) -> Self {
        Self {
            recipients: Vec::new(),
            network: network.into(),
            reference: None,
        }
    }
}
