mod agent;
mod commands;
mod mode;
mod operator;
mod output;

use clap::{Parser, Subcommand};

use mode::RunMode;
use output::CommandOutput;

#[derive(Debug, Parser)]
#[command(name = "laminar", version, about = "Laminar dual-mode CLI")]
struct Cli {
    /// Force machine-readable JSON output mode.
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate(commands::validate::ValidateArgs),
    Construct(commands::construct::ConstructArgs),
    Generate(commands::generate::GenerateArgs),
}

fn command_label(command: &Commands) -> &'static str {
    match command {
        Commands::Validate(_) => "validate",
        Commands::Construct(_) => "construct",
        Commands::Generate(_) => "generate",
    }
}

fn dispatch(command: Commands) -> laminar_core::Result<CommandOutput> {
    match command {
        Commands::Validate(args) => commands::validate::run(args),
        Commands::Construct(args) => commands::construct::run(args),
        Commands::Generate(args) => commands::generate::run(args),
    }
}

fn main() {
    let cli = Cli::parse();
    let mode = mode::select_mode(cli.json);
    let label = command_label(&cli.command).to_string();

    let output = match dispatch(cli.command) {
        Ok(data) => data,
        Err(err) => CommandOutput::from_error(err, &label),
    };

    match mode {
        RunMode::Operator => operator::render(&output),
        RunMode::Agent => agent::render(&output),
    }

    if !output.ok {
        std::process::exit(1);
    }
}
