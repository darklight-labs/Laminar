use clap::Args;

use crate::output::CommandOutput;

#[derive(Debug, Clone, Args)]
pub struct ValidateArgs {
    #[arg(long)]
    pub input: String,
}

pub fn run(_args: ValidateArgs) -> laminar_core::Result<CommandOutput> {
    // TODO: dispatch CSV/JSON validation pipeline in laminar-core.
    Ok(CommandOutput::todo("validate"))
}
