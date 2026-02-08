//! Laminar CLI entry point: CSV -> parse -> validate -> intent -> output.

use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, ContentArrangement, Table};
use indicatif::{ProgressBar, ProgressStyle};
use is_terminal::IsTerminal;

use laminar_core::{
    format_zat_as_zec, parse_zec_to_zat, truncate_address, validate_address, validate_memo,
    AgentError, Network, OutputMode, Recipient, RowIssue, TransactionIntent,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Auto,
    Json,
    Human,
}

/// CLI-only enum to satisfy clap's ValueEnum without adding clap to core.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliNetwork {
    Mainnet,
    Testnet,
}

impl CliNetwork {
    fn to_core(self) -> Network {
        match self {
            CliNetwork::Mainnet => Network::Mainnet,
            CliNetwork::Testnet => Network::Testnet,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "laminar-cli", version = "0.0.1-alpha")]
#[command(about = "Laminar tracer bullet: CSV -> parse -> validate -> construct intent -> output")]
struct Cli {
    /// Input CSV file path (must include header row).
    #[arg(long)]
    input: PathBuf,

    /// Output format: auto (tty=human, pipe=agent), json (agent), human (operator).
    #[arg(long, value_enum, default_value = "auto")]
    output: OutputFormat,

    /// Network (mainnet/testnet)
    #[arg(long, value_enum, default_value = "mainnet")]
    network: CliNetwork,

    /// Bypass confirmation prompts (required for agent mode).
    #[arg(long)]
    force: bool,
}

/// Detect output mode based on CLI flags and TTY detection.
fn detect_output_mode(output: OutputFormat) -> OutputMode {
    match output {
        OutputFormat::Human => OutputMode::Human,
        OutputFormat::Json => OutputMode::Agent,
        OutputFormat::Auto => {
            if std::io::stdout().is_terminal() {
                OutputMode::Human
            } else {
                OutputMode::Agent
            }
        }
    }
}

fn spinner(mode: OutputMode, message: &str) -> Option<ProgressBar> {
    if mode != OutputMode::Human {
        return None;
    }
    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::with_template("{spinner} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());
    pb.set_style(style);
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    Some(pb)
}

fn human_header(title: &str) {
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════════╗".bright_white()
    );
    println!("{}", format!("║  {:<61}║", title).bright_white());
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════╝".bright_white()
    );
    println!();
}

fn render_recipients_table(recipients: &[Recipient]) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Row").add_attribute(Attribute::Bold),
        Cell::new("Address").add_attribute(Attribute::Bold),
        Cell::new("Amount").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ]);

    for (i, r) in recipients.iter().enumerate() {
        let row_num = i + 1;
        let addr = truncate_address(&r.address);
        let amt = format_zat_as_zec(r.amount_zat);
        let status = format!("{} {}", "✓".green(), "Valid".green());
        table.add_row(vec![
            Cell::new(row_num),
            Cell::new(addr),
            Cell::new(amt),
            Cell::new(status),
        ]);
    }

    table
}

fn render_issues_table(issues: &[RowIssue]) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Row").add_attribute(Attribute::Bold),
        Cell::new("Field").add_attribute(Attribute::Bold),
        Cell::new("Message").add_attribute(Attribute::Bold),
    ]);

    for issue in issues {
        table.add_row(vec![
            Cell::new(issue.row),
            Cell::new(&issue.field),
            Cell::new(&issue.message),
        ]);
    }

    table
}

fn confirm_or_abort(force: bool) -> Result<bool> {
    if force {
        return Ok(true);
    }

    println!(
        "{}",
        "⚠️  This action will construct a payment intent.\n    Type 'yes' to continue, or 'no' to cancel:"
            .yellow()
    );
    print!("> ");
    io::stdout().flush().context("failed to flush stdout")?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .context("failed to read stdin")?;
    let answer = line.trim().to_lowercase();

    Ok(answer == "y" || answer == "yes")
}

fn emit_agent_error(err: AgentError) -> Result<()> {
    let json = serde_json::to_string(&err).context("failed to serialize agent error")?;
    eprintln!("{json}");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mode = detect_output_mode(cli.output);
    let network = cli.network.to_core();

    // Agent mode is non-interactive; enforce --force for destructive intent creation.
    if mode == OutputMode::Agent && !cli.force {
        let err = AgentError {
            error: "confirmation_required".to_string(),
            code: 2,
            details: None,
        };
        emit_agent_error(err)?;
        std::process::exit(2);
    }

    let pb = spinner(mode, "Reading CSV…");

    let file = File::open(&cli.input)
        .with_context(|| format!("failed to open input file: {:?}", cli.input))?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut issues: Vec<RowIssue> = Vec::new();
    let mut recipients: Vec<Recipient> = Vec::new();
    let mut total_zat: u64 = 0;

    for (i, result) in rdr.records().enumerate() {
        let row_num = i + 2;
        let row_issue_start = issues.len();
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                issues.push(RowIssue {
                    row: row_num,
                    field: "csv".to_string(),
                    message: format!("csv parse error: {e}"),
                });
                continue;
            }
        };

        let address = record.get(0).unwrap_or("").trim().to_string();
        let amount_str = record.get(1).unwrap_or("").trim().to_string();
        let memo_str = record.get(2).unwrap_or("").trim().to_string();

        if !memo_str.is_empty() {
            if let Err(e) = validate_memo(&memo_str) {
                issues.push(RowIssue {
                    row: row_num,
                    field: "memo".to_string(),
                    message: e.to_string(),
                });
            }
        }

        if let Err(e) = validate_address(&address, network) {
            issues.push(RowIssue {
                row: row_num,
                field: "address".to_string(),
                message: e.to_string(),
            });
        }

        let amount_zat = match parse_zec_to_zat(&amount_str) {
            Ok(v) => v,
            Err(e) => {
                issues.push(RowIssue {
                    row: row_num,
                    field: "amount".to_string(),
                    message: e.to_string(),
                });
                0
            }
        };

        if issues.len() == row_issue_start && amount_zat == 0 {
            issues.push(RowIssue {
                row: row_num,
                field: "amount".to_string(),
                message: "amount must be greater than 0".to_string(),
            });
        }

        // Accumulate only rows that introduced no validation issues.
        if issues.len() == row_issue_start {
            total_zat = total_zat
                .checked_add(amount_zat)
                .context("total amount overflow")?;

            let memo = if memo_str.is_empty() {
                None
            } else {
                Some(memo_str)
            };

            recipients.push(Recipient {
                address,
                amount_zat,
                memo,
            });
        }
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    if !issues.is_empty() {
        match mode {
            OutputMode::Human => {
                human_header("LAMINAR — Batch Rejected");
                println!(
                    "{} {}",
                    "✗".red(),
                    "Validation failed. No intent was constructed.".red()
                );
                println!();
                let table = render_issues_table(&issues);
                println!("{table}");
                println!();
                println!("{}", "Fix the errors above and re-run.".yellow());
            }
            OutputMode::Agent => {
                let err = AgentError {
                    error: "validation_failed".to_string(),
                    code: 1,
                    details: Some(issues),
                };
                emit_agent_error(err)?;
            }
        }
        std::process::exit(1);
    }

    if mode == OutputMode::Human {
        human_header("LAMINAR — Batch Review");
        let table = render_recipients_table(&recipients);
        println!("{table}");
        println!();
        println!(
            "{} {}",
            "Total:".bright_white().bold(),
            format_zat_as_zec(total_zat).bright_white().bold()
        );
        println!(
            "{} {}",
            "Recipients:".bright_white().bold(),
            recipients.len().to_string().bright_white().bold()
        );
        println!();

        let proceed = confirm_or_abort(cli.force)?;
        if !proceed {
            println!("{}", "Aborted. No intent was produced.".yellow());
            return Ok(());
        }
    }

    let intent = TransactionIntent {
        schema_version: "1.0".to_string(),
        network: network.as_str().to_string(),
        recipient_count: recipients.len() as u64,
        total_zat,
        recipients,
    };

    match mode {
        OutputMode::Human => {
            println!();
            println!("{}", "Intent constructed.".green().bold());
            println!(
                "{}",
                "───────────────────────────────────────────────────────────────".bright_black()
            );
            println!(
                "{}",
                "Intent JSON (for downstream tooling):"
                    .bright_white()
                    .bold()
            );
            let json =
                serde_json::to_string_pretty(&intent).context("failed to serialize intent")?;
            println!("{json}");
        }
        OutputMode::Agent => {
            let json = serde_json::to_string(&intent).context("failed to serialize intent")?;
            print!("{json}");
        }
    }

    Ok(())
}
