use crate::error::{LaminarError, Result};
use crate::types::TransactionIntent;

pub fn parse_json_str(_json_input: &str) -> Result<TransactionIntent> {
    // TODO: parse JSON payloads into transaction intents.
    Err(LaminarError::Unimplemented("json_parser::parse_json_str"))
}
