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
    use super::*;

    #[test]
    fn stub_modules_are_linked() {
        let err = csv_parser::parse_csv_str("").expect_err("stub should fail");
        assert!(matches!(err, LaminarError::Unimplemented(_)));
    }
}
