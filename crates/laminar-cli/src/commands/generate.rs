use clap::Args;

use crate::output::CommandOutput;

#[derive(Debug, Clone, Args)]
pub struct GenerateArgs {
    #[arg(long)]
    pub input: String,

    #[arg(long)]
    pub out_dir: Option<String>,
}

pub fn run(_args: GenerateArgs) -> laminar_core::Result<CommandOutput> {
    // TODO: generate QR assets and receipt bundle outputs.
    Ok(CommandOutput::todo("generate"))
}
