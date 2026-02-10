use crate::error::{LaminarError, Result};
use crate::types::TransactionIntent;

pub fn parse_csv_str(_csv_input: &str) -> Result<TransactionIntent> {
    // TODO: parse CSV rows into recipients and detect network conflicts.
    Err(LaminarError::Unimplemented("csv_parser::parse_csv_str"))
}
