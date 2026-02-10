use std::io::{self, IsTerminal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Operator,
    Agent,
}

pub fn select_mode(force_json: bool) -> RunMode {
    // TODO: refine mode selection with explicit env controls.
    if force_json || !io::stdout().is_terminal() {
        RunMode::Agent
    } else {
        RunMode::Operator
    }
}
