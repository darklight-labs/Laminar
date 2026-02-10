use crate::output::CommandOutput;

pub fn render(output: &CommandOutput) {
    // TODO: version the agent schema and expose machine-readable error codes.
    match serde_json::to_string_pretty(output) {
        Ok(json) => println!("{json}"),
        Err(err) => println!(
            "{{\"ok\":false,\"message\":\"serialization failed: {}\"}}",
            err
        ),
    }
}
