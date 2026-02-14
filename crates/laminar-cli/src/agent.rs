use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::output::{BatchSummary, CommandOutput, OutputError, OutputHandler, QrInfo};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentError {
    pub code: u16,
    pub name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentResponse {
    pub success: bool,
    pub laminar_version: String,
    pub mode: String,
    pub operation: String,
    pub timestamp: String,
    pub result: Option<Value>,
    pub error: Option<AgentError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

pub struct AgentOutputHandler;

impl AgentOutputHandler {
    pub fn new(_quiet: bool) -> Self {
        Self
    }
}

pub(crate) fn build_agent_response(output: &CommandOutput) -> AgentResponse {
    let payload = output.payload.as_ref();
    let timestamp = payload
        .and_then(|p| p.get("timestamp"))
        .and_then(Value::as_str)
        .unwrap_or("1970-01-01T00:00:00Z")
        .to_string();
    let warnings = payload
        .and_then(|p| p.get("warnings"))
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .filter(|entries| !entries.is_empty());

    let result = payload
        .and_then(|p| p.get("result"))
        .cloned()
        .filter(|value| !value.is_null());

    let error = payload
        .and_then(|p| p.get("error"))
        .and_then(|v| serde_json::from_value::<AgentError>(v.clone()).ok())
        .or_else(|| {
            if output.ok {
                None
            } else {
                Some(AgentError {
                    code: 9999,
                    name: "GENERIC_9999".to_string(),
                    message: output.message.clone(),
                    details: if output.details.is_empty() {
                        None
                    } else {
                        Some(json!({ "messages": output.details }))
                    },
                })
            }
        });

    AgentResponse {
        success: output.ok,
        laminar_version: env!("CARGO_PKG_VERSION").to_string(),
        mode: "agent".to_string(),
        operation: output.command.clone(),
        timestamp,
        result,
        error,
        warnings,
    }
}

fn sort_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = serde_json::Map::new();
            for (key, value) in entries {
                sorted.insert(key, sort_json_value(value));
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(sort_json_value).collect()),
        other => other,
    }
}

pub(crate) fn render_agent_json(output: &CommandOutput) -> Result<String, serde_json::Error> {
    let response = build_agent_response(output);
    let value = serde_json::to_value(response)?;
    let sorted = sort_json_value(value);
    serde_json::to_string_pretty(&sorted)
}

impl OutputHandler for AgentOutputHandler {
    fn start_operation(&mut self, _operation: &str) {}

    fn progress(&mut self, _message: &str) {}

    fn display_batch_summary(&mut self, _summary: &BatchSummary) {}

    fn display_validation_errors(&mut self, _errors: &[String]) {}

    fn confirm_proceed(&mut self, _prompt: &str) -> Result<bool, OutputError> {
        Ok(true)
    }

    fn display_qr_info(&mut self, _info: &QrInfo) {}

    fn complete(&mut self, output: &CommandOutput) {
        match render_agent_json(output) {
            Ok(json) => println!("{json}"),
            Err(err) => println!(
                "{{\"error\":{{\"code\":9999,\"details\":{{\"reason\":\"serialization failed\"}},\"message\":\"serialization failed: {}\",\"name\":\"GENERIC_9999\"}},\"laminar_version\":\"{}\",\"mode\":\"agent\",\"operation\":\"{}\",\"result\":null,\"success\":false,\"timestamp\":\"1970-01-01T00:00:00Z\"}}",
                err,
                env!("CARGO_PKG_VERSION"),
                output.command
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::output::{CommandOutput, OutputHandler};

    use super::{render_agent_json, AgentOutputHandler};

    fn is_sorted_recursive(value: &Value) -> bool {
        match value {
            Value::Object(map) => {
                let keys: Vec<&String> = map.keys().collect();
                let mut sorted = keys.clone();
                sorted.sort();
                if keys != sorted {
                    return false;
                }
                map.values().all(is_sorted_recursive)
            }
            Value::Array(values) => values.iter().all(is_sorted_recursive),
            _ => true,
        }
    }

    #[test]
    fn confirm_proceed_is_non_blocking_and_true() {
        let mut handler = AgentOutputHandler::new(false);
        assert!(handler.confirm_proceed("Proceed?").unwrap());
    }

    #[test]
    fn agent_json_output_is_valid_json() {
        let output = CommandOutput::success(
            "validate",
            "validation completed",
            Vec::new(),
            Some(serde_json::json!({
                "timestamp": "1970-01-01T00:00:00Z",
                "result": {
                    "schemaVersion": "1.0",
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "createdAt": "2025-01-28T12:00:00Z",
                    "network": "mainnet",
                    "recipients": [],
                    "totalZat": "100",
                    "zip321Uri": "zcash:t1example?amount=0.000001",
                    "payloadBytes": 42,
                    "urEncoded": null
                }
            })),
        );
        let rendered = render_agent_json(&output).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn keys_are_sorted_alphabetically_recursive() {
        let output = CommandOutput::success(
            "validate",
            "ok",
            Vec::new(),
            Some(serde_json::json!({
                "warnings": ["duplicate"],
                "timestamp": "1970-01-01T00:00:00Z",
                "result": {
                    "zip321Uri": "zcash:...",
                    "schemaVersion": "1.0",
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "createdAt": "2025-01-28T12:00:00Z",
                    "network": "mainnet",
                    "recipients": [],
                    "totalZat": "2",
                    "payloadBytes": 128,
                    "urEncoded": null
                }
            })),
        );
        let rendered = render_agent_json(&output).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert!(is_sorted_recursive(&parsed));
    }

    #[test]
    fn error_code_is_preserved_in_error_response() {
        let output = CommandOutput {
            ok: false,
            command: "construct".to_string(),
            message: "validation failed".to_string(),
            details: vec!["detail".to_string()],
            payload: Some(serde_json::json!({
                "error": {
                    "code": 1001,
                    "name": "VALIDATION_1001",
                    "message": "bad address",
                    "details": {
                        "row": 1
                    }
                },
                "timestamp": "1970-01-01T00:00:00Z"
            })),
        };
        let rendered = render_agent_json(&output).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["error"]["code"], 1001);
        assert_eq!(parsed["error"]["name"], "VALIDATION_1001");
    }

    #[test]
    fn deterministic_rendering_for_same_input() {
        let output = CommandOutput::success(
            "generate",
            "done",
            vec!["warn".to_string()],
            Some(serde_json::json!({
                "timestamp": "1970-01-01T00:00:00Z",
                "warnings": ["warn"],
                "result": {
                    "schemaVersion": "1.0",
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "createdAt": "2025-01-28T12:00:00Z",
                    "network": "mainnet",
                    "recipients": [],
                    "totalZat": "1",
                    "zip321Uri": "zcash:...",
                    "payloadBytes": 64,
                    "urEncoded": ["ur:bytes/1-1/lftansgt"]
                }
            })),
        );

        let first = render_agent_json(&output).unwrap();
        let second = render_agent_json(&output).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn static_analysis_no_blocking_input_calls() {
        let source = include_str!("agent.rs");
        let needle = ["std", "in"].concat();
        assert!(!source.contains(&needle));
        let line_reader = ["read", "_line"].concat();
        assert!(!source.contains(&line_reader));
    }
}
