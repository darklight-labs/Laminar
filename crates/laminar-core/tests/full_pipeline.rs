use std::path::PathBuf;

use laminar_core::csv_parser::parse_csv;
use laminar_core::qr::{generate_qr, QrMode, QrOutput};
use laminar_core::receipt::{generate_receipt, receipt_to_json, Receipt};
use laminar_core::types::{BatchConfig, Network, TransactionIntent};
use laminar_core::validation::{validate_batch, ValidatedBatch};
use laminar_core::zip321::construct_zip321;

const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

fn test_vector_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test-vectors")
        .join(name)
}

fn decode_qr_data(png_bytes: &[u8]) -> String {
    let gray = image::load_from_memory(png_bytes).unwrap().to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(gray);
    let mut grids = prepared.detect_grids();
    assert!(!grids.is_empty(), "expected at least one QR grid");
    let (_, content) = grids.remove(0).decode().unwrap();
    content
}

fn run_pipeline(name: &str) -> (ValidatedBatch, TransactionIntent, QrOutput, Receipt, String) {
    let path = test_vector_path(name);
    let input = std::fs::read(&path).unwrap_or_else(|err| {
        panic!("failed to read test vector '{}': {err}", path.display());
    });

    let config = BatchConfig {
        network: Network::Mainnet,
        max_recipients: 500,
        source_file: path.display().to_string(),
    };

    let rows = parse_csv(&input, &config).expect("csv parsing should succeed");
    let validated = validate_batch(rows, &config).expect("validation should succeed");
    let intent = construct_zip321(&validated).expect("zip321 construction should succeed");
    let qr_output = generate_qr(&intent).expect("qr generation should succeed");
    let receipt = generate_receipt(&intent, qr_output.total_frames);
    let receipt_json = receipt_to_json(&receipt);

    (validated, intent, qr_output, receipt, receipt_json)
}

#[test]
fn full_pipeline_valid_simple() {
    let (validated, intent, qr_output, receipt, receipt_json) = run_pipeline("valid-simple.csv");

    assert_eq!(validated.recipients.len(), 10);
    assert_eq!(validated.total, intent.total_zat);
    assert_eq!(qr_output.mode, QrMode::Static);
    assert_eq!(qr_output.total_frames, 1);
    assert_eq!(qr_output.frames.len(), 1);
    assert!(qr_output.frames[0].png_bytes.starts_with(&PNG_SIGNATURE));
    assert_eq!(
        decode_qr_data(&qr_output.frames[0].png_bytes),
        intent.zip321_uri
    );

    assert_eq!(receipt.recipient_count, 10);
    assert_eq!(receipt.total_zatoshis, intent.total_zat.as_u64());
    assert_eq!(receipt.zip321_payload_hash, intent.payload_hash);
    let parsed_json: serde_json::Value =
        serde_json::from_str(&receipt_json).expect("receipt JSON should be valid");
    assert_eq!(parsed_json["recipient_count"], 10);
}

#[test]
fn full_pipeline_valid_simple_is_deterministic() {
    let first = run_pipeline("valid-simple.csv");
    let second = run_pipeline("valid-simple.csv");

    assert_eq!(first.0, second.0, "validated batches should match");
    assert_eq!(first.1, second.1, "constructed intents should match");
    assert_eq!(first.2, second.2, "QR outputs should match");
    assert_eq!(first.3, second.3, "receipts should match");
    assert_eq!(first.4, second.4, "receipt JSON should match");
}
