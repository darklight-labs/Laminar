use clap::Args;

use crate::output::CommandOutput;

#[derive(Debug, Clone, Args)]
pub struct ConstructArgs {
    #[arg(long)]
    pub input: String,

    #[arg(long)]
    pub network: Option<String>,
}

pub fn run(_args: ConstructArgs) -> laminar_core::Result<CommandOutput> {
    // TODO: construct ZIP-321 and UR payloads from validated intents.
    Ok(CommandOutput::todo("construct"))
}
