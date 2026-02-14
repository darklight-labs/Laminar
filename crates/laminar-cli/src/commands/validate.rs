use std::path::Path;

use serde_json::json;

use crate::commands::read_and_validate_file;
use crate::output::{BatchSummary, CommandOutput, OutputHandler};

pub fn run(
    file: &Path,
    network: &str,
    output: &mut dyn OutputHandler,
) -> laminar_core::Result<CommandOutput> {
    output.progress(&format!("validating {}", file.display()));
    let validated = read_and_validate_file(file, network)?;

    output.display_batch_summary(&BatchSummary {
        network: network.to_ascii_lowercase(),
        recipient_count: validated.recipients.len(),
        total_zatoshis: validated.total.as_u64(),
        total_zec: validated.total.to_zec_string(),
        segment_count: 1,
    });

    let rows: Vec<serde_json::Value> = validated
        .recipients
        .iter()
        .map(|recipient| {
            json!({
                "row": recipient.row_number,
                "address": recipient.recipient.address,
                "label": recipient.recipient.label,
                "amount": recipient.recipient.amount.to_zec_string(),
                "memo": recipient.recipient.memo,
                "status": "valid"
            })
        })
        .collect();

    let warnings = validated.warnings.clone();
    let payload = json!({
        "result": serde_json::Value::Null,
        "rows": rows,
        "summary": {
            "network": network.to_ascii_lowercase(),
            "recipientCount": validated.recipients.len(),
            "totalZat": validated.total.as_u64().to_string(),
            "totalZec": validated.total.to_zec_string()
        },
        "timestamp": "1970-01-01T00:00:00Z",
        "warnings": warnings,
    });

    Ok(CommandOutput::success(
        "validate",
        "validation completed",
        validated.warnings.clone(),
        Some(payload),
    ))
}
