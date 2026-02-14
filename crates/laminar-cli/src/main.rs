mod agent;
mod commands;
mod mode;
mod operator;
mod output;

use std::path::PathBuf;

use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};
use laminar_core::error::TaxonomyCode;
use laminar_core::LaminarError;
use serde_json::json;

use mode::Mode;
use output::{BatchSummary, CommandOutput, OutputError, OutputHandler};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum OutputFormat {
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "laminar", version, about = "Laminar dual-mode CLI")]
struct Cli {
    #[arg(long = "output", value_enum, global = true)]
    output_format: Option<OutputFormat>,

    #[arg(long, global = true)]
    interactive: bool,

    #[arg(long, global = true)]
    force: bool,

    #[arg(long, global = true)]
    quiet: bool,

    #[arg(long = "no-color", global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    Validate {
        file: PathBuf,
        #[arg(long)]
        network: String,
    },
    Construct {
        file: PathBuf,
        #[arg(long)]
        network: String,
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    Generate {
        file: PathBuf,
        #[arg(long)]
        network: String,
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitCode {
    Success = 0,
    ValidationError = 1,
    ConfigError = 2,
    IoError = 3,
    InternalError = 4,
    ConfirmationRequired = 10,
    StdinBlocked = 11,
}

impl ExitCode {
    const fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug)]
enum CliError {
    Core(LaminarError),
    ConfirmationRequired,
    StdinBlocked,
    Internal(String),
}

impl From<LaminarError> for CliError {
    fn from(value: LaminarError) -> Self {
        Self::Core(value)
    }
}

impl From<OutputError> for CliError {
    fn from(value: OutputError) -> Self {
        match value {
            OutputError::StdinBlocked => Self::StdinBlocked,
            OutputError::Io(err) => Self::Internal(err.to_string()),
        }
    }
}

fn build_output_handler(mode: Mode, quiet: bool, no_color: bool) -> Box<dyn OutputHandler> {
    match mode {
        Mode::Operator => Box::new(operator::OperatorOutput::new(quiet, no_color)),
        Mode::Agent => Box::new(agent::AgentOutputHandler::new(quiet)),
    }
}

fn command_label(command: &Commands) -> &'static str {
    match command {
        Commands::Validate { .. } => "validate",
        Commands::Construct { .. } => "construct",
        Commands::Generate { .. } => "generate",
    }
}

fn detect_mode_from_raw_args(args: &[String]) -> Mode {
    let mut output_json = false;
    let mut interactive = false;

    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--interactive" {
            interactive = true;
            index += 1;
            continue;
        }
        if arg == "--output" {
            if args
                .get(index + 1)
                .is_some_and(|value| value.eq_ignore_ascii_case("json"))
            {
                output_json = true;
            }
            index += 2;
            continue;
        }
        if arg.eq_ignore_ascii_case("--output=json") {
            output_json = true;
            index += 1;
            continue;
        }
        index += 1;
    }

    mode::detect_mode(output_json, interactive)
}

fn infer_operation_from_raw_args(args: &[String]) -> &'static str {
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "validate" => return "validate",
            "construct" => return "construct",
            "generate" => return "generate",
            "receipt" => return "receipt",
            _ => {}
        }
    }
    "validate"
}

fn clap_error_to_cli_error(err: &clap::Error) -> CliError {
    let code = match err.kind() {
        ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand => TaxonomyCode::Cli2001,
        ErrorKind::ArgumentConflict
        | ErrorKind::UnknownArgument
        | ErrorKind::InvalidValue
        | ErrorKind::TooManyValues => TaxonomyCode::Cli2002,
        _ => TaxonomyCode::Cli2001,
    };
    let message = err.to_string().trim().to_string();
    CliError::Core(LaminarError::taxonomy(code, message))
}

fn maybe_confirm(
    output: &mut dyn OutputHandler,
    force: bool,
    prompt: &str,
) -> std::result::Result<(), CliError> {
    if force {
        return Ok(());
    }

    match output.confirm_proceed(prompt) {
        Ok(true) => Ok(()),
        Ok(false) => Err(CliError::ConfirmationRequired),
        Err(err) => Err(err.into()),
    }
}

fn dispatch(
    command: &Commands,
    force: bool,
    output: &mut dyn OutputHandler,
) -> std::result::Result<CommandOutput, CliError> {
    match command {
        Commands::Validate { file, network } => {
            commands::validate::run(file, network, output).map_err(CliError::from)
        }
        Commands::Construct {
            file,
            network,
            output_dir,
        } => {
            let validated =
                commands::read_and_validate_file(file, network).map_err(CliError::from)?;
            output.display_batch_summary(&BatchSummary {
                network: network.to_ascii_lowercase(),
                recipient_count: validated.recipients.len(),
                total_zatoshis: validated.total.as_u64(),
                total_zec: validated.total.to_zec_string(),
                segment_count: 1,
            });
            maybe_confirm(output, force, "Proceed?")?;
            commands::construct::run(file, network, output_dir.as_deref(), output)
                .map_err(CliError::from)
        }
        Commands::Generate {
            file,
            network,
            output_dir,
        } => {
            let validated =
                commands::read_and_validate_file(file, network).map_err(CliError::from)?;
            output.display_batch_summary(&BatchSummary {
                network: network.to_ascii_lowercase(),
                recipient_count: validated.recipients.len(),
                total_zatoshis: validated.total.as_u64(),
                total_zec: validated.total.to_zec_string(),
                segment_count: 1,
            });
            maybe_confirm(output, force, "Proceed?")?;
            commands::generate::run(file, network, output_dir.as_deref(), output)
                .map_err(CliError::from)
        }
    }
}

fn exit_code_for_core_error(err: &LaminarError) -> ExitCode {
    match err {
        LaminarError::Amount(_) | LaminarError::BatchValidation(_) => ExitCode::ValidationError,
        LaminarError::Io(_) | LaminarError::Csv(_) | LaminarError::Json(_) => ExitCode::IoError,
        LaminarError::Unimplemented(_) => ExitCode::InternalError,
        LaminarError::Taxonomy(taxonomy) => {
            let code = taxonomy.code();
            if (1001..=1013).contains(&code) {
                ExitCode::ValidationError
            } else if (2001..=2005).contains(&code) {
                ExitCode::ConfigError
            } else if (3001..=3003).contains(&code) {
                ExitCode::IoError
            } else {
                ExitCode::InternalError
            }
        }
    }
}

fn exit_code_for_error(err: &CliError) -> ExitCode {
    match err {
        CliError::Core(core) => exit_code_for_core_error(core),
        CliError::ConfirmationRequired => ExitCode::ConfirmationRequired,
        CliError::StdinBlocked => ExitCode::StdinBlocked,
        CliError::Internal(_) => ExitCode::InternalError,
    }
}

fn error_name_for_code(code: u16) -> &'static str {
    match code {
        1001 => "INVALID_ADDRESS",
        1002 => "INVALID_AMOUNT",
        1003 => "AMOUNT_OUT_OF_RANGE",
        1004 => "MEMO_TOO_LONG",
        1008 => "CSV_PARSE_ERROR",
        1009 => "CSV_FORMULA_INJECTION",
        1010 => "NETWORK_MISMATCH",
        1012 => "MISSING_REQUIRED_COLUMN",
        1013 => "BATCH_TOTAL_OVERFLOW",
        2001 => "MISSING_REQUIRED_ARGUMENT",
        2002 => "INVALID_FLAG_COMBINATION",
        2003 => "CONFIRMATION_REQUIRED",
        2004 => "STDIN_BLOCKED",
        2005 => "INVALID_JSON_INPUT",
        3001 => "DB_UNAVAILABLE",
        3003 => "ENCRYPTION_FAILED",
        5003 => "PAYLOAD_TOO_LARGE",
        5008 => "UR_ENCODING_FAILED",
        _ => "UNKNOWN",
    }
}

fn details_as_lines(details: &Option<serde_json::Value>) -> Vec<String> {
    if let Some(value) = details {
        return vec![value.to_string()];
    }
    Vec::new()
}

fn output_from_error(err: &CliError, command: &str) -> CommandOutput {
    let agent_error = cli_error_to_agent_error(err);
    let details = details_as_lines(&agent_error.details);
    match err {
        CliError::Core(core) => {
            let mut output = CommandOutput::from_core_error(core, command);
            output.message = agent_error.message.clone();
            output.details = details.clone();
            output.payload = Some(json!({
                "error": agent_error,
                "timestamp": "1970-01-01T00:00:00Z",
                "warnings": [],
            }));
            output
        }
        CliError::ConfirmationRequired => CommandOutput {
            ok: false,
            command: command.to_string(),
            message: agent_error.message.clone(),
            details: details.clone(),
            payload: Some(json!({
                "error": agent_error,
                "timestamp": "1970-01-01T00:00:00Z",
                "warnings": [],
            })),
        },
        CliError::StdinBlocked => CommandOutput {
            ok: false,
            command: command.to_string(),
            message: agent_error.message.clone(),
            details: details.clone(),
            payload: Some(json!({
                "error": agent_error,
                "timestamp": "1970-01-01T00:00:00Z",
                "warnings": [],
            })),
        },
        CliError::Internal(message) => CommandOutput {
            ok: false,
            command: command.to_string(),
            message: format!("internal error: {message}"),
            details,
            payload: Some(json!({
                "error": agent_error,
                "timestamp": "1970-01-01T00:00:00Z",
                "warnings": [],
            })),
        },
    }
}

fn cli_error_to_agent_error(err: &CliError) -> agent::AgentError {
    match err {
        CliError::Core(core) => match core {
            LaminarError::Taxonomy(taxonomy) => agent::AgentError {
                code: taxonomy.code(),
                name: error_name_for_code(taxonomy.code()).to_string(),
                message: taxonomy.message.clone(),
                details: Some(json!({
                    "raw": core.to_string()
                })),
            },
            LaminarError::Amount(amount) => agent::AgentError {
                code: amount.code(),
                name: error_name_for_code(amount.code()).to_string(),
                message: amount.to_string(),
                details: Some(json!({
                    "raw": amount.to_string()
                })),
            },
            LaminarError::BatchValidation(batch) => {
                let first_issue = batch.issues.first();
                let code = first_issue.map(|issue| issue.code.code()).unwrap_or(1001);
                agent::AgentError {
                    code,
                    name: error_name_for_code(code).to_string(),
                    message: first_issue
                        .map(|issue| issue.message.clone())
                        .unwrap_or_else(|| "batch validation failed".to_string()),
                    details: Some(json!({
                        "issues": batch
                            .issues
                            .iter()
                            .map(|issue| {
                                json!({
                                    "code": issue.code.code(),
                                    "name": error_name_for_code(issue.code.code()),
                                    "row": issue.row_number,
                                    "column": issue.column,
                                    "message": issue.message,
                                })
                            })
                            .collect::<Vec<_>>()
                    })),
                }
            }
            LaminarError::Io(io) => agent::AgentError {
                code: 3001,
                name: error_name_for_code(3001).to_string(),
                message: io.to_string(),
                details: Some(json!({
                    "raw": io.to_string()
                })),
            },
            LaminarError::Csv(csv) => agent::AgentError {
                code: 1008,
                name: error_name_for_code(1008).to_string(),
                message: csv.to_string(),
                details: Some(json!({
                    "raw": csv.to_string()
                })),
            },
            LaminarError::Json(json) => agent::AgentError {
                code: 2005,
                name: error_name_for_code(2005).to_string(),
                message: json.to_string(),
                details: Some(json!({
                    "raw": json.to_string()
                })),
            },
            LaminarError::Unimplemented(message) => agent::AgentError {
                code: 9999,
                name: error_name_for_code(9999).to_string(),
                message: message.to_string(),
                details: Some(json!({
                    "raw": core.to_string()
                })),
            },
        },
        CliError::ConfirmationRequired => agent::AgentError {
            code: 2003,
            name: error_name_for_code(2003).to_string(),
            message: "confirmation required".to_string(),
            details: Some(json!({
                "hint": "Re-run with --force for non-interactive execution."
            })),
        },
        CliError::StdinBlocked => agent::AgentError {
            code: 2004,
            name: error_name_for_code(2004).to_string(),
            message: "stdin is blocked for confirmation".to_string(),
            details: Some(json!({
                "hint": "Use --force or run in an interactive terminal."
            })),
        },
        CliError::Internal(message) => agent::AgentError {
            code: 9999,
            name: error_name_for_code(9999).to_string(),
            message: message.clone(),
            details: Some(json!({
                "raw": message
            })),
        },
    }
}

fn run(cli: Cli) -> ExitCode {
    let output_json = matches!(cli.output_format, Some(OutputFormat::Json));
    let mode = mode::detect_mode(output_json, cli.interactive);
    let mut output = build_output_handler(mode, cli.quiet, cli.no_color);
    let command = command_label(&cli.command).to_string();

    output.start_operation(&command);
    match dispatch(&cli.command, cli.force, output.as_mut()) {
        Ok(result) => {
            output.complete(&result);
            ExitCode::Success
        }
        Err(err) => {
            if let CliError::Core(LaminarError::BatchValidation(validation)) = &err {
                let issues: Vec<String> = validation
                    .issues
                    .iter()
                    .map(|issue| issue.to_string())
                    .collect();
                output.display_validation_errors(&issues);
            }
            let failed_output = output_from_error(&err, &command);
            output.complete(&failed_output);
            exit_code_for_error(&err)
        }
    }
}

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    let detected_mode = detect_mode_from_raw_args(&raw_args);
    let cli = match Cli::try_parse_from(&raw_args) {
        Ok(parsed) => parsed,
        Err(err) => {
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = err.print();
                std::process::exit(0);
            }

            if detected_mode == Mode::Agent {
                let cli_error = clap_error_to_cli_error(&err);
                let operation = infer_operation_from_raw_args(&raw_args);
                let failed_output = output_from_error(&cli_error, operation);
                let mut output = agent::AgentOutputHandler::new(true);
                output.complete(&failed_output);
                std::process::exit(exit_code_for_error(&cli_error).as_i32());
            }

            let _ = err.print();
            std::process::exit(ExitCode::ConfigError.as_i32());
        }
    };
    let code = run(cli);
    std::process::exit(code.as_i32());
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use clap::Parser;
    use laminar_core::error::TaxonomyCode;
    use laminar_core::LaminarError;
    use serde_json::Value;

    use crate::output::{BatchSummary, CommandOutput, OutputError, OutputHandler, QrInfo};

    use super::{
        clap_error_to_cli_error, detect_mode_from_raw_args, dispatch, exit_code_for_error,
        output_from_error, Cli, CliError, Commands, ExitCode, OutputFormat,
    };

    struct MockOutput {
        confirm_calls: usize,
    }

    impl MockOutput {
        fn new() -> Self {
            Self { confirm_calls: 0 }
        }
    }

    impl OutputHandler for MockOutput {
        fn start_operation(&mut self, _operation: &str) {}
        fn progress(&mut self, _message: &str) {}
        fn display_batch_summary(&mut self, _summary: &BatchSummary) {}
        fn display_validation_errors(&mut self, _errors: &[String]) {}
        fn confirm_proceed(&mut self, _prompt: &str) -> Result<bool, OutputError> {
            self.confirm_calls += 1;
            Ok(false)
        }
        fn display_qr_info(&mut self, _info: &QrInfo) {}
        fn complete(&mut self, _output: &CommandOutput) {}
    }

    fn unique_path(prefix: &str, extension: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "{prefix}-{}-{}.{}",
            std::process::id(),
            nanos,
            extension
        ))
    }

    fn write_valid_csv() -> PathBuf {
        let path = unique_path("laminar-agent-valid", "csv");
        fs::write(
            &path,
            "address,amount,memo,label\n\
t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,1,coffee,alice\n\
t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,2,tea,bob\n",
        )
        .unwrap();
        path
    }

    #[test]
    fn cli_parses_validate_with_machine_flags() {
        let cli = Cli::try_parse_from([
            "laminar",
            "--output",
            "json",
            "--quiet",
            "validate",
            "batch.csv",
            "--network",
            "mainnet",
        ])
        .unwrap();

        assert_eq!(cli.output_format, Some(OutputFormat::Json));
        assert!(cli.quiet);
        match cli.command {
            Commands::Validate { file, network } => {
                assert_eq!(file, PathBuf::from("batch.csv"));
                assert_eq!(network, "mainnet");
            }
            _ => panic!("expected validate command"),
        }
    }

    #[test]
    fn cli_parses_construct_with_output_dir() {
        let cli = Cli::try_parse_from([
            "laminar",
            "--interactive",
            "--force",
            "--no-color",
            "construct",
            "batch.json",
            "--network",
            "testnet",
            "--output-dir",
            "artifacts",
        ])
        .unwrap();

        assert!(cli.interactive);
        assert!(cli.force);
        assert!(cli.no_color);
        match cli.command {
            Commands::Construct {
                file,
                network,
                output_dir,
            } => {
                assert_eq!(file, PathBuf::from("batch.json"));
                assert_eq!(network, "testnet");
                assert_eq!(output_dir, Some(PathBuf::from("artifacts")));
            }
            _ => panic!("expected construct command"),
        }
    }

    #[test]
    fn cli_parses_generate_minimal() {
        let cli =
            Cli::try_parse_from(["laminar", "generate", "batch.json", "--network", "mainnet"])
                .unwrap();

        match cli.command {
            Commands::Generate {
                file,
                network,
                output_dir,
            } => {
                assert_eq!(file, PathBuf::from("batch.json"));
                assert_eq!(network, "mainnet");
                assert_eq!(output_dir, None);
            }
            _ => panic!("expected generate command"),
        }
    }

    #[test]
    fn global_flags_are_accepted_after_subcommand() {
        let cli = Cli::try_parse_from([
            "laminar",
            "construct",
            "batch.csv",
            "--network",
            "mainnet",
            "--force",
            "--output",
            "json",
            "--quiet",
        ])
        .unwrap();

        assert!(cli.force);
        assert!(cli.quiet);
        assert_eq!(cli.output_format, Some(OutputFormat::Json));
        match cli.command {
            Commands::Construct { file, network, .. } => {
                assert_eq!(file, PathBuf::from("batch.csv"));
                assert_eq!(network, "mainnet");
            }
            _ => panic!("expected construct command"),
        }
    }

    #[test]
    fn exit_code_validation_error_mapping() {
        let err = CliError::Core(LaminarError::taxonomy(
            TaxonomyCode::Validation1001,
            "validation",
        ));
        assert_eq!(exit_code_for_error(&err), ExitCode::ValidationError);
    }

    #[test]
    fn exit_code_config_error_mapping() {
        let err = CliError::Core(LaminarError::taxonomy(TaxonomyCode::Cli2001, "config"));
        assert_eq!(exit_code_for_error(&err), ExitCode::ConfigError);
    }

    #[test]
    fn exit_code_io_error_mapping() {
        let err = CliError::Core(LaminarError::Io(io::Error::other("io")));
        assert_eq!(exit_code_for_error(&err), ExitCode::IoError);
    }

    #[test]
    fn exit_code_internal_error_mapping() {
        let err = CliError::Core(LaminarError::Unimplemented("todo"));
        assert_eq!(exit_code_for_error(&err), ExitCode::InternalError);
    }

    #[test]
    fn exit_code_agent_confirmation_and_stdin_blocked_mapping() {
        assert_eq!(
            exit_code_for_error(&CliError::ConfirmationRequired),
            ExitCode::ConfirmationRequired
        );
        assert_eq!(
            exit_code_for_error(&CliError::StdinBlocked),
            ExitCode::StdinBlocked
        );
    }

    #[test]
    fn force_bypasses_confirmation_in_dispatch() {
        let input = write_valid_csv();
        let out_dir = unique_path("laminar-force-bypass", "dir");
        fs::create_dir_all(&out_dir).unwrap();
        let command = Commands::Construct {
            file: input,
            network: "mainnet".to_string(),
            output_dir: Some(out_dir),
        };
        let mut output = MockOutput::new();

        let result = dispatch(&command, true, &mut output);
        assert!(result.is_ok());
        assert_eq!(output.confirm_calls, 0);
    }

    #[test]
    fn agent_validate_construct_generate_all_work() {
        let input = write_valid_csv();
        let out_dir = unique_path("laminar-agent-out", "dir");
        fs::create_dir_all(&out_dir).unwrap();
        let mut output = crate::agent::AgentOutputHandler::new(false);

        let validate = dispatch(
            &Commands::Validate {
                file: input.clone(),
                network: "mainnet".to_string(),
            },
            false,
            &mut output,
        )
        .unwrap();
        assert!(validate.ok);
        let validate_json = crate::agent::render_agent_json(&validate).unwrap();
        let validate_parsed: Value = serde_json::from_str(&validate_json).unwrap();
        assert_eq!(validate_parsed["operation"], "validate");
        assert_eq!(validate_parsed["success"], true);
        assert!(validate_parsed["result"].is_null());

        let construct = dispatch(
            &Commands::Construct {
                file: input.clone(),
                network: "mainnet".to_string(),
                output_dir: Some(out_dir.clone()),
            },
            false,
            &mut output,
        )
        .unwrap();
        assert!(construct.ok);
        let construct_json = crate::agent::render_agent_json(&construct).unwrap();
        let construct_parsed: Value = serde_json::from_str(&construct_json).unwrap();
        assert_eq!(construct_parsed["operation"], "construct");
        assert_eq!(construct_parsed["success"], true);
        assert!(construct_parsed["result"]["zip321Uri"].is_string());
        assert!(construct_parsed["result"]["totalZat"].is_string());

        let generate = dispatch(
            &Commands::Generate {
                file: input.clone(),
                network: "mainnet".to_string(),
                output_dir: Some(out_dir.clone()),
            },
            false,
            &mut output,
        )
        .unwrap();
        assert!(generate.ok);
        let generate_json = crate::agent::render_agent_json(&generate).unwrap();
        let generate_parsed: Value = serde_json::from_str(&generate_json).unwrap();
        assert_eq!(generate_parsed["operation"], "generate");
        assert_eq!(generate_parsed["success"], true);
        assert!(generate_parsed["result"]["zip321Uri"].is_string());
    }

    #[test]
    fn agent_completion_for_test_vector_is_under_5_seconds() {
        let input = PathBuf::from("test-vectors/valid-simple.csv");
        let mut output = crate::agent::AgentOutputHandler::new(false);
        let out_dir = unique_path("laminar-agent-timeout", "dir");
        fs::create_dir_all(&out_dir).unwrap();
        let start = Instant::now();

        for command in [
            Commands::Validate {
                file: input.clone(),
                network: "mainnet".to_string(),
            },
            Commands::Construct {
                file: input.clone(),
                network: "mainnet".to_string(),
                output_dir: Some(out_dir.clone()),
            },
            Commands::Generate {
                file: input.clone(),
                network: "mainnet".to_string(),
                output_dir: Some(out_dir.clone()),
            },
        ] {
            let output_value = match dispatch(&command, false, &mut output) {
                Ok(value) => value,
                Err(err) => output_from_error(&err, "agent-test"),
            };
            let rendered = crate::agent::render_agent_json(&output_value).unwrap();
            let parsed: Value = serde_json::from_str(&rendered).unwrap();
            assert!(parsed.is_object());
        }

        assert!(start.elapsed().as_secs_f64() < 5.0);
    }

    #[test]
    fn deterministic_json_for_same_input() {
        let input = write_valid_csv();
        let out_dir = unique_path("laminar-agent-determinism", "dir");
        fs::create_dir_all(&out_dir).unwrap();
        let mut output = crate::agent::AgentOutputHandler::new(false);
        let command = Commands::Construct {
            file: input,
            network: "mainnet".to_string(),
            output_dir: Some(out_dir),
        };

        let first = dispatch(&command, false, &mut output).unwrap();
        let first_json = crate::agent::render_agent_json(&first).unwrap();
        let second = dispatch(&command, false, &mut output).unwrap();
        let second_json = crate::agent::render_agent_json(&second).unwrap();
        assert_eq!(first_json, second_json);
    }

    #[test]
    fn agent_error_response_contains_expected_code_name_message_details() {
        let cli_error = CliError::Core(LaminarError::taxonomy(
            TaxonomyCode::Validation1001,
            "invalid address",
        ));
        let output = output_from_error(&cli_error, "validate");
        let rendered = crate::agent::render_agent_json(&output).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], 1001);
        assert_eq!(parsed["error"]["name"], "INVALID_ADDRESS");
        assert_eq!(parsed["error"]["message"], "invalid address");
        assert!(parsed["error"]["details"].is_object());
    }

    #[test]
    fn clap_parse_errors_map_to_cli_codes() {
        let missing_required = Cli::try_parse_from(["laminar", "validate"]);
        assert!(missing_required.is_err());
        let err = missing_required.unwrap_err();
        let cli_error = clap_error_to_cli_error(&err);
        assert_eq!(exit_code_for_error(&cli_error), ExitCode::ConfigError);

        let valid = Cli::try_parse_from([
            "laminar",
            "--interactive",
            "--output",
            "json",
            "validate",
            "batch.csv",
            "--network",
            "mainnet",
        ]);
        assert!(valid.is_ok());

        let unknown_flag = Cli::try_parse_from([
            "laminar",
            "validate",
            "batch.csv",
            "--network",
            "mainnet",
            "--unknown-flag",
        ]);
        assert!(unknown_flag.is_err());
        let cli_error = clap_error_to_cli_error(&unknown_flag.unwrap_err());
        match cli_error {
            CliError::Core(LaminarError::Taxonomy(taxonomy)) => {
                assert_eq!(taxonomy.code(), 2002);
            }
            _ => panic!("expected taxonomy config error"),
        }
    }

    #[test]
    fn raw_arg_mode_detection_obeys_priority_rules() {
        let mode = detect_mode_from_raw_args(&[
            "laminar".to_string(),
            "--output".to_string(),
            "json".to_string(),
            "--interactive".to_string(),
        ]);
        assert_eq!(mode, crate::mode::Mode::Agent);
    }
}
