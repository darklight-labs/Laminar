use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use colored::Colorize;
use comfy_table::{presets::ASCII_BORDERS_ONLY, ContentArrangement, Table};
use indicatif::{ProgressBar, ProgressStyle};

use crate::output::{BatchSummary, CommandOutput, OutputError, OutputHandler, QrInfo};

const SPINNER_TICKS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_TICK_MS: u64 = 80;
const ADDRESS_TRUNCATE_LEN: usize = 24;
const MEMO_TRUNCATE_LEN: usize = 24;
const LABEL_TRUNCATE_LEN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchRowStatus {
    Valid,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchTableRow {
    pub row: usize,
    pub address: String,
    pub label: Option<String>,
    pub amount: String,
    pub memo: Option<String>,
    pub status: BatchRowStatus,
}

pub struct OperatorOutput {
    quiet: bool,
    no_color: bool,
    spinner: Option<ProgressBar>,
}

impl OperatorOutput {
    pub fn new(quiet: bool, no_color: bool) -> Self {
        Self {
            quiet,
            no_color,
            spinner: None,
        }
    }

    fn success_text(&self, text: &str) -> String {
        if self.no_color {
            text.to_string()
        } else {
            text.green().to_string()
        }
    }

    fn warning_text(&self, text: &str) -> String {
        if self.no_color {
            text.to_string()
        } else {
            text.yellow().to_string()
        }
    }

    fn error_text(&self, text: &str) -> String {
        if self.no_color {
            text.to_string()
        } else {
            text.red().to_string()
        }
    }

    fn start_spinner(&mut self, operation: &str) {
        if self.quiet {
            return;
        }

        let spinner = ProgressBar::new_spinner();
        let style = ProgressStyle::with_template("{spinner} {msg}")
            .expect("valid spinner style")
            .tick_strings(&SPINNER_TICKS);
        spinner.set_style(style);
        spinner.set_message(format!("{operation}..."));
        spinner.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        self.spinner = Some(spinner);
    }

    fn stop_spinner(&mut self) {
        if let Some(spinner) = self.spinner.take() {
            spinner.finish_and_clear();
        }
    }

    fn truncate(value: &str, max_len: usize) -> String {
        let mut chars = value.chars();
        let truncated: String = chars.by_ref().take(max_len).collect();
        if chars.next().is_some() {
            format!("{truncated}...")
        } else {
            truncated
        }
    }

    fn status_text(&self, status: BatchRowStatus) -> String {
        let raw = match status {
            BatchRowStatus::Valid => "valid",
            BatchRowStatus::Warning => "warning",
            BatchRowStatus::Error => "error",
        };
        if self.no_color {
            raw.to_string()
        } else {
            match status {
                BatchRowStatus::Valid => raw.green().to_string(),
                BatchRowStatus::Warning => raw.yellow().to_string(),
                BatchRowStatus::Error => raw.red().to_string(),
            }
        }
    }

    pub fn render_batch_table(&self, rows: &[BatchTableRow]) -> String {
        let mut table = Table::new();
        table
            .load_preset(ASCII_BORDERS_ONLY)
            .set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(vec!["Row", "Address", "Label", "Amount", "Memo", "Status"]);

        for row in rows {
            table.add_row(vec![
                row.row.to_string(),
                Self::truncate(&row.address, ADDRESS_TRUNCATE_LEN),
                row.label
                    .as_deref()
                    .map(|v| Self::truncate(v, LABEL_TRUNCATE_LEN))
                    .unwrap_or_else(|| "-".to_string()),
                row.amount.clone(),
                row.memo
                    .as_deref()
                    .map(|v| Self::truncate(v, MEMO_TRUNCATE_LEN))
                    .unwrap_or_else(|| "-".to_string()),
                self.status_text(row.status),
            ]);
        }

        table.to_string()
    }

    fn extract_rows_from_payload(&self, payload: &serde_json::Value) -> Vec<BatchTableRow> {
        let Some(rows) = payload.get("rows").and_then(|rows| rows.as_array()) else {
            return Vec::new();
        };

        rows.iter()
            .filter_map(|row| {
                let row_num = row.get("row")?.as_u64()? as usize;
                let address = row.get("address")?.as_str()?.to_string();
                let amount = row.get("amount")?.as_str()?.to_string();
                let label = row
                    .get("label")
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
                let memo = row
                    .get("memo")
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
                let status = match row
                    .get("status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("valid")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "warning" => BatchRowStatus::Warning,
                    "error" => BatchRowStatus::Error,
                    _ => BatchRowStatus::Valid,
                };

                Some(BatchTableRow {
                    row: row_num,
                    address,
                    label,
                    amount,
                    memo,
                    status,
                })
            })
            .collect()
    }

    pub fn render_validation_progress(&self, total_rows: usize) {
        if self.quiet || total_rows == 0 {
            return;
        }

        let progress = ProgressBar::new(total_rows as u64);
        let style = ProgressStyle::with_template("{bar:40.cyan/blue} {pos}/{len} {msg}")
            .expect("valid progress style")
            .progress_chars("=> ");
        progress.set_style(style);
        progress.set_message("batch validation");
        progress.set_position(total_rows as u64);
        progress.finish_with_message("batch validation complete");
    }

    pub fn is_confirmation_accepted(input: &str) -> bool {
        matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
    }

    fn suggestion_for_error(error: &str) -> &'static str {
        let lowered = error.to_ascii_lowercase();
        if lowered.contains("address") {
            "Suggestion: verify the address encoding and the selected network."
        } else if lowered.contains("amount") {
            "Suggestion: confirm numeric format and zatoshi bounds."
        } else if lowered.contains("memo") {
            "Suggestion: ensure memo is UTF-8 and <= 512 bytes."
        } else if lowered.contains("network") {
            "Suggestion: pass the matching --network value for all recipients."
        } else {
            "Suggestion: review the input row values and retry."
        }
    }

    pub fn format_validation_errors(errors: &[String]) -> Vec<String> {
        errors
            .iter()
            .map(|err| format!("{err}\n  {}", Self::suggestion_for_error(err)))
            .collect()
    }
}

impl OutputHandler for OperatorOutput {
    fn start_operation(&mut self, operation: &str) {
        self.start_spinner(operation);
    }

    fn progress(&mut self, message: &str) {
        if self.quiet {
            return;
        }

        if let Some(spinner) = &self.spinner {
            spinner.set_message(message.to_string());
        } else {
            println!("{message}");
        }
    }

    fn display_batch_summary(&mut self, summary: &BatchSummary) {
        if self.quiet {
            return;
        }

        self.stop_spinner();

        let mut table = Table::new();
        table
            .load_preset(ASCII_BORDERS_ONLY)
            .set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(vec!["Network", "Recipients", "Total ZEC", "Total zatoshis"]);
        table.add_row(vec![
            summary.network.as_str(),
            &summary.recipient_count.to_string(),
            summary.total_zec.as_str(),
            &summary.total_zatoshis.to_string(),
        ]);
        println!("{table}");

        self.render_validation_progress(summary.recipient_count);
    }

    fn display_validation_errors(&mut self, errors: &[String]) {
        for formatted in Self::format_validation_errors(errors) {
            eprintln!("{}", self.error_text(&formatted));
        }
    }

    fn confirm_proceed(&mut self, prompt: &str) -> Result<bool, OutputError> {
        if !io::stdin().is_terminal() {
            return Err(OutputError::StdinBlocked);
        }

        print!("{prompt} [y/N]: ");
        io::stdout().flush().map_err(OutputError::Io)?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(OutputError::Io)?;
        Ok(Self::is_confirmation_accepted(&input))
    }

    fn display_qr_info(&mut self, info: &QrInfo) {
        if self.quiet {
            return;
        }

        self.stop_spinner();
        if info.mode.eq_ignore_ascii_case("static") {
            println!("Static QR generated ({} bytes)", info.payload_bytes);
        } else {
            println!(
                "Animated UR sequence: {} frames ({} bytes)",
                info.frame_count, info.payload_bytes
            );
        }
    }

    fn complete(&mut self, output: &CommandOutput) {
        self.stop_spinner();

        if output.ok {
            println!("{}", self.success_text(&output.message));
        } else {
            eprintln!("{}", self.error_text(&output.message));
        }

        if !output.details.is_empty() {
            for detail in &output.details {
                let lowered = detail.to_ascii_lowercase();
                if lowered.contains("duplicate address") {
                    eprintln!("{}", self.warning_text(detail));
                } else {
                    println!("{detail}");
                }
            }
        }

        if let Some(payload) = &output.payload {
            let rows = self.extract_rows_from_payload(payload);
            if !rows.is_empty() {
                println!("{}", self.render_batch_table(&rows));
            } else if let Ok(text) = serde_json::to_string_pretty(payload) {
                println!("{text}");
            } else {
                eprintln!(
                    "{}",
                    self.error_text("serialization error: failed to render payload")
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::output::{CommandOutput, OutputHandler, QrInfo};

    use super::{BatchRowStatus, BatchTableRow, OperatorOutput};

    fn make_rows(count: usize) -> Vec<BatchTableRow> {
        (0..count)
            .map(|i| BatchTableRow {
                row: i + 1,
                address: format!("t1{0:0>33}", i),
                label: Some(format!("label-{i}")),
                amount: "1.0".to_string(),
                memo: Some(format!("memo-{i}")),
                status: match i % 3 {
                    0 => BatchRowStatus::Valid,
                    1 => BatchRowStatus::Warning,
                    _ => BatchRowStatus::Error,
                },
            })
            .collect()
    }

    #[test]
    fn table_renders_for_10_50_100_rows_without_panic() {
        let output = OperatorOutput::new(false, true);
        for size in [10_usize, 50, 100] {
            let table = output.render_batch_table(&make_rows(size));
            assert!(table.contains("Row"));
            assert!(table.contains("Status"));
        }
    }

    #[test]
    fn confirmation_accepts_y_yes_case_insensitive() {
        for input in ["y", "yes", "Y", "YES"] {
            assert!(OperatorOutput::is_confirmation_accepted(input));
        }
    }

    #[test]
    fn confirmation_rejects_other_inputs() {
        for input in ["", "n", "no", "Nope", " true ", "1"] {
            assert!(!OperatorOutput::is_confirmation_accepted(input));
        }
    }

    #[test]
    fn error_display_formats_all_rows_with_suggestions() {
        let errors = vec![
            "[1001:VALIDATION_1001] row 4 address: invalid address".to_string(),
            "[1002:VALIDATION_1002] row 8 amount: invalid amount".to_string(),
        ];
        let formatted = OperatorOutput::format_validation_errors(&errors);
        assert_eq!(formatted.len(), 2);
        assert!(formatted[0].contains("row 4"));
        assert!(formatted[0].contains("Suggestion"));
        assert!(formatted[1].contains("row 8"));
        assert!(formatted[1].contains("Suggestion"));
    }

    #[test]
    fn qr_display_messages_do_not_panic() {
        let mut output = OperatorOutput::new(false, true);
        output.display_qr_info(&QrInfo {
            mode: "static".to_string(),
            frame_count: 1,
            frame_interval_ms: 100,
            payload_bytes: 512,
        });
        output.display_qr_info(&QrInfo {
            mode: "animated-ur".to_string(),
            frame_count: 12,
            frame_interval_ms: 100,
            payload_bytes: 8192,
        });
    }

    #[test]
    fn complete_renders_batch_table_from_payload_rows() {
        let mut output = OperatorOutput::new(false, true);
        let result = CommandOutput {
            ok: true,
            command: "validate".to_string(),
            message: "done".to_string(),
            details: vec![],
            payload: Some(json!({
                "rows": [
                    {
                        "row": 1,
                        "address": "t1abc",
                        "label": "alice",
                        "amount": "1.0",
                        "memo": "memo",
                        "status": "valid"
                    }
                ]
            })),
        };
        output.complete(&result);
    }
}
