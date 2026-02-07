//! Output helpers for human and agent modes.

use serde::Serialize;

/// Human (TTY) vs Agent (non-interactive) output selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Agent,
}

/// Structured error collected for a specific CSV row.
#[derive(Debug, Clone, Serialize)]
pub struct RowIssue {
    pub row: usize,
    pub field: String,
    pub message: String,
}

/// Agent-mode error payload.
#[derive(Debug, Clone, Serialize)]
pub struct AgentError {
    pub error: String,
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<RowIssue>>,
}

/// Truncate long addresses for human-readable tables.
pub fn truncate_address(addr: &str) -> String {
    let s = addr.trim();
    if s.chars().count() <= 14 {
        return s.to_string();
    }

    let start: String = s.chars().take(6).collect();
    let end: String = s
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("{start}...{end}")
}

/// Format a zatoshi amount as a ZEC string with at least 2 decimals.
pub fn format_zat_as_zec(amount_zat: u64) -> String {
    const ZAT_PER_ZEC: u64 = 100_000_000;
    let whole = amount_zat / ZAT_PER_ZEC;
    let frac = amount_zat % ZAT_PER_ZEC;

    let mut frac_str = format!("{:08}", frac);

    while frac_str.ends_with('0') && frac_str.len() > 2 {
        frac_str.pop();
    }
    if frac_str.len() < 2 {
        while frac_str.len() < 2 {
            frac_str.push('0');
        }
    }

    format!("{}.{} ZEC", whole, frac_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_address_is_unchanged() {
        assert_eq!(truncate_address("u1abc"), "u1abc");
    }

    #[test]
    fn truncate_long_ascii_address() {
        assert_eq!(truncate_address("u1abcdefghijklmnop"), "u1abcd...mnop");
    }

    #[test]
    fn truncate_long_unicode_address_without_panic() {
        let han = "\u{4F60}";
        assert_eq!(
            truncate_address(&format!(
                "u1{han}{han}{han}{han}{han}{han}{han}{han}{han}{han}{han}{han}{han}{han}"
            )),
            format!("u1{han}{han}{han}{han}...{han}{han}{han}{han}")
        );
    }
}
