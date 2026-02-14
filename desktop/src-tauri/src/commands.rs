use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use laminar_core::qr::{generate_qr as core_generate_qr, QrMode};
use laminar_core::receipt;
use laminar_core::types::{BatchConfig, Network, Recipient, TransactionIntent};
use laminar_core::validation::{validate_batch as core_validate_batch, ValidatedBatch};
use laminar_core::zip321;
use laminar_core::{csv_parser, json_parser};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::async_runtime;
use tauri::Emitter;
use tauri::Manager;
use zeroize::Zeroize;

use crate::{encryption, storage};

const OPERATION_PROGRESS_EVENT: &str = "laminar://operation-progress";

#[derive(Clone, Serialize)]
struct OperationProgressPayload {
    operation_id: String,
    phase: String,
    step: usize,
    total_steps: usize,
    title: String,
    detail: String,
}

fn parse_network(network: &str) -> Result<Network, String> {
    match network.trim().to_ascii_lowercase().as_str() {
        "mainnet" => Ok(Network::Mainnet),
        "testnet" => Ok(Network::Testnet),
        other => Err(format!(
            "unsupported network '{other}', expected mainnet or testnet"
        )),
    }
}

fn app_storage_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data dir: {err}"))?;
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create app data dir: {err}"))?;
    Ok(dir)
}

fn emit_operation_progress(
    app: &tauri::AppHandle,
    operation_id: &Option<String>,
    phase: &str,
    step: usize,
    total_steps: usize,
    title: &str,
    detail: &str,
) {
    let Some(op_id) = operation_id else {
        return;
    };

    let payload = OperationProgressPayload {
        operation_id: op_id.clone(),
        phase: phase.to_string(),
        step,
        total_steps,
        title: title.to_string(),
        detail: detail.to_string(),
    };
    let _ = app.emit(OPERATION_PROGRESS_EVENT, payload);
}

fn demo_log_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("LAMINAR_DEMO_LOG")
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false)
    })
}

fn log_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn log_line(tag: &str, message: &str, ansi_color: &str) {
    if demo_log_enabled() {
        eprintln!(
            "\x1b[1;{ansi_color}m>>> [{tag}]\x1b[0m \x1b[2m{}\x1b[0m {}",
            log_timestamp_ms(),
            message
        );
        return;
    }

    eprintln!(">>> [{tag}] {message}");
}

fn log_banner(title: &str) {
    if demo_log_enabled() {
        eprintln!();
        eprintln!("\x1b[1;92m>>> ╔══════════════════════════════════════════════════════════╗\x1b[0m");
        eprintln!("\x1b[1;92m>>> ║ {:<56} ║\x1b[0m", title);
        eprintln!("\x1b[1;92m>>> ╚══════════════════════════════════════════════════════════╝\x1b[0m");
        return;
    }

    eprintln!();
    eprintln!(">>> [{title}]");
}

fn log_step_elapsed(
    tag: &str,
    step: usize,
    total_steps: usize,
    detail: &str,
    started: &Instant,
    ansi_color: &str,
) {
    let elapsed_ms = started.elapsed().as_millis();
    log_line(
        tag,
        &format!("[{step}/{total_steps}] {detail} (+{elapsed_ms}ms)"),
        ansi_color,
    );
}

fn log_command_enter(name: &str, detail: &str) {
    let suffix = if detail.trim().is_empty() {
        String::new()
    } else {
        format!(" {detail}")
    };
    log_line("COMMAND", &format!("[ENTER] {name}{suffix}"), "96");
}

fn log_command_ok(name: &str, detail: &str) {
    let suffix = if detail.trim().is_empty() {
        String::new()
    } else {
        format!(" {detail}")
    };
    log_line("COMMAND", &format!("[OK] {name}{suffix}"), "92");
}

fn log_command_err(name: &str, err: &str) {
    log_line("COMMAND", &format!("[ERROR] {name}: {err}"), "91");
}

fn read_and_validate_with_progress<F>(
    file_path: &str,
    network: &str,
    mut on_progress: F,
) -> Result<ValidatedBatch, String>
where
    F: FnMut(usize, &str),
{
    let path = Path::new(file_path);
    let network = parse_network(network)?;
    let config = BatchConfig {
        network,
        max_recipients: 500,
        source_file: path.display().to_string(),
    };

    on_progress(1, "Reading batch file from disk...");
    let mut bytes = fs::read(path).map_err(|err| format!("failed to read file: {err}"))?;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    let parse_detail = match extension.as_str() {
        "csv" => "Parsing CSV rows...",
        "json" => "Parsing JSON recipient array...",
        _ => "Parsing input rows...",
    };
    on_progress(2, parse_detail);
    let parsed_rows = match extension.as_str() {
        "csv" => csv_parser::parse_csv(&bytes, &config).map_err(|err| err.to_string()),
        "json" => json_parser::parse_json(&bytes, &config).map_err(|err| err.to_string()),
        _ => Err(format!(
            "unsupported input extension for '{}': expected .csv or .json",
            path.display()
        )),
    };
    bytes.zeroize();

    on_progress(3, "Applying validation rules...");
    let rows = parsed_rows?;
    let validated = core_validate_batch(rows, &config).map_err(|err| err.to_string())?;
    on_progress(4, "Validation complete.");
    Ok(validated)
}

fn read_and_validate(file_path: &str, network: &str) -> Result<ValidatedBatch, String> {
    read_and_validate_with_progress(file_path, network, |_, _| {})
}

fn address_type_to_str(
    address_type: laminar_core::validation::RecipientAddressType,
) -> &'static str {
    match address_type {
        laminar_core::validation::RecipientAddressType::Unified => "unified",
        laminar_core::validation::RecipientAddressType::Sapling => "sapling",
        laminar_core::validation::RecipientAddressType::Transparent => "transparent",
    }
}

fn network_to_str(network: Network) -> &'static str {
    match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
    }
}

fn validated_batch_to_json(batch: &ValidatedBatch) -> Value {
    let recipients: Vec<Value> = batch
        .recipients
        .iter()
        .map(|validated| {
            json!({
                "row_number": validated.row_number,
                "address_type": address_type_to_str(validated.address_type),
                "recipient": {
                    "address": validated.recipient.address,
                    "amount": validated.recipient.amount.as_u64().to_string(),
                    "memo": validated.recipient.memo,
                    "label": validated.recipient.label,
                }
            })
        })
        .collect();

    json!({
        "recipients": recipients,
        "total": batch.total.as_u64().to_string(),
        "network": network_to_str(batch.network),
        "warnings": batch.warnings,
    })
}

fn qr_mode_to_str(mode: QrMode) -> &'static str {
    match mode {
        QrMode::Static => "Static",
        QrMode::AnimatedUr => "AnimatedUr",
    }
}

fn qr_output_to_json(qr_output: &laminar_core::qr::QrOutput) -> Value {
    let frames: Vec<Value> = qr_output
        .frames
        .iter()
        .map(|frame| {
            json!({
                "index": frame.index,
                "png_bytes": frame.png_bytes,
                "data": frame.data,
            })
        })
        .collect();

    json!({
        "mode": qr_mode_to_str(qr_output.mode),
        "frames": frames,
        "total_frames": qr_output.total_frames,
        "payload_bytes": qr_output.payload_bytes,
    })
}

fn recipient_to_json(recipient: &Recipient) -> Value {
    json!({
        "address": recipient.address,
        "amount": recipient.amount.as_u64().to_string(),
        "memo": recipient.memo,
        "label": recipient.label,
    })
}

fn transaction_intent_to_json(intent: &TransactionIntent) -> Value {
    json!({
        "schema_version": intent.schema_version,
        "id": intent.id,
        "created_at": intent.created_at,
        "network": network_to_str(intent.network),
        "recipients": intent.recipients.iter().map(recipient_to_json).collect::<Vec<_>>(),
        "total_zat": intent.total_zat.as_u64().to_string(),
        "zip321_uri": intent.zip321_uri,
        "payload_bytes": intent.payload_bytes,
        "payload_hash": intent.payload_hash,
    })
}

fn receipt_to_json_value(receipt: &receipt::Receipt) -> Value {
    json!({
        "laminar_version": receipt.laminar_version,
        "timestamp": receipt.timestamp,
        "batch_id": receipt.batch_id,
        "network": network_to_str(receipt.network),
        "total_zatoshis": receipt.total_zatoshis.to_string(),
        "total_zec": receipt.total_zec,
        "recipient_count": receipt.recipient_count,
        "recipients": receipt.recipients.iter().map(|recipient| {
            json!({
                "address": recipient.address,
                "amount_zatoshis": recipient.amount_zatoshis.to_string(),
                "amount_zec": recipient.amount_zec,
                "memo": recipient.memo,
                "label": recipient.label,
            })
        }).collect::<Vec<_>>(),
        "zip321_payload_hash": receipt.zip321_payload_hash,
        "segments": receipt.segments,
    })
}

fn parse_zatoshis(input: &str) -> Result<u64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("zatoshi value is empty".to_string());
    }
    if trimmed.len() > 1 && trimmed.starts_with('0') {
        return Err(format!("zatoshi value has leading zeroes: {trimmed}"));
    }
    if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("zatoshi value is not an unsigned integer: {trimmed}"));
    }
    trimmed
        .parse::<u64>()
        .map_err(|err| format!("invalid zatoshi value '{trimmed}': {err}"))
}

fn split_qr_outputs_to_json(validated: &ValidatedBatch) -> Result<Vec<Value>, String> {
    let mut out = Vec::with_capacity(validated.recipients.len());
    for (index, entry) in validated.recipients.iter().enumerate() {
        let single_batch = ValidatedBatch {
            recipients: vec![entry.clone()],
            total: entry.recipient.amount,
            network: validated.network,
            warnings: Vec::new(),
        };
        let single_intent = zip321::construct_zip321(&single_batch).map_err(|err| {
            format!(
                "failed to construct single-recipient ZIP-321 intent at row {}: {err}",
                entry.row_number
            )
        })?;
        let single_qr = core_generate_qr(&single_intent).map_err(|err| {
            format!(
                "failed to generate single-recipient QR at row {}: {err}",
                entry.row_number
            )
        })?;

        out.push(json!({
            "index": index,
            "row_number": entry.row_number,
            "address_type": address_type_to_str(entry.address_type),
            "address": entry.recipient.address,
            "amount_zatoshis": entry.recipient.amount.as_u64().to_string(),
            "amount_zec": entry.recipient.amount.to_zec_string(),
            "memo": entry.recipient.memo,
            "label": entry.recipient.label,
            "transaction_intent": transaction_intent_to_json(&single_intent),
            "qr_output": qr_output_to_json(&single_qr),
        }));
    }
    Ok(out)
}

fn zec_string_from_zatoshis(value: u64) -> String {
    let whole = value / 100_000_000;
    let frac = value % 100_000_000;
    if frac == 0 {
        return whole.to_string();
    }

    let mut frac_str = format!("{frac:08}");
    while frac_str.ends_with('0') {
        frac_str.pop();
    }
    format!("{whole}.{frac_str}")
}

fn draft_to_validate_response(draft: &storage::DraftLoaded) -> Result<Value, String> {
    let mut total_zatoshis = 0_u64;
    let recipients_json: Vec<Value> = draft
        .recipients
        .iter()
        .map(|recipient| {
            let amount = parse_zatoshis(&recipient.amount_zatoshis)?;
            total_zatoshis = total_zatoshis
                .checked_add(amount)
                .ok_or_else(|| "overflow while summing draft amounts".to_string())?;

            Ok(json!({
                "row_number": recipient.row_number,
                "address_type": recipient.address_type,
                "recipient": {
                    "address": recipient.address,
                    "amount": recipient.amount_zatoshis,
                    "memo": recipient.memo,
                    "label": recipient.label,
                }
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let network = draft.network.to_ascii_lowercase();
    Ok(json!({
        "draft": {
            "id": draft.id,
            "name": draft.name,
            "network": network,
            "created_at": draft.created_at,
            "updated_at": draft.updated_at
        },
        "validated_batch": {
            "recipients": recipients_json,
            "total": total_zatoshis.to_string(),
            "network": network,
            "warnings": []
        },
        "summary": {
            "network": network,
            "recipient_count": draft.recipients.len(),
            "total_zatoshis": total_zatoshis.to_string(),
            "total_zec": zec_string_from_zatoshis(total_zatoshis)
        }
    }))
}

async fn run_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    async_runtime::spawn_blocking(task)
        .await
        .map_err(|err| format!("background task join error: {err}"))?
}

#[tauri::command]
pub async fn validate_batch(
    app: tauri::AppHandle,
    file_path: String,
    network: String,
    operation_id: Option<String>,
) -> Result<Value, String> {
    run_blocking(move || {
        let started = Instant::now();
        log_banner("LAMINAR VALIDATION PIPELINE");
        log_line("LAMINAR CORE", &format!("INGESTING BATCH: {file_path}"), "92");
        log_line(
            "VALIDATION",
            &format!("TARGET NETWORK: {}", network.to_ascii_lowercase()),
            "92",
        );
        if let Some(op_id) = operation_id.as_deref() {
            log_line("TRACE", &format!("OPERATION ID: {op_id}"), "90");
        }

        let total_steps = 4;
        let phase = "validate";
        let title = "Validating Batch";
        let validated = match read_and_validate_with_progress(&file_path, &network, |step, detail| {
            log_step_elapsed("VALIDATION", step, total_steps, detail, &started, "92");
            emit_operation_progress(
                &app,
                &operation_id,
                phase,
                step,
                total_steps,
                title,
                detail,
            );
        }) {
            Ok(validated) => validated,
            Err(err) => {
                log_line(
                    "ERROR",
                    &format!(
                        "VALIDATION FAILED: {err} (+{}ms)",
                        started.elapsed().as_millis()
                    ),
                    "91",
                );
                return Err(err);
            }
        };

        log_line(
            "SUCCESS",
            &format!(
                "BATCH VALIDATED. {} RECORDS READY. (+{}ms)",
                validated.recipients.len(),
                started.elapsed().as_millis()
            ),
            "92",
        );
        log_line(
            "SUMMARY",
            &format!(
                "TOTAL {} zatoshis ({} ZEC)",
                validated.total.as_u64(),
                validated.total.to_zec_string()
            ),
            "96",
        );
        Ok(json!({
            "validated_batch": validated_batch_to_json(&validated),
            "summary": {
                "network": network.to_ascii_lowercase(),
                "recipient_count": validated.recipients.len(),
                "total_zatoshis": validated.total.as_u64().to_string(),
                "total_zec": validated.total.to_zec_string()
            }
        }))
    })
    .await
}

#[tauri::command]
pub async fn construct_batch(file_path: String, network: String) -> Result<Value, String> {
    run_blocking(move || {
        let started = Instant::now();
        log_banner("LAMINAR CONSTRUCT PIPELINE");
        log_line(
            "LAMINAR CORE",
            &format!("CONSTRUCTING BATCH INTENT: {file_path}"),
            "92",
        );
        log_line(
            "CONSTRUCT",
            &format!("TARGET NETWORK: {}", network.to_ascii_lowercase()),
            "92",
        );
        log_step_elapsed(
            "CONSTRUCT",
            1,
            2,
            "Validating recipient batch...",
            &started,
            "96",
        );
        let validated = match read_and_validate(&file_path, &network) {
            Ok(validated) => validated,
            Err(err) => {
                log_line(
                    "ERROR",
                    &format!(
                        "CONSTRUCT FAILED DURING VALIDATION: {err} (+{}ms)",
                        started.elapsed().as_millis()
                    ),
                    "91",
                );
                return Err(err);
            }
        };
        log_step_elapsed(
            "CONSTRUCT",
            2,
            2,
            "Building deterministic ZIP-321 payload...",
            &started,
            "96",
        );
        let intent = match zip321::construct_zip321(&validated).map_err(|err| err.to_string()) {
            Ok(intent) => intent,
            Err(err) => {
                log_line(
                    "ERROR",
                    &format!(
                        "CONSTRUCT FAILED DURING ZIP-321 BUILD: {err} (+{}ms)",
                        started.elapsed().as_millis()
                    ),
                    "91",
                );
                return Err(err);
            }
        };
        log_line(
            "SUCCESS",
            &format!(
                "INTENT READY. recipients={} total={} zatoshis payload={} bytes (+{}ms)",
                validated.recipients.len(),
                validated.total.as_u64(),
                intent.payload_bytes,
                started.elapsed().as_millis()
            ),
            "92",
        );

        Ok(json!({
            "validated_batch": validated_batch_to_json(&validated),
            "transaction_intent": transaction_intent_to_json(&intent),
        }))
    })
    .await
}

#[tauri::command]
pub async fn generate_qr(
    app: tauri::AppHandle,
    file_path: String,
    network: String,
    operation_id: Option<String>,
) -> Result<Value, String> {
    run_blocking(move || {
        let started = Instant::now();
        log_banner("LAMINAR QR PIPELINE");
        log_line(
            "LAMINAR CORE",
            &format!("GENERATING QR ARTIFACTS: {file_path}"),
            "92",
        );
        log_line(
            "QR PIPELINE",
            &format!("TARGET NETWORK: {}", network.to_ascii_lowercase()),
            "92",
        );
        if let Some(op_id) = operation_id.as_deref() {
            log_line("TRACE", &format!("OPERATION ID: {op_id}"), "90");
        }

        let total_steps = 6;
        let phase = "generate";
        let title = "Constructing Payment Request";
        let emit_step = |step: usize, detail: &str| {
            log_step_elapsed("QR", step, total_steps, detail, &started, "96");
            emit_operation_progress(
                &app,
                &operation_id,
                phase,
                step,
                total_steps,
                title,
                detail,
            );
        };

        emit_step(1, "Reading and validating recipients...");
        let validated = match read_and_validate(&file_path, &network) {
            Ok(validated) => validated,
            Err(err) => {
                log_line(
                    "ERROR",
                    &format!(
                        "QR GENERATION FAILED DURING VALIDATION: {err} (+{}ms)",
                        started.elapsed().as_millis()
                    ),
                    "91",
                );
                return Err(err);
            }
        };

        emit_step(2, "Constructing deterministic ZIP-321 payload...");
        let intent = match zip321::construct_zip321(&validated).map_err(|err| err.to_string()) {
            Ok(intent) => intent,
            Err(err) => {
                log_line(
                    "ERROR",
                    &format!(
                        "QR GENERATION FAILED DURING ZIP-321 CONSTRUCTION: {err} (+{}ms)",
                        started.elapsed().as_millis()
                    ),
                    "91",
                );
                return Err(err);
            }
        };

        emit_step(3, "Rendering batch QR frame sequence...");
        let qr_output = match core_generate_qr(&intent).map_err(|err| err.to_string()) {
            Ok(qr_output) => qr_output,
            Err(err) => {
                log_line(
                    "ERROR",
                    &format!(
                        "QR GENERATION FAILED DURING BATCH QR RENDER: {err} (+{}ms)",
                        started.elapsed().as_millis()
                    ),
                    "91",
                );
                return Err(err);
            }
        };

        emit_step(4, "Generating split-recipient fallback QRs...");
        let split_qr_outputs = match split_qr_outputs_to_json(&validated) {
            Ok(split_qr_outputs) => split_qr_outputs,
            Err(err) => {
                log_line(
                    "ERROR",
                    &format!(
                        "QR GENERATION FAILED DURING SPLIT QR RENDER: {err} (+{}ms)",
                        started.elapsed().as_millis()
                    ),
                    "91",
                );
                return Err(err);
            }
        };

        emit_step(5, "Building receipt metadata and payload hash...");
        let receipt = receipt::generate_receipt(&intent, qr_output.total_frames);

        emit_step(6, "Finalizing response payload...");
        log_line(
            "SUCCESS",
            &format!(
                "QR READY. mode={} frames={} payload={} bytes split={} receipt_batch={} (+{}ms)",
                qr_mode_to_str(qr_output.mode),
                qr_output.total_frames,
                qr_output.payload_bytes,
                split_qr_outputs.len(),
                receipt.batch_id,
                started.elapsed().as_millis()
            ),
            "92",
        );

        Ok(json!({
            "transaction_intent": transaction_intent_to_json(&intent),
            "qr_output": qr_output_to_json(&qr_output),
            "split_qr_outputs": split_qr_outputs,
            "receipt": receipt_to_json_value(&receipt),
        }))
    })
    .await
}

#[tauri::command]
pub fn save_receipt(receipt_json: String, file_path: String) -> Result<(), String> {
    log_command_enter(
        "save_receipt",
        &format!("path={file_path} bytes={}", receipt_json.len()),
    );
    let result = (|| {
        let path = Path::new(&file_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create parent directories: {err}"))?;
            }
        }
        fs::write(path, receipt_json).map_err(|err| format!("failed to write receipt: {err}"))
    })();
    match &result {
        Ok(_) => log_command_ok("save_receipt", "written"),
        Err(err) => log_command_err("save_receipt", err),
    }
    result
}

#[tauri::command]
pub fn set_storage_passphrase(passphrase: String) -> Result<(), String> {
    log_command_enter(
        "set_storage_passphrase",
        &format!("passphrase_len={}", passphrase.chars().count()),
    );
    let result = encryption::set_passphrase(passphrase);
    match &result {
        Ok(_) => log_command_ok("set_storage_passphrase", "unlocked"),
        Err(err) => log_command_err("set_storage_passphrase", err),
    }
    result
}

#[tauri::command]
pub fn is_storage_unlocked() -> bool {
    log_command_enter("is_storage_unlocked", "");
    let unlocked = encryption::is_unlocked();
    log_command_ok("is_storage_unlocked", &format!("result={unlocked}"));
    unlocked
}

#[tauri::command]
pub fn clear_sensitive_memory() -> Result<(), String> {
    log_command_enter("clear_sensitive_memory", "");
    encryption::clear_session_key();
    log_command_ok("clear_sensitive_memory", "session key cleared");
    Ok(())
}

#[tauri::command]
pub fn init_local_storage(app: tauri::AppHandle) -> Result<Value, String> {
    log_command_enter("init_local_storage", "");
    let result: Result<Value, String> = (|| -> Result<Value, String> {
        let dir = app_storage_dir(&app)?;
        storage::init_db(&dir)?;
        Ok(json!({
            "db_name": storage::DB_NAME,
            "version": storage::DB_VERSION
        }))
    })();
    match &result {
        Ok(_) => log_command_ok("init_local_storage", "ready"),
        Err(err) => log_command_err("init_local_storage", err),
    }
    result
}

#[tauri::command]
pub fn create_contact(
    app: tauri::AppHandle,
    address: String,
    label: String,
    notes: String,
) -> Result<Value, String> {
    log_command_enter(
        "create_contact",
        &format!(
            "address_len={} label_len={} notes_len={}",
            address.len(),
            label.len(),
            notes.len()
        ),
    );
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        let contact = storage::create_contact(&dir, address, label, notes)?;
        serde_json::to_value(contact).map_err(|err| format!("failed to serialize contact: {err}"))
    })();
    match &result {
        Ok(_) => log_command_ok("create_contact", "created"),
        Err(err) => log_command_err("create_contact", err),
    }
    result
}

#[tauri::command]
pub fn list_contacts(app: tauri::AppHandle) -> Result<Value, String> {
    log_command_enter("list_contacts", "");
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        let contacts = storage::list_contacts(&dir)?;
        serde_json::to_value(contacts).map_err(|err| format!("failed to serialize contacts: {err}"))
    })();
    match &result {
        Ok(_) => log_command_ok("list_contacts", "listed"),
        Err(err) => log_command_err("list_contacts", err),
    }
    result
}

#[tauri::command]
pub fn update_contact(
    app: tauri::AppHandle,
    id: String,
    address: Option<String>,
    label: Option<String>,
    notes: Option<String>,
) -> Result<Value, String> {
    log_command_enter(
        "update_contact",
        &format!(
            "id={id} address_set={} label_set={} notes_set={}",
            address.is_some(),
            label.is_some(),
            notes.is_some()
        ),
    );
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        let contact = storage::update_contact(&dir, &id, address, label, notes)?;
        serde_json::to_value(contact).map_err(|err| format!("failed to serialize contact: {err}"))
    })();
    match &result {
        Ok(_) => log_command_ok("update_contact", "updated"),
        Err(err) => log_command_err("update_contact", err),
    }
    result
}

#[tauri::command]
pub fn delete_contact(app: tauri::AppHandle, id: String) -> Result<(), String> {
    log_command_enter("delete_contact", &format!("id={id}"));
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        storage::delete_contact(&dir, &id)
    })();
    match &result {
        Ok(_) => log_command_ok("delete_contact", "deleted"),
        Err(err) => log_command_err("delete_contact", err),
    }
    result
}

#[tauri::command]
pub async fn save_draft(
    app: tauri::AppHandle,
    name: String,
    file_path: String,
    network: String,
) -> Result<Value, String> {
    log_command_enter(
        "save_draft",
        &format!("name={name} path={file_path} network={}", network.to_ascii_lowercase()),
    );
    let result = run_blocking(move || {
        let validated = read_and_validate(&file_path, &network)?;
        let recipients: Vec<storage::DraftRecipient> = validated
            .recipients
            .iter()
            .map(|entry| storage::DraftRecipient {
                row_number: entry.row_number,
                address_type: address_type_to_str(entry.address_type).to_string(),
                address: entry.recipient.address.clone(),
                amount_zatoshis: entry.recipient.amount.as_u64().to_string(),
                memo: entry.recipient.memo.clone(),
                label: entry.recipient.label.clone(),
            })
            .collect();

        let dir = app_storage_dir(&app)?;
        let draft = storage::save_draft(&dir, name, network.to_ascii_lowercase(), &recipients)?;
        serde_json::to_value(draft).map_err(|err| format!("failed to serialize draft: {err}"))
    })
    .await;
    match &result {
        Ok(_) => log_command_ok("save_draft", "saved"),
        Err(err) => log_command_err("save_draft", err),
    }
    result
}

#[tauri::command]
pub fn list_drafts(app: tauri::AppHandle) -> Result<Value, String> {
    log_command_enter("list_drafts", "");
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        let drafts = storage::list_drafts(&dir)?;
        serde_json::to_value(drafts).map_err(|err| format!("failed to serialize drafts: {err}"))
    })();
    match &result {
        Ok(_) => log_command_ok("list_drafts", "listed"),
        Err(err) => log_command_err("list_drafts", err),
    }
    result
}

#[tauri::command]
pub fn load_draft(app: tauri::AppHandle, id: String) -> Result<Value, String> {
    log_command_enter("load_draft", &format!("id={id}"));
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        let draft = storage::load_draft(&dir, &id)?;
        draft_to_validate_response(&draft)
    })();
    match &result {
        Ok(_) => log_command_ok("load_draft", "loaded"),
        Err(err) => log_command_err("load_draft", err),
    }
    result
}

#[tauri::command]
pub fn delete_draft(app: tauri::AppHandle, id: String) -> Result<(), String> {
    log_command_enter("delete_draft", &format!("id={id}"));
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        storage::delete_draft(&dir, &id)
    })();
    match &result {
        Ok(_) => log_command_ok("delete_draft", "deleted"),
        Err(err) => log_command_err("delete_draft", err),
    }
    result
}

#[tauri::command]
pub fn export_plaintext_data(app: tauri::AppHandle) -> Result<Value, String> {
    log_command_enter("export_plaintext_data", "");
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        let payload = storage::export_plaintext(&dir)?;
        serde_json::to_value(payload)
            .map_err(|err| format!("failed to serialize export payload: {err}"))
    })();
    match &result {
        Ok(_) => log_command_ok("export_plaintext_data", "exported"),
        Err(err) => log_command_err("export_plaintext_data", err),
    }
    result
}

#[tauri::command]
pub fn export_plaintext_data_to_file(
    app: tauri::AppHandle,
    file_path: String,
) -> Result<(), String> {
    log_command_enter("export_plaintext_data_to_file", &format!("path={file_path}"));
    let result = (|| {
        let dir = app_storage_dir(&app)?;
        let payload = storage::export_plaintext(&dir)?;
        let content = serde_json::to_string_pretty(&payload)
            .map_err(|err| format!("failed to serialize export payload: {err}"))?;

        let target = Path::new(&file_path);
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create export directory: {err}"))?;
            }
        }
        fs::write(target, content).map_err(|err| format!("failed to write export file: {err}"))
    })();
    match &result {
        Ok(_) => log_command_ok("export_plaintext_data_to_file", "written"),
        Err(err) => log_command_err("export_plaintext_data_to_file", err),
    }
    result
}

#[tauri::command]
pub fn encrypt_field(plaintext: String) -> Result<String, String> {
    log_command_enter("encrypt_field", &format!("plaintext_len={}", plaintext.len()));
    let result = encryption::encrypt_string(&plaintext);
    match &result {
        Ok(ciphertext) => log_command_ok("encrypt_field", &format!("ciphertext_len={}", ciphertext.len())),
        Err(err) => log_command_err("encrypt_field", err),
    }
    result
}

#[tauri::command]
pub fn decrypt_field(data: String) -> Result<String, String> {
    log_command_enter("decrypt_field", &format!("ciphertext_len={}", data.len()));
    let result = encryption::decrypt_string(&data);
    match &result {
        Ok(plaintext) => log_command_ok("decrypt_field", &format!("plaintext_len={}", plaintext.len())),
        Err(err) => log_command_err("decrypt_field", err),
    }
    result
}
