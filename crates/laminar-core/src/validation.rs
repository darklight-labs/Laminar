use std::collections::HashMap;
use std::fmt;

use zcash_address::{TryFromAddress, ZcashAddress};
use zcash_protocol::consensus::NetworkType;

use crate::error::{AmountError, LaminarError, Result, TaxonomyCode};
use crate::types::{BatchConfig, Network, Recipient, Zatoshi, ZATOSHI_MAX, ZATOSHI_MIN};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRow {
    pub row_number: usize,
    pub address: String,
    pub amount_zec: Option<String>,
    pub amount_zatoshis: Option<String>,
    pub memo: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientAddressType {
    Unified,
    Sapling,
    Transparent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRecipient {
    pub row_number: usize,
    pub address_type: RecipientAddressType,
    pub recipient: Recipient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedBatch {
    pub recipients: Vec<ValidatedRecipient>,
    pub total: Zatoshi,
    pub network: Network,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchValidationIssue {
    pub code: TaxonomyCode,
    pub row_number: Option<usize>,
    pub column: Option<String>,
    pub message: String,
}

impl BatchValidationIssue {
    pub fn new(
        code: TaxonomyCode,
        row_number: Option<usize>,
        column: Option<impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            row_number,
            column: column.map(Into::into),
            message: message.into(),
        }
    }
}

impl fmt::Display for BatchValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.row_number, &self.column) {
            (Some(row), Some(column)) => write!(
                f,
                "[{}:{}] row {} {}: {}",
                self.code.code(),
                self.code.name(),
                row,
                column,
                self.message
            ),
            (Some(row), None) => write!(
                f,
                "[{}:{}] row {}: {}",
                self.code.code(),
                self.code.name(),
                row,
                self.message
            ),
            (None, Some(column)) => write!(
                f,
                "[{}:{}] {}: {}",
                self.code.code(),
                self.code.name(),
                column,
                self.message
            ),
            (None, None) => write!(
                f,
                "[{}:{}] {}",
                self.code.code(),
                self.code.name(),
                self.message
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchValidationError {
    pub issues: Vec<BatchValidationIssue>,
}

impl fmt::Display for BatchValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "batch validation failed with {} issue(s):",
            self.issues.len()
        )?;
        for issue in &self.issues {
            writeln!(f, " - {issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BatchValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedAddressMeta {
    network: NetworkType,
    kind: RecipientAddressType,
}

impl TryFromAddress for ParsedAddressMeta {
    type Error = &'static str;

    fn try_from_sprout(
        _net: NetworkType,
        _data: [u8; 64],
    ) -> std::result::Result<Self, zcash_address::ConversionError<Self::Error>> {
        Err(zcash_address::ConversionError::User(
            "sprout addresses are not supported",
        ))
    }

    fn try_from_sapling(
        net: NetworkType,
        _data: [u8; 43],
    ) -> std::result::Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            network: net,
            kind: RecipientAddressType::Sapling,
        })
    }

    fn try_from_unified(
        net: NetworkType,
        _data: zcash_address::unified::Address,
    ) -> std::result::Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            network: net,
            kind: RecipientAddressType::Unified,
        })
    }

    fn try_from_transparent_p2pkh(
        net: NetworkType,
        _data: [u8; 20],
    ) -> std::result::Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            network: net,
            kind: RecipientAddressType::Transparent,
        })
    }

    fn try_from_transparent_p2sh(
        net: NetworkType,
        _data: [u8; 20],
    ) -> std::result::Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            network: net,
            kind: RecipientAddressType::Transparent,
        })
    }

    fn try_from_tex(
        net: NetworkType,
        _data: [u8; 20],
    ) -> std::result::Result<Self, zcash_address::ConversionError<Self::Error>> {
        Ok(Self {
            network: net,
            kind: RecipientAddressType::Transparent,
        })
    }
}

pub fn validate_batch(rows: Vec<RawRow>, config: &BatchConfig) -> Result<ValidatedBatch> {
    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    let mut validated = Vec::new();
    let mut seen_rows_by_address: HashMap<String, usize> = HashMap::new();

    if rows.is_empty() {
        issues.push(BatchValidationIssue::new(
            TaxonomyCode::Validation1012,
            None,
            None::<String>,
            "batch has no rows",
        ));
    }

    let batch_limit = if config.max_recipients == 0 {
        500
    } else {
        config.max_recipients.min(500)
    };
    if rows.len() > batch_limit {
        issues.push(BatchValidationIssue::new(
            TaxonomyCode::Validation1011,
            None,
            None::<String>,
            format!(
                "batch has {} rows; max allowed is {}",
                rows.len(),
                batch_limit
            ),
        ));
    }

    let mut total: Option<Zatoshi> = None;
    for row in rows {
        let address_result = validate_address_row(&row, config.network);
        let amount_result = validate_amount_row(&row);
        let memo_result = validate_memo_row(&row);

        if let Err(issue) = &address_result {
            issues.push(issue.clone());
        }
        if let Err(issue) = &amount_result {
            issues.push(issue.clone());
        }
        if let Err(issue) = &memo_result {
            issues.push(issue.clone());
        }

        let (address_type, amount, mut memo) = match (address_result, amount_result, memo_result) {
            (Ok(address_type), Ok(amount), Ok(memo)) => (address_type, amount, memo),
            _ => continue,
        };

        if matches!(address_type, RecipientAddressType::Transparent)
            && memo.as_ref().is_some_and(|value| !value.is_empty())
        {
            warnings.push(format!(
                "row {}: memo ignored for transparent address (transparent recipients do not support memos)",
                row.row_number
            ));
            memo = None;
        }

        let normalized_address = row.address.trim().to_string();
        if let Some(first_row) = seen_rows_by_address.get(&normalized_address).copied() {
            warnings.push(format!(
                "duplicate address '{}' at row {} (first seen at row {})",
                normalized_address, row.row_number, first_row
            ));
        } else {
            seen_rows_by_address.insert(normalized_address.clone(), row.row_number);
        }

        match total {
            Some(current) => match current.checked_add(amount) {
                Ok(next) => total = Some(next),
                Err(_) => issues.push(BatchValidationIssue::new(
                    TaxonomyCode::Validation1013,
                    Some(row.row_number),
                    Some("amount"),
                    "batch total overflow while summing zatoshis",
                )),
            },
            None => total = Some(amount),
        }

        validated.push(ValidatedRecipient {
            row_number: row.row_number,
            address_type,
            recipient: Recipient {
                address: normalized_address,
                amount,
                memo,
                label: row.label.map(|value| value.trim().to_string()),
            },
        });
    }

    if !issues.is_empty() {
        return Err(BatchValidationError { issues }.into());
    }

    let total = total.ok_or_else(|| BatchValidationError {
        issues: vec![BatchValidationIssue::new(
            TaxonomyCode::Validation1012,
            None,
            None::<String>,
            "batch has no valid recipients",
        )],
    })?;

    Ok(ValidatedBatch {
        recipients: validated,
        total,
        network: config.network,
        warnings,
    })
}

pub fn validate_address(address: &str, network: Network) -> Result<RecipientAddressType> {
    let row = RawRow {
        row_number: 1,
        address: address.to_string(),
        amount_zec: None,
        amount_zatoshis: Some("1".to_string()),
        memo: None,
        label: None,
    };
    validate_address_row(&row, network).map_err(|issue| {
        BatchValidationError {
            issues: vec![issue],
        }
        .into()
    })
}

pub fn validate_amount(amount: Zatoshi) -> Result<()> {
    if amount.as_u64() < ZATOSHI_MIN {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1002,
            "amount must be greater than zero",
        ));
    }
    if amount.as_u64() > ZATOSHI_MAX {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1002,
            "amount exceeds maximum supply",
        ));
    }
    Ok(())
}

pub fn validate_memo(memo: Option<&str>) -> Result<()> {
    if let Some(value) = memo {
        if value.len() > 512 {
            return Err(LaminarError::taxonomy(
                TaxonomyCode::Validation1004,
                "memo exceeds 512 bytes",
            ));
        }
    }

    Ok(())
}

fn validate_address_row(
    row: &RawRow,
    expected_network: Network,
) -> std::result::Result<RecipientAddressType, BatchValidationIssue> {
    let address = row.address.trim();
    if address.is_empty() {
        return Err(BatchValidationIssue::new(
            TaxonomyCode::Validation1001,
            Some(row.row_number),
            Some("address"),
            "address is empty",
        ));
    }

    let parsed = ZcashAddress::try_from_encoded(address).map_err(|err| {
        BatchValidationIssue::new(
            TaxonomyCode::Validation1001,
            Some(row.row_number),
            Some("address"),
            format!("invalid address encoding: {err}"),
        )
    })?;

    let metadata = parsed.convert::<ParsedAddressMeta>().map_err(|err| {
        BatchValidationIssue::new(
            TaxonomyCode::Validation1001,
            Some(row.row_number),
            Some("address"),
            format!("unsupported address type: {err}"),
        )
    })?;

    let expected = network_type(expected_network);
    if metadata.network != expected {
        return Err(BatchValidationIssue::new(
            TaxonomyCode::Validation1005,
            Some(row.row_number),
            Some("address"),
            format!(
                "network mismatch: expected {:?}, got {:?}",
                expected, metadata.network
            ),
        ));
    }

    match (metadata.kind, expected_network) {
        (RecipientAddressType::Unified, Network::Mainnet) if !address.starts_with("u1") => {
            return Err(BatchValidationIssue::new(
                TaxonomyCode::Validation1001,
                Some(row.row_number),
                Some("address"),
                "invalid unified mainnet prefix (expected u1)",
            ));
        }
        (RecipientAddressType::Unified, Network::Testnet) if !address.starts_with("utest1") => {
            return Err(BatchValidationIssue::new(
                TaxonomyCode::Validation1001,
                Some(row.row_number),
                Some("address"),
                "invalid unified testnet prefix (expected utest1)",
            ));
        }
        (RecipientAddressType::Sapling, Network::Mainnet) if !address.starts_with("zs") => {
            return Err(BatchValidationIssue::new(
                TaxonomyCode::Validation1001,
                Some(row.row_number),
                Some("address"),
                "invalid sapling mainnet prefix (expected zs)",
            ));
        }
        (RecipientAddressType::Sapling, Network::Testnet)
            if !address.starts_with("ztestsapling") =>
        {
            return Err(BatchValidationIssue::new(
                TaxonomyCode::Validation1001,
                Some(row.row_number),
                Some("address"),
                "invalid sapling testnet prefix (expected ztestsapling)",
            ));
        }
        (RecipientAddressType::Transparent, Network::Mainnet)
            if !(address.starts_with("t1") || address.starts_with("t3")) =>
        {
            return Err(BatchValidationIssue::new(
                TaxonomyCode::Validation1001,
                Some(row.row_number),
                Some("address"),
                "invalid transparent mainnet prefix (expected t1 or t3)",
            ));
        }
        _ => {}
    }

    Ok(metadata.kind)
}

fn validate_amount_row(row: &RawRow) -> std::result::Result<Zatoshi, BatchValidationIssue> {
    let parsed = if let Some(zat) = row.amount_zatoshis.as_deref().map(str::trim) {
        if zat.is_empty() {
            Err(AmountError::EmptyInput)
        } else {
            parse_zatoshis_str(zat)
        }
    } else if let Some(zec) = row.amount_zec.as_deref().map(str::trim) {
        if zec.is_empty() {
            Err(AmountError::EmptyInput)
        } else {
            Zatoshi::from_zec_str(zec)
        }
    } else {
        Err(AmountError::EmptyInput)
    };

    parsed.map_err(|err| {
        BatchValidationIssue::new(
            TaxonomyCode::Validation1002,
            Some(row.row_number),
            Some("amount"),
            format!("invalid amount: {err}"),
        )
    })
}

fn validate_memo_row(row: &RawRow) -> std::result::Result<Option<String>, BatchValidationIssue> {
    match row.memo.as_deref() {
        None => Ok(None),
        Some(memo) => {
            if memo.len() > 512 {
                Err(BatchValidationIssue::new(
                    TaxonomyCode::Validation1004,
                    Some(row.row_number),
                    Some("memo"),
                    "memo exceeds 512 bytes",
                ))
            } else {
                Ok(Some(memo.to_string()))
            }
        }
    }
}

fn parse_zatoshis_str(value: &str) -> std::result::Result<Zatoshi, AmountError> {
    if value.starts_with('-') {
        return Err(AmountError::NegativeNotAllowed);
    }
    if value.is_empty() {
        return Err(AmountError::EmptyInput);
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AmountError::InvalidNumeric);
    }

    let parsed = value.parse::<u64>().map_err(|_| AmountError::Overflow)?;
    Zatoshi::new(parsed)
}

fn network_type(network: Network) -> NetworkType {
    match network {
        Network::Mainnet => NetworkType::Main,
        Network::Testnet => NetworkType::Test,
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{BatchConfig, Network, Zatoshi};

    use super::{
        validate_address, validate_amount, validate_batch, validate_memo, BatchValidationError,
        RawRow, RecipientAddressType,
    };

    const MAINNET_TADDR: &str = "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs";
    const MAINNET_SAPLING: &str =
        "zs1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq6d8g";
    const MAINNET_UNIFIED: &str =
        "u1qpatys4zruk99pg59gcscrt7y6akvl9vrhcfyhm9yxvxz7h87q6n8cgrzzpe9zru68uq39uhmlpp5uefxu0su5uqyqfe5zp3tycn0ecl";
    const TESTNET_TADDR: &str = "tm9iMLAuYMzJ6jtFLcA7rzUmfreGuKvr7Ma";
    const TESTNET_SAPLING: &str =
        "ztestsapling1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqfhgwqu";
    const TESTNET_UNIFIED: &str =
        "utest10c5kutapazdnf8ztl3pu43nkfsjx89fy3uuff8tsmxm6s86j37pe7uz94z5jhkl49pqe8yz75rlsaygexk6jpaxwx0esjr8wm5ut7d5s";

    fn config() -> BatchConfig {
        BatchConfig {
            network: Network::Mainnet,
            max_recipients: 500,
            source_file: "batch.csv".to_string(),
        }
    }

    fn err_issues(err: BatchValidationError) -> Vec<(u16, Option<usize>, Option<String>)> {
        err.issues
            .into_iter()
            .map(|issue| (issue.code.code(), issue.row_number, issue.column))
            .collect()
    }

    fn laminar_taxonomy_codes(err: crate::error::LaminarError) -> Vec<u16> {
        match err {
            crate::error::LaminarError::Taxonomy(taxonomy) => vec![taxonomy.code()],
            crate::error::LaminarError::BatchValidation(batch) => batch
                .issues
                .into_iter()
                .map(|issue| issue.code.code())
                .collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn all_valid_rows_succeed() {
        let rows = vec![
            RawRow {
                row_number: 1,
                address: MAINNET_TADDR.to_string(),
                amount_zec: Some("1.5".to_string()),
                amount_zatoshis: None,
                memo: Some("ok".to_string()),
                label: Some("alice".to_string()),
            },
            RawRow {
                row_number: 2,
                address: MAINNET_SAPLING.to_string(),
                amount_zec: None,
                amount_zatoshis: Some("1000".to_string()),
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 3,
                address: MAINNET_UNIFIED.to_string(),
                amount_zec: Some("0.00000001".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
        ];

        let batch = validate_batch(rows, &config()).unwrap();
        assert_eq!(batch.recipients.len(), 3);
        assert_eq!(batch.total.as_u64(), 150_001_001);
    }

    #[test]
    fn one_invalid_row_rejects_entire_batch() {
        let rows = vec![
            RawRow {
                row_number: 1,
                address: MAINNET_TADDR.to_string(),
                amount_zec: Some("1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 2,
                address: "not-an-address".to_string(),
                amount_zec: Some("1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
        ];

        let err = validate_batch(rows, &config()).unwrap_err();
        match err {
            crate::error::LaminarError::BatchValidation(batch_err) => {
                let issues = err_issues(batch_err);
                assert!(issues
                    .iter()
                    .any(|(code, row, _)| *code == 1001 && *row == Some(2)));
            }
            other => panic!("expected batch validation error, got {other}"),
        }
    }

    #[test]
    fn collects_all_errors_across_rows() {
        let rows = vec![
            RawRow {
                row_number: 1,
                address: "not-an-address".to_string(),
                amount_zec: Some("abc".to_string()),
                amount_zatoshis: None,
                memo: Some("x".repeat(513)),
                label: None,
            },
            RawRow {
                row_number: 2,
                address: "".to_string(),
                amount_zec: None,
                amount_zatoshis: None,
                memo: Some("x".repeat(600)),
                label: None,
            },
        ];

        let err = validate_batch(rows, &config()).unwrap_err();
        match err {
            crate::error::LaminarError::BatchValidation(batch_err) => {
                assert!(batch_err.issues.len() >= 5);
                assert!(batch_err
                    .issues
                    .iter()
                    .any(|issue| issue.code.code() == 1001));
                assert!(batch_err
                    .issues
                    .iter()
                    .any(|issue| issue.code.code() == 1002));
                assert!(batch_err
                    .issues
                    .iter()
                    .any(|issue| issue.code.code() == 1004));
            }
            other => panic!("expected batch validation error, got {other}"),
        }
    }

    #[test]
    fn duplicate_addresses_produce_warning_not_error() {
        let rows = vec![
            RawRow {
                row_number: 1,
                address: MAINNET_TADDR.to_string(),
                amount_zec: Some("1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 2,
                address: MAINNET_TADDR.to_string(),
                amount_zec: Some("2".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
        ];

        let batch = validate_batch(rows, &config()).unwrap();
        assert_eq!(batch.warnings.len(), 1);
        assert!(batch.warnings[0].contains("duplicate address"));
    }

    #[test]
    fn overflow_detected_with_1013() {
        let rows = vec![
            RawRow {
                row_number: 1,
                address: MAINNET_TADDR.to_string(),
                amount_zec: None,
                amount_zatoshis: Some(
                    Zatoshi::new(crate::types::ZATOSHI_MAX)
                        .unwrap()
                        .as_u64()
                        .to_string(),
                ),
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 2,
                address: MAINNET_SAPLING.to_string(),
                amount_zec: None,
                amount_zatoshis: Some("1".to_string()),
                memo: None,
                label: None,
            },
        ];

        let err = validate_batch(rows, &config()).unwrap_err();
        match err {
            crate::error::LaminarError::BatchValidation(batch_err) => {
                assert!(batch_err
                    .issues
                    .iter()
                    .any(|issue| issue.code.code() == 1013));
            }
            other => panic!("expected batch validation error, got {other}"),
        }
    }

    #[test]
    fn memo_512_valid_memo_513_invalid_1004() {
        let rows_ok = vec![RawRow {
            row_number: 1,
            address: MAINNET_TADDR.to_string(),
            amount_zec: Some("1".to_string()),
            amount_zatoshis: None,
            memo: Some("a".repeat(512)),
            label: None,
        }];
        assert!(validate_batch(rows_ok, &config()).is_ok());

        let rows_bad = vec![RawRow {
            row_number: 1,
            address: MAINNET_TADDR.to_string(),
            amount_zec: Some("1".to_string()),
            amount_zatoshis: None,
            memo: Some("a".repeat(513)),
            label: None,
        }];
        let err = validate_batch(rows_bad, &config()).unwrap_err();
        match err {
            crate::error::LaminarError::BatchValidation(batch_err) => {
                assert!(batch_err
                    .issues
                    .iter()
                    .any(|issue| issue.code.code() == 1004));
            }
            other => panic!("expected batch validation error, got {other}"),
        }
    }

    #[test]
    fn helper_functions_cover_success_and_failure_paths() {
        let address_type = validate_address(MAINNET_TADDR, Network::Mainnet).unwrap();
        assert_eq!(address_type, RecipientAddressType::Transparent);

        let mismatch = validate_address(TESTNET_TADDR, Network::Mainnet).unwrap_err();
        assert!(laminar_taxonomy_codes(mismatch).contains(&1005));

        assert!(validate_amount(Zatoshi::new(1).unwrap()).is_ok());
        assert!(validate_memo(None).is_ok());
        assert!(validate_memo(Some("ok")).is_ok());
        let memo_err = validate_memo(Some(&"x".repeat(513))).unwrap_err();
        assert!(laminar_taxonomy_codes(memo_err).contains(&1004));
    }

    #[test]
    fn empty_rows_and_recipient_limit_are_rejected() {
        let empty_err = validate_batch(Vec::new(), &config()).unwrap_err();
        assert!(laminar_taxonomy_codes(empty_err).contains(&1012));

        let small_limit = BatchConfig {
            network: Network::Mainnet,
            max_recipients: 1,
            source_file: "batch.csv".to_string(),
        };
        let rows = vec![
            RawRow {
                row_number: 1,
                address: MAINNET_TADDR.to_string(),
                amount_zec: Some("1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 2,
                address: MAINNET_TADDR.to_string(),
                amount_zec: Some("1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
        ];
        let limit_err = validate_batch(rows, &small_limit).unwrap_err();
        assert!(laminar_taxonomy_codes(limit_err).contains(&1011));
    }

    #[test]
    fn invalid_zatoshi_strings_are_rejected() {
        let rows = vec![
            RawRow {
                row_number: 1,
                address: MAINNET_TADDR.to_string(),
                amount_zec: None,
                amount_zatoshis: Some("-1".to_string()),
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 2,
                address: MAINNET_TADDR.to_string(),
                amount_zec: None,
                amount_zatoshis: Some("abc".to_string()),
                memo: None,
                label: None,
            },
        ];
        let err = validate_batch(rows, &config()).unwrap_err();
        let codes = laminar_taxonomy_codes(err);
        assert!(codes.iter().all(|code| *code == 1002));
    }

    #[test]
    fn testnet_addresses_validate_under_testnet_config() {
        let config = BatchConfig {
            network: Network::Testnet,
            max_recipients: 500,
            source_file: "batch.csv".to_string(),
        };
        let rows = vec![
            RawRow {
                row_number: 1,
                address: TESTNET_TADDR.to_string(),
                amount_zec: Some("0.1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 2,
                address: TESTNET_SAPLING.to_string(),
                amount_zec: Some("0.2".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 3,
                address: TESTNET_UNIFIED.to_string(),
                amount_zec: Some("0.3".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
        ];
        let validated = validate_batch(rows, &config).unwrap();
        assert_eq!(validated.recipients.len(), 3);
        assert_eq!(validated.network, Network::Testnet);
    }

    #[test]
    fn mixed_network_addresses_fail_validation() {
        let rows = vec![
            RawRow {
                row_number: 1,
                address: MAINNET_TADDR.to_string(),
                amount_zec: Some("1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
            RawRow {
                row_number: 2,
                address: TESTNET_TADDR.to_string(),
                amount_zec: Some("1".to_string()),
                amount_zatoshis: None,
                memo: None,
                label: None,
            },
        ];
        let err = validate_batch(rows, &config()).unwrap_err();
        assert!(laminar_taxonomy_codes(err).contains(&1005));
    }

    #[test]
    fn transparent_memo_is_dropped_with_warning() {
        let rows = vec![RawRow {
            row_number: 1,
            address: MAINNET_TADDR.to_string(),
            amount_zec: Some("1".to_string()),
            amount_zatoshis: None,
            memo: Some("should-not-be-on-transparent".to_string()),
            label: None,
        }];

        let validated = validate_batch(rows, &config()).unwrap();
        assert_eq!(validated.recipients.len(), 1);
        assert_eq!(validated.recipients[0].recipient.memo, None);
        assert!(validated
            .warnings
            .iter()
            .any(|warning| warning.contains("memo ignored for transparent address")));
    }
}
