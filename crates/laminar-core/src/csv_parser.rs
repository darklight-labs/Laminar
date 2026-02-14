use csv::{ReaderBuilder, StringRecord};

use crate::error::{LaminarError, TaxonomyCode};
use crate::types::BatchConfig;
use crate::validation::RawRow;

const MAX_FILE_SIZE_BYTES: usize = 10 * 1024 * 1024;
const MAX_ROWS: usize = 1000;

#[derive(Debug, Default, Clone, Copy)]
struct HeaderIndexes {
    address: Option<usize>,
    amount_zec: Option<usize>,
    amount_zatoshis: Option<usize>,
    memo: Option<usize>,
    label: Option<usize>,
}

pub fn parse_csv(input: &[u8], config: &BatchConfig) -> Result<Vec<RawRow>, LaminarError> {
    if input.len() > MAX_FILE_SIZE_BYTES {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1010,
            format!(
                "CSV input '{}' exceeds maximum file size of {} bytes",
                config.source_file, MAX_FILE_SIZE_BYTES
            ),
        ));
    }

    let normalized = strip_utf8_bom(input);
    let mut reader = ReaderBuilder::new().flexible(true).from_reader(normalized);

    let headers = reader
        .headers()
        .map_err(|err| {
            LaminarError::taxonomy(
                TaxonomyCode::Validation1006,
                format!("failed reading CSV headers: {err}"),
            )
        })?
        .clone();

    let indexes = parse_headers(&headers)?;

    let mut rows = Vec::new();
    for (record_idx, record_result) in reader.records().enumerate() {
        let row_number = record_idx + 1;
        if row_number > MAX_ROWS {
            return Err(LaminarError::taxonomy(
                TaxonomyCode::Validation1011,
                format!("CSV has more than {MAX_ROWS} data rows"),
            ));
        }

        let record = record_result.map_err(|err| {
            LaminarError::taxonomy(
                TaxonomyCode::Validation1006,
                format!("failed parsing CSV row {}: {err}", row_number + 1),
            )
        })?;

        check_formula_injection(&record, row_number + 1, Some(&headers))?;

        let address = required_cell(&record, indexes.address).unwrap_or_default();
        let amount_zec = optional_cell(&record, indexes.amount_zec);
        let amount_zatoshis = optional_cell(&record, indexes.amount_zatoshis);
        let memo = optional_cell(&record, indexes.memo);
        let label = optional_cell(&record, indexes.label);

        rows.push(RawRow {
            row_number,
            address,
            amount_zec,
            amount_zatoshis,
            memo,
            label,
        });
    }

    Ok(rows)
}

fn parse_headers(headers: &StringRecord) -> Result<HeaderIndexes, LaminarError> {
    let mut indexes = HeaderIndexes::default();

    check_formula_injection(headers, 1, None)?;

    for (idx, header) in headers.iter().enumerate() {
        let normalized = header.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "address" | "recipient" | "to" => {
                if indexes.address.is_none() {
                    indexes.address = Some(idx);
                }
            }
            "amount" | "value" | "zec" => {
                if indexes.amount_zec.is_none() {
                    indexes.amount_zec = Some(idx);
                }
            }
            "amount_zatoshis" | "zatoshis" | "zats" => {
                if indexes.amount_zatoshis.is_none() {
                    indexes.amount_zatoshis = Some(idx);
                }
            }
            "memo" | "message" | "note" => {
                if indexes.memo.is_none() {
                    indexes.memo = Some(idx);
                }
            }
            "label" | "name" | "recipient_name" => {
                if indexes.label.is_none() {
                    indexes.label = Some(idx);
                }
            }
            _ => {}
        }
    }

    if indexes.address.is_none() {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1012,
            "missing required CSV column: address|recipient|to",
        ));
    }
    if indexes.amount_zatoshis.is_none() && indexes.amount_zec.is_none() {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1012,
            "missing required CSV amount column: amount_zatoshis|zatoshis|zats or amount|value|zec",
        ));
    }

    Ok(indexes)
}

fn check_formula_injection(
    record: &StringRecord,
    row_number: usize,
    headers: Option<&StringRecord>,
) -> Result<(), LaminarError> {
    for (idx, cell) in record.iter().enumerate() {
        if has_formula_prefix(cell) {
            let column = headers
                .and_then(|h| h.get(idx))
                .map(|name| name.to_string())
                .unwrap_or_else(|| format!("column#{}", idx + 1));
            return Err(LaminarError::taxonomy(
                TaxonomyCode::Validation1009,
                format!(
                    "formula injection detected at row {}, column {}",
                    row_number, column
                ),
            ));
        }
    }

    Ok(())
}

fn has_formula_prefix(value: &str) -> bool {
    matches!(
        value.chars().next(),
        Some('=') | Some('+') | Some('-') | Some('@') | Some('\t') | Some('\r')
    )
}

fn required_cell(record: &StringRecord, idx: Option<usize>) -> Option<String> {
    idx.and_then(|i| record.get(i))
        .map(|value| value.trim().to_string())
}

fn optional_cell(record: &StringRecord, idx: Option<usize>) -> Option<String> {
    idx.and_then(|i| record.get(i))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn strip_utf8_bom(input: &[u8]) -> &[u8] {
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &input[3..]
    } else {
        input
    }
}

#[cfg(test)]
mod tests {
    use crate::error::LaminarError;
    use crate::types::{BatchConfig, Network};

    use super::{parse_csv, MAX_FILE_SIZE_BYTES};

    fn config() -> BatchConfig {
        BatchConfig {
            network: Network::Mainnet,
            max_recipients: 500,
            source_file: "test.csv".to_string(),
        }
    }

    fn taxonomy_code(err: &LaminarError) -> Option<u16> {
        match err {
            LaminarError::Taxonomy(t) => Some(t.code()),
            LaminarError::BatchValidation(batch) => {
                batch.issues.first().map(|issue| issue.code.code())
            }
            _ => None,
        }
    }

    #[test]
    fn parses_valid_csv() {
        let csv = "address,amount\n\
                   t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,1.5\n";
        let rows = parse_csv(csv.as_bytes(), &config()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].row_number, 1);
        assert_eq!(rows[0].amount_zec.as_deref(), Some("1.5"));
    }

    #[test]
    fn rejects_missing_required_amount_column_with_1012() {
        let csv = "address,memo\n\
                   t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,hello\n";
        let err = parse_csv(csv.as_bytes(), &config()).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1012));
    }

    #[test]
    fn rejects_formula_injection_with_1009() {
        let csv = "address,amount\n\
                   t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,=1+1\n";
        let err = parse_csv(csv.as_bytes(), &config()).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1009));
        assert!(err.to_string().contains("row 2"));
    }

    #[test]
    fn parses_csv_with_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(
            b"address,amount\n\
              t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,2\n",
        );
        let rows = parse_csv(&bytes, &config()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount_zec.as_deref(), Some("2"));
    }

    #[test]
    fn parses_crlf_csv() {
        let csv = "address,amount\r\n\
                   t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,1\r\n";
        let rows = parse_csv(csv.as_bytes(), &config()).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn parses_quoted_fields() {
        let csv = "address,amount,memo,label\n\
                   t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,1,\"hello, world\",\"Alice\"\n";
        let rows = parse_csv(csv.as_bytes(), &config()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].memo.as_deref(), Some("hello, world"));
        assert_eq!(rows[0].label.as_deref(), Some("Alice"));
    }

    #[test]
    fn rejects_file_over_10mb() {
        let oversized = vec![b'a'; MAX_FILE_SIZE_BYTES + 1];
        let err = parse_csv(&oversized, &config()).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1010));
    }

    #[test]
    fn rejects_csv_over_1000_rows() {
        let mut csv = String::from("address,amount\n");
        for _ in 0..1001 {
            csv.push_str("t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,1\n");
        }
        let err = parse_csv(csv.as_bytes(), &config()).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1011));
    }
}
