use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutput {
    pub ok: bool,
    pub command: String,
    pub message: String,
    pub details: Vec<String>,
    pub payload: Option<serde_json::Value>,
}

impl CommandOutput {
    pub fn todo(command: &str) -> Self {
        // TODO: replace placeholder payloads with typed response structs.
        Self {
            ok: true,
            command: command.to_string(),
            message: format!("{command}: scaffolded (TODO implementation)"),
            details: vec!["Business logic is not implemented yet.".to_string()],
            payload: None,
        }
    }

    pub fn from_error(err: laminar_core::LaminarError, command: &str) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            message: err.to_string(),
            details: vec!["See laminar-core error taxonomy for more context.".to_string()],
            payload: None,
        }
    }
}
