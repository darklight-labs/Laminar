pub mod batch;
pub mod csv_parser;
pub mod error;
pub mod json_parser;
pub mod qr;
pub mod receipt;
pub mod types;
pub mod ur_encoder;
pub mod validation;
pub mod zip321;

pub use error::{LaminarError, Result};

#[cfg(test)]
mod tests {
    use super::types::{Zatoshi, ZATOSHI_PER_ZEC};

    #[test]
    fn modules_are_linked() {
        let value = Zatoshi::from_zec_str("1").expect("valid amount");
        assert_eq!(value.as_u64(), ZATOSHI_PER_ZEC);
    }
}
