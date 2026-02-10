use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{LaminarError, Result};
use crate::types::TransactionIntent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptBundle {
    pub receipt_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub digest_b64: String,
    pub payload: serde_json::Value,
}

pub fn build_receipt_bundle(_intent: &TransactionIntent) -> Result<ReceiptBundle> {
    // TODO: construct signed receipt bundle and hash canonical payload.
    Err(LaminarError::Unimplemented(
        "receipt::build_receipt_bundle",
    ))
}
