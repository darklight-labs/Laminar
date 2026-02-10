use crate::error::{LaminarError, Result};
use crate::types::TransactionIntent;

pub fn build_payment_request(_intent: &TransactionIntent) -> Result<String> {
    // TODO: implement ZIP-321 URI construction and canonical ordering.
    Err(LaminarError::Unimplemented("zip321::build_payment_request"))
}
