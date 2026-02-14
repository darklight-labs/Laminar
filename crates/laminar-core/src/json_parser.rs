use serde::Deserialize;
use serde_json::Value;

use crate::error::{LaminarError, TaxonomyCode};
use crate::types::{BatchConfig, Network};
use crate::validation::RawRow;

#[derive(Debug, Deserialize)]
struct JsonBatch {
    version: String,
    network: String,
    recipients: Vec<JsonRecipient>,
}

#[derive(Debug, Deserialize)]
struct JsonRecipient {
    address: Option<String>,
    amount_zatoshis: Option<Value>,
    memo: Option<String>,
    label: Option<String>,
}

pub fn parse_json(input: &[u8], config: &BatchConfig) -> Result<Vec<RawRow>, LaminarError> {
    let normalized = strip_utf8_bom(input);
    let batch: JsonBatch = serde_json::from_slice(normalized).map_err(|err| {
        LaminarError::taxonomy(
            TaxonomyCode::Validation1006,
            format!("failed parsing JSON batch: {err}"),
        )
    })?;

    if batch.version != "1.0" {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1012,
            format!(
                "unsupported schema version '{}', expected '1.0'",
                batch.version
            ),
        ));
    }

    let expected_network = network_as_str(config.network);
    if !batch.network.eq_ignore_ascii_case(expected_network) {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1005,
            format!(
                "network mismatch: JSON='{}', config='{}'",
                batch.network, expected_network
            ),
        ));
    }

    let mut rows = Vec::with_capacity(batch.recipients.len());
    for (idx, recipient) in batch.recipients.into_iter().enumerate() {
        let row_number = idx + 1;

        let address = recipient.address.ok_or_else(|| {
            LaminarError::taxonomy(
                TaxonomyCode::Validation1012,
                format!("missing required field: recipients[{idx}].address (row {row_number})"),
            )
        })?;

        let amount_zatoshis = recipient.amount_zatoshis.ok_or_else(|| {
            LaminarError::taxonomy(
                TaxonomyCode::Validation1012,
                format!(
                    "missing required field: recipients[{idx}].amount_zatoshis (row {row_number})"
                ),
            )
        })?;

        let amount_zatoshis = json_amount_zatoshis_to_string(amount_zatoshis, row_number)?;

        rows.push(RawRow {
            row_number,
            address: address.trim().to_string(),
            amount_zec: None,
            amount_zatoshis: Some(amount_zatoshis),
            memo: recipient.memo,
            label: recipient.label,
        });
    }

    Ok(rows)
}

fn json_amount_zatoshis_to_string(value: Value, row_number: usize) -> Result<String, LaminarError> {
    match value {
        Value::String(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                Err(LaminarError::taxonomy(
                    TaxonomyCode::Validation1012,
                    format!("empty amount_zatoshis at row {}", row_number),
                ))
            } else {
                Ok(trimmed)
            }
        }
        Value::Number(number) => number.as_u64().map(|v| v.to_string()).ok_or_else(|| {
            LaminarError::taxonomy(
                TaxonomyCode::Validation1006,
                format!(
                    "amount_zatoshis must be a non-negative integer at row {}",
                    row_number
                ),
            )
        }),
        _ => Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1006,
            format!(
                "amount_zatoshis must be string or integer at row {}",
                row_number
            ),
        )),
    }
}

fn network_as_str(network: Network) -> &'static str {
    match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
    }
}

fn strip_utf8_bom(input: &[u8]) -> &[u8] {
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &input[3..]
    } else {
        input
    }
}

#[cfg(test)]
mod tests {
    use crate::error::LaminarError;
    use crate::types::{BatchConfig, Network};

    use super::parse_json;

    fn config(network: Network) -> BatchConfig {
        BatchConfig {
            network,
            max_recipients: 500,
            source_file: "test.json".to_string(),
        }
    }

    fn taxonomy_code(err: &LaminarError) -> Option<u16> {
        match err {
            LaminarError::Taxonomy(t) => Some(t.code()),
            LaminarError::BatchValidation(batch) => {
                batch.issues.first().map(|issue| issue.code.code())
            }
            _ => None,
        }
    }

    #[test]
    fn parses_valid_json() {
        let payload = r#"{
            "version": "1.0",
            "network": "mainnet",
            "recipients": [
                {
                    "address": "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs",
                    "amount_zatoshis": 1000,
                    "memo": "ok",
                    "label": "alice"
                }
            ]
        }"#;

        let rows = parse_json(payload.as_bytes(), &config(Network::Mainnet)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount_zatoshis.as_deref(), Some("1000"));
    }

    #[test]
    fn rejects_wrong_version() {
        let payload = r#"{
            "version": "2.0",
            "network": "mainnet",
            "recipients": []
        }"#;
        let err = parse_json(payload.as_bytes(), &config(Network::Mainnet)).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1012));
    }

    #[test]
    fn rejects_network_mismatch() {
        let payload = r#"{
            "version": "1.0",
            "network": "testnet",
            "recipients": []
        }"#;
        let err = parse_json(payload.as_bytes(), &config(Network::Mainnet)).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1005));
    }

    #[test]
    fn rejects_missing_fields() {
        let payload = r#"{
            "version": "1.0",
            "network": "mainnet",
            "recipients": [
                { "memo": "missing required fields" }
            ]
        }"#;
        let err = parse_json(payload.as_bytes(), &config(Network::Mainnet)).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1012));
    }

    #[test]
    fn parses_json_with_bom_and_string_amount() {
        let payload = br#"{
            "version": "1.0",
            "network": "testnet",
            "recipients": [
                {
                    "address": "tm9iMLAuYMzJ6jtFLcA7rzUmfreGuKvr7Ma",
                    "amount_zatoshis": " 42 ",
                    "memo": "ok"
                }
            ]
        }"#;
        let mut with_bom = vec![0xEF, 0xBB, 0xBF];
        with_bom.extend_from_slice(payload);

        let rows = parse_json(&with_bom, &config(Network::Testnet)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount_zatoshis.as_deref(), Some("42"));
    }

    #[test]
    fn rejects_invalid_json_with_1006() {
        let err = parse_json(br#"{invalid"#, &config(Network::Mainnet)).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1006));
    }

    #[test]
    fn rejects_empty_amount_string() {
        let payload = r#"{
            "version": "1.0",
            "network": "mainnet",
            "recipients": [
                {
                    "address": "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs",
                    "amount_zatoshis": "   "
                }
            ]
        }"#;
        let err = parse_json(payload.as_bytes(), &config(Network::Mainnet)).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1012));
    }

    #[test]
    fn rejects_non_integer_and_negative_amount_values() {
        let payload_bool = r#"{
            "version": "1.0",
            "network": "mainnet",
            "recipients": [
                {
                    "address": "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs",
                    "amount_zatoshis": true
                }
            ]
        }"#;
        let err_bool = parse_json(payload_bool.as_bytes(), &config(Network::Mainnet)).unwrap_err();
        assert_eq!(taxonomy_code(&err_bool), Some(1006));

        let payload_negative = r#"{
            "version": "1.0",
            "network": "mainnet",
            "recipients": [
                {
                    "address": "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs",
                    "amount_zatoshis": -1
                }
            ]
        }"#;
        let err_negative =
            parse_json(payload_negative.as_bytes(), &config(Network::Mainnet)).unwrap_err();
        assert_eq!(taxonomy_code(&err_negative), Some(1006));
    }
}
