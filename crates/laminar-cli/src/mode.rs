use std::io::{self, IsTerminal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Operator,
    Agent,
}

pub fn detect_mode(output_json: bool, interactive: bool) -> Mode {
    detect_mode_with_terminal(output_json, interactive, io::stdout().is_terminal())
}

pub(crate) fn detect_mode_with_terminal(
    output_json: bool,
    interactive: bool,
    stdout_is_terminal: bool,
) -> Mode {
    if output_json {
        return Mode::Agent;
    }
    if interactive {
        return Mode::Operator;
    }
    if stdout_is_terminal {
        Mode::Operator
    } else {
        Mode::Agent
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_mode_with_terminal, Mode};

    #[test]
    fn output_json_has_highest_priority() {
        let mode = detect_mode_with_terminal(true, true, true);
        assert_eq!(mode, Mode::Agent);
    }

    #[test]
    fn interactive_overrides_non_terminal_stdout() {
        let mode = detect_mode_with_terminal(false, true, false);
        assert_eq!(mode, Mode::Operator);
    }

    #[test]
    fn tty_defaults_to_operator() {
        let mode = detect_mode_with_terminal(false, false, true);
        assert_eq!(mode, Mode::Operator);
    }

    #[test]
    fn pipe_defaults_to_agent() {
        let mode = detect_mode_with_terminal(false, false, false);
        assert_eq!(mode, Mode::Agent);
    }

    #[test]
    fn mode_detection_is_deterministic_for_same_inputs() {
        let first = detect_mode_with_terminal(false, false, true);
        let second = detect_mode_with_terminal(false, false, true);
        let third = detect_mode_with_terminal(false, false, true);
        assert_eq!(first, second);
        assert_eq!(second, third);
    }
}
