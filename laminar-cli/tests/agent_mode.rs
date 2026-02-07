use std::io::Write;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::NamedTempFile;

fn run_agent(csv_rows: &[&str], network: &str) -> Output {
    let mut csv_file = NamedTempFile::new().expect("failed to create temp csv");
    writeln!(csv_file, "address,amount,memo").expect("failed to write csv header");
    for row in csv_rows {
        writeln!(csv_file, "{row}").expect("failed to write csv row");
    }
    csv_file.flush().expect("failed to flush csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("laminar-cli"));
    cmd.arg("--input")
        .arg(csv_file.path())
        .arg("--output")
        .arg("json")
        .arg("--force")
        .arg("--network")
        .arg(network);
    cmd.output().expect("failed to run laminar-cli")
}

fn parse_agent_error(output: &Output) -> Value {
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8");
    serde_json::from_str(stderr.trim()).expect("stderr should contain JSON error payload")
}

#[test]
fn rejects_mainnet_prefix_when_testnet_selected() {
    let output = run_agent(&["u1mainnetaddr123456,1,ok"], "testnet");
    assert_eq!(output.status.code(), Some(1));

    let payload = parse_agent_error(&output);
    assert_eq!(payload["error"], "validation_failed");

    let details = payload["details"]
        .as_array()
        .expect("details should be an array");
    assert!(details.iter().any(|issue| {
        issue["field"] == "address"
            && issue["message"]
                .as_str()
                .map(|m| m.contains("selected network"))
                .unwrap_or(false)
    }));
}

#[test]
fn malformed_amount_reports_single_specific_error() {
    let output = run_agent(&["u1mainnetaddr123456,-1.00,ok"], "mainnet");
    assert_eq!(output.status.code(), Some(1));

    let payload = parse_agent_error(&output);
    let details = payload["details"]
        .as_array()
        .expect("details should be an array");

    let amount_issues: Vec<&Value> = details
        .iter()
        .filter(|issue| issue["field"] == "amount")
        .collect();

    assert_eq!(amount_issues.len(), 1);
    assert!(amount_issues[0]["message"]
        .as_str()
        .map(|m| m.contains("sign"))
        .unwrap_or(false));
    assert!(!details.iter().any(|issue| {
        issue["message"]
            .as_str()
            .map(|m| m == "amount must be greater than 0")
            .unwrap_or(false)
    }));
}

#[test]
fn unicode_address_is_rejected_without_panic() {
    let output = run_agent(
        &["u1\u{4F60}\u{4F60}\u{4F60}\u{4F60}\u{4F60}\u{4F60}\u{4F60}\u{4F60},1,ok"],
        "mainnet",
    );
    assert_eq!(output.status.code(), Some(1));

    let payload = parse_agent_error(&output);
    let details = payload["details"]
        .as_array()
        .expect("details should be an array");
    assert!(details.iter().any(|issue| {
        issue["field"] == "address"
            && issue["message"]
                .as_str()
                .map(|m| m.contains("invalid characters"))
                .unwrap_or(false)
    }));
}
