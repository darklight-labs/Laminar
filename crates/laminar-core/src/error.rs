use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaminarError {
    #[error("validation error [{code}]: {message}")]
    Validation { code: &'static str, message: String },

    #[error("parse error [{code}]: {message}")]
    Parse { code: &'static str, message: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unimplemented: {0}")]
    Unimplemented(&'static str),
}

pub type Result<T> = std::result::Result<T, LaminarError>;
