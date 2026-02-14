pub mod construct;
pub mod generate;
pub mod validate;

use std::path::Path;

use laminar_core::error::TaxonomyCode;
use laminar_core::types::{BatchConfig, Network, TransactionIntent};
use laminar_core::validation::{validate_batch, ValidatedBatch};
use laminar_core::{csv_parser, json_parser, LaminarError};
use serde_json::json;

pub(crate) fn parse_network(input: &str) -> Result<Network, LaminarError> {
    match input.trim().to_ascii_lowercase().as_str() {
        "mainnet" => Ok(Network::Mainnet),
        "testnet" => Ok(Network::Testnet),
        other => Err(LaminarError::taxonomy(
            TaxonomyCode::Cli2001,
            format!("unsupported network '{other}', expected mainnet or testnet"),
        )),
    }
}

pub(crate) const fn network_to_str(network: Network) -> &'static str {
    match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
    }
}

pub(crate) fn intent_to_agent_result(
    intent: &TransactionIntent,
    ur_encoded: Option<Vec<String>>,
) -> serde_json::Value {
    json!({
        "schemaVersion": intent.schema_version,
        "id": intent.id.to_string(),
        "createdAt": intent.created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "network": network_to_str(intent.network),
        "recipients": intent.recipients.iter().map(|recipient| {
            json!({
                "address": recipient.address,
                "amountZat": recipient.amount.as_u64().to_string(),
                "memo": recipient.memo,
                "label": recipient.label
            })
        }).collect::<Vec<_>>(),
        "totalZat": intent.total_zat.as_u64().to_string(),
        "zip321Uri": intent.zip321_uri,
        "payloadBytes": intent.payload_bytes,
        "urEncoded": ur_encoded
    })
}

pub(crate) fn read_and_validate_file(
    file: &Path,
    network: &str,
) -> Result<ValidatedBatch, LaminarError> {
    let network = parse_network(network)?;
    let config = BatchConfig {
        network,
        max_recipients: 500,
        source_file: file.display().to_string(),
    };

    let bytes = std::fs::read(file)?;
    let extension = file
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    let rows = match extension.as_str() {
        "csv" => csv_parser::parse_csv(&bytes, &config)?,
        "json" => json_parser::parse_json(&bytes, &config)?,
        _ => {
            return Err(LaminarError::taxonomy(
                TaxonomyCode::Cli2001,
                format!(
                    "unsupported input extension for '{}': expected .csv or .json",
                    file.display()
                ),
            ))
        }
    };

    validate_batch(rows, &config)
}
