use std::path::Path;

use serde_json::json;

use crate::commands::{intent_to_agent_result, read_and_validate_file};
use crate::output::{CommandOutput, OutputHandler, QrInfo};
use laminar_core::zip321::construct_zip321;

pub fn run(
    file: &Path,
    network: &str,
    output_dir: Option<&Path>,
    output: &mut dyn OutputHandler,
) -> laminar_core::Result<CommandOutput> {
    output.progress(&format!("constructing from {}", file.display()));
    let validated = read_and_validate_file(file, network)?;
    let intent = construct_zip321(&validated)?;

    output.display_qr_info(&QrInfo {
        mode: if intent.payload_bytes <= laminar_core::batch::PAYLOAD_LIMIT_QR_STATIC {
            "static".to_string()
        } else {
            "animated-ur".to_string()
        },
        frame_count: 1,
        frame_interval_ms: laminar_core::qr::UR_FRAME_INTERVAL_MS,
        payload_bytes: intent.payload_bytes,
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
        "result": intent_to_agent_result(&intent, None),
        "rows": rows,
        "timestamp": intent.created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "warnings": warnings,
        "output_dir": output_dir,
    });

    Ok(CommandOutput::success(
        "construct",
        "construction completed",
        validated.warnings.clone(),
        Some(payload),
    ))
}
