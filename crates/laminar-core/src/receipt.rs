use chrono::{DateTime, SecondsFormat};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{Network, TransactionIntent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptRecipient {
    pub address: String,
    pub amount_zatoshis: u64,
    pub amount_zec: String,
    pub memo: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub laminar_version: String,
    pub timestamp: String,
    pub batch_id: Uuid,
    pub network: Network,
    pub total_zatoshis: u64,
    pub total_zec: String,
    pub recipient_count: usize,
    pub recipients: Vec<ReceiptRecipient>,
    pub zip321_payload_hash: String,
    pub segments: usize,
}

pub fn generate_receipt(intent: &TransactionIntent, segment_count: usize) -> Receipt {
    let recipients: Vec<ReceiptRecipient> = intent
        .recipients
        .iter()
        .map(|recipient| ReceiptRecipient {
            address: recipient.address.clone(),
            amount_zatoshis: recipient.amount.as_u64(),
            amount_zec: recipient.amount.to_zec_string(),
            memo: recipient.memo.clone(),
            label: recipient.label.clone(),
        })
        .collect();

    Receipt {
        laminar_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: intent.created_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        batch_id: intent.id,
        network: intent.network,
        total_zatoshis: intent.total_zat.as_u64(),
        total_zec: intent.total_zat.to_zec_string(),
        recipient_count: recipients.len(),
        recipients,
        zip321_payload_hash: intent.payload_hash.clone(),
        segments: segment_count,
    }
}

pub fn receipt_to_json(receipt: &Receipt) -> String {
    serde_json::to_string_pretty(receipt).expect("receipt serialization should never fail")
}

pub fn default_receipt_filename(receipt: &Receipt) -> String {
    let date = DateTime::parse_from_rfc3339(&receipt.timestamp)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| receipt.timestamp.chars().take(10).collect());
    let batch_id = receipt.batch_id.to_string();
    let batch_prefix = batch_id.get(..8).unwrap_or(batch_id.as_str());

    format!("laminar-receipt-{date}-{batch_prefix}.json")
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::types::{Network, Recipient, Zatoshi};
    use crate::validation::{RecipientAddressType, ValidatedBatch, ValidatedRecipient};
    use crate::zip321::construct_zip321;

    use super::{default_receipt_filename, generate_receipt, receipt_to_json, Receipt};

    const MAINNET_TADDR: &str = "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs";

    fn sample_intent() -> crate::types::TransactionIntent {
        let recipients = vec![
            ValidatedRecipient {
                row_number: 1,
                address_type: RecipientAddressType::Transparent,
                recipient: Recipient {
                    address: MAINNET_TADDR.to_string(),
                    amount: Zatoshi::new(125_000_000).unwrap(),
                    memo: Some("coffee".to_string()),
                    label: Some("alice".to_string()),
                },
            },
            ValidatedRecipient {
                row_number: 2,
                address_type: RecipientAddressType::Transparent,
                recipient: Recipient {
                    address: MAINNET_TADDR.to_string(),
                    amount: Zatoshi::new(50_000_000).unwrap(),
                    memo: None,
                    label: Some("bob".to_string()),
                },
            },
        ];

        let total = recipients[0]
            .recipient
            .amount
            .checked_add(recipients[1].recipient.amount)
            .unwrap();
        let batch = ValidatedBatch {
            recipients,
            total,
            network: Network::Mainnet,
            warnings: Vec::new(),
        };
        construct_zip321(&batch).unwrap()
    }

    #[test]
    fn generate_from_valid_intent_produces_valid_json() {
        let intent = sample_intent();
        let receipt = generate_receipt(&intent, 3);
        let json = receipt_to_json(&receipt);
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed["recipient_count"], 2);
        assert_eq!(parsed["segments"], 3);
    }

    #[test]
    fn filename_format_is_correct() {
        let intent = sample_intent();
        let receipt = generate_receipt(&intent, 1);
        let filename = default_receipt_filename(&receipt);
        let expected = format!(
            "laminar-receipt-{}-{}.json",
            &receipt.timestamp[..10],
            &receipt.batch_id.to_string()[..8]
        );
        assert_eq!(filename, expected);
    }

    #[test]
    fn json_roundtrip_preserves_receipt() {
        let intent = sample_intent();
        let receipt = generate_receipt(&intent, 2);
        let json = receipt_to_json(&receipt);
        let reparsed: Receipt = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, receipt);
    }

    #[test]
    fn amounts_match_exactly() {
        let intent = sample_intent();
        let receipt = generate_receipt(&intent, 2);

        assert_eq!(receipt.total_zatoshis, intent.total_zat.as_u64());
        assert_eq!(receipt.total_zec, intent.total_zat.to_zec_string());
        assert_eq!(receipt.recipients.len(), intent.recipients.len());

        for (receipt_recipient, intent_recipient) in
            receipt.recipients.iter().zip(&intent.recipients)
        {
            assert_eq!(
                receipt_recipient.amount_zatoshis,
                intent_recipient.amount.as_u64()
            );
            assert_eq!(
                receipt_recipient.amount_zec,
                intent_recipient.amount.to_zec_string()
            );
            assert_eq!(receipt_recipient.memo, intent_recipient.memo);
            assert_eq!(receipt_recipient.label, intent_recipient.label);
        }
    }

    #[test]
    fn payload_hash_is_consistent() {
        let intent = sample_intent();
        let receipt = generate_receipt(&intent, 1);
        assert_eq!(receipt.zip321_payload_hash, intent.payload_hash);
        assert!(receipt.zip321_payload_hash.starts_with("sha256:"));
    }
}
