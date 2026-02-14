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
    pub fn success(
        command: &str,
        message: impl Into<String>,
        details: Vec<String>,
        payload: Option<serde_json::Value>,
    ) -> Self {
        Self {
            ok: true,
            command: command.to_string(),
            message: message.into(),
            details,
            payload,
        }
    }

    pub fn from_core_error(err: &laminar_core::LaminarError, command: &str) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            message: err.to_string(),
            details: vec!["See laminar-core error taxonomy for more context.".to_string()],
            payload: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchSummary {
    pub network: String,
    pub recipient_count: usize,
    pub total_zatoshis: u64,
    pub total_zec: String,
    pub segment_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct QrInfo {
    pub mode: String,
    pub frame_count: usize,
    pub frame_interval_ms: u64,
    pub payload_bytes: usize,
}

#[derive(Debug)]
pub enum OutputError {
    StdinBlocked,
    Io(std::io::Error),
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdinBlocked => write!(f, "stdin is not interactive"),
            Self::Io(err) => write!(f, "io error: {err}"),
        }
    }
}

impl std::error::Error for OutputError {}

pub trait OutputHandler {
    fn start_operation(&mut self, operation: &str);
    fn progress(&mut self, message: &str);
    fn display_batch_summary(&mut self, summary: &BatchSummary);
    fn display_validation_errors(&mut self, errors: &[String]);
    fn confirm_proceed(&mut self, prompt: &str) -> Result<bool, OutputError>;
    fn display_qr_info(&mut self, info: &QrInfo);
    fn complete(&mut self, output: &CommandOutput);
}
