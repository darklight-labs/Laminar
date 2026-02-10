use crate::error::{LaminarError, Result};

pub fn encode_ur_fragments(_payload: &[u8], _max_fragment_len: usize) -> Result<Vec<String>> {
    // TODO: encode payload using UR framing suitable for animated QR handoff.
    Err(LaminarError::Unimplemented("ur_encoder::encode_ur_fragments"))
}
