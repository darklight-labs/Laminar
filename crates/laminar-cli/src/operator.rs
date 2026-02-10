use colored::Colorize;
use comfy_table::{presets::UTF8_BORDERS_ONLY, Table};

use crate::output::CommandOutput;

pub fn render(output: &CommandOutput) {
    // TODO: make operator output stream-friendly for large batches.
    if output.ok {
        println!("{}", output.message.green());
    } else {
        eprintln!("{}", output.message.red());
    }

    if !output.details.is_empty() {
        let mut table = Table::new();
        table.load_preset(UTF8_BORDERS_ONLY);
        table.set_header(vec!["detail"]);
        for detail in &output.details {
            table.add_row(vec![detail.as_str()]);
        }
        println!("{table}");
    }

    if let Some(payload) = &output.payload {
        let text = serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string());
        println!("{text}");
    }
}
