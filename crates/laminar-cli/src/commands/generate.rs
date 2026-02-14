use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::commands::{intent_to_agent_result, read_and_validate_file};
use crate::output::{CommandOutput, OutputHandler, QrInfo};
use laminar_core::qr::{generate_qr, QrMode};
use laminar_core::receipt::{default_receipt_filename, generate_receipt, receipt_to_json};
use laminar_core::zip321::construct_zip321;

pub fn run(
    file: &Path,
    network: &str,
    output_dir: Option<&Path>,
    output: &mut dyn OutputHandler,
) -> laminar_core::Result<CommandOutput> {
    output.progress(&format!("generating artifacts for {}", file.display()));

    let validated = read_and_validate_file(file, network)?;
    let intent = construct_zip321(&validated)?;
    let qr_output = generate_qr(&intent)?;
    let receipt = generate_receipt(&intent, qr_output.total_frames);

    output.display_qr_info(&QrInfo {
        mode: match qr_output.mode {
            QrMode::Static => "static".to_string(),
            QrMode::AnimatedUr => "animated-ur".to_string(),
        },
        frame_count: qr_output.total_frames,
        frame_interval_ms: laminar_core::qr::UR_FRAME_INTERVAL_MS,
        payload_bytes: qr_output.payload_bytes,
    });

    let output_root = resolve_output_dir(file, output_dir);
    fs::create_dir_all(&output_root)?;

    let mut qr_files = Vec::with_capacity(qr_output.frames.len());
    for frame in &qr_output.frames {
        let filename = match qr_output.mode {
            QrMode::Static => "qr-static.png".to_string(),
            QrMode::AnimatedUr => format!("qr-ur-{:04}.png", frame.index + 1),
        };
        let path = output_root.join(filename);
        fs::write(&path, &frame.png_bytes)?;
        qr_files.push(path.to_string_lossy().to_string());
    }

    let receipt_filename = default_receipt_filename(&receipt);
    let receipt_path = output_root.join(receipt_filename);
    fs::write(&receipt_path, receipt_to_json(&receipt))?;

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

    let ur_encoded = match qr_output.mode {
        QrMode::Static => None,
        QrMode::AnimatedUr => Some(
            qr_output
                .frames
                .iter()
                .map(|frame| frame.data.clone())
                .collect::<Vec<_>>(),
        ),
    };

    let warnings = validated.warnings.clone();
    let payload = json!({
        "result": intent_to_agent_result(&intent, ur_encoded),
        "rows": rows,
        "timestamp": intent.created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "warnings": warnings,
        "artifacts": {
            "qrFiles": qr_files,
            "receiptFile": receipt_path.to_string_lossy().to_string(),
        },
    });

    Ok(CommandOutput::success(
        "generate",
        "generation completed",
        validated.warnings,
        Some(payload),
    ))
}

fn resolve_output_dir(input_file: &Path, output_dir: Option<&Path>) -> PathBuf {
    if let Some(path) = output_dir {
        return path.to_path_buf();
    }

    input_file
        .parent()
        .map(|parent| parent.join("laminar-output"))
        .unwrap_or_else(|| PathBuf::from("laminar-output"))
}
