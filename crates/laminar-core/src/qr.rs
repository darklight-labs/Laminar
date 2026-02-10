use crate::error::{LaminarError, Result};

pub fn generate_qr_png(_data: &str, _size: u32) -> Result<Vec<u8>> {
    // TODO: render deterministic QR PNG bytes with selectable error correction level.
    Err(LaminarError::Unimplemented("qr::generate_qr_png"))
}
