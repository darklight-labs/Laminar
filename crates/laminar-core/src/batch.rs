use crate::error::{LaminarError, Result};
use crate::types::Recipient;

pub fn segment_recipients(recipients: &[Recipient], max_per_batch: usize) -> Result<Vec<Vec<Recipient>>> {
    // TODO: enforce wallet constraints and deterministic segmentation policy.
    if max_per_batch == 0 {
        return Err(LaminarError::Validation {
            code: "E_BATCH_SIZE",
            message: "max_per_batch must be greater than zero".to_string(),
        });
    }

    Ok(recipients
        .chunks(max_per_batch)
        .map(|chunk| chunk.to_vec())
        .collect())
}
