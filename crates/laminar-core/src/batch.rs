use crate::error::{LaminarError, Result, TaxonomyCode};
use crate::types::{Network, Recipient, Zatoshi};
use crate::validation::{ValidatedBatch, ValidatedRecipient};
use crate::zip321::{build_zip321_uri, construct_zip321};

pub const PAYLOAD_LIMIT_QR_STATIC: usize = 2510;
pub const PAYLOAD_LIMIT_QR_ANIMATED: usize = 29_000;
pub const PAYLOAD_LIMIT_DEEPLINK: usize = 7200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedBatch {
    pub segments: Vec<crate::types::TransactionIntent>,
    pub max_payload_bytes: usize,
    pub original_recipient_count: usize,
}

pub fn segment_batch(batch: &ValidatedBatch, max_payload_bytes: usize) -> Result<SegmentedBatch> {
    if max_payload_bytes == 0 {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1005,
            "max_payload_bytes must be greater than zero",
        ));
    }
    if batch.recipients.is_empty() {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1012,
            "cannot segment an empty validated batch",
        ));
    }

    let mut segments = Vec::new();
    let mut current = Vec::new();

    for recipient in &batch.recipients {
        current.push(recipient.clone());
        let uri = build_zip321_uri(&current);

        if uri.len() <= max_payload_bytes {
            continue;
        }

        // Current recipient made the segment exceed the payload cap.
        let overflow_recipient = current.pop().expect("just pushed one recipient");
        if current.is_empty() {
            return Err(LaminarError::taxonomy(
                TaxonomyCode::Validation1010,
                format!(
                    "single recipient at row {} exceeds payload limit of {} bytes",
                    overflow_recipient.row_number, max_payload_bytes
                ),
            ));
        }

        segments.push(construct_segment(&current, batch.network)?);
        current.clear();
        current.push(overflow_recipient);

        let overflow_uri = build_zip321_uri(&current);
        if overflow_uri.len() > max_payload_bytes {
            return Err(LaminarError::taxonomy(
                TaxonomyCode::Validation1010,
                format!(
                    "single recipient at row {} exceeds payload limit of {} bytes",
                    current[0].row_number, max_payload_bytes
                ),
            ));
        }
    }

    if !current.is_empty() {
        segments.push(construct_segment(&current, batch.network)?);
    }

    Ok(SegmentedBatch {
        segments,
        max_payload_bytes,
        original_recipient_count: batch.recipients.len(),
    })
}

pub fn segment_recipients(
    recipients: &[Recipient],
    max_per_batch: usize,
) -> Result<Vec<Vec<Recipient>>> {
    if max_per_batch == 0 {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1005,
            "max_per_batch must be greater than zero",
        ));
    }

    Ok(recipients
        .chunks(max_per_batch)
        .map(|chunk| chunk.to_vec())
        .collect())
}

fn construct_segment(
    recipients: &[ValidatedRecipient],
    network: Network,
) -> Result<crate::types::TransactionIntent> {
    let total = sum_total(recipients).map_err(|_| {
        LaminarError::taxonomy(
            TaxonomyCode::Validation1013,
            "overflow while summing segment recipients",
        )
    })?;
    let validated = ValidatedBatch {
        recipients: recipients.to_vec(),
        total,
        network,
        warnings: Vec::new(),
    };
    construct_zip321(&validated)
}

fn sum_total(recipients: &[ValidatedRecipient]) -> std::result::Result<Zatoshi, ()> {
    let mut total: Option<Zatoshi> = None;
    for validated in recipients {
        total = Some(match total {
            Some(current) => current
                .checked_add(validated.recipient.amount)
                .map_err(|_| ())?,
            None => validated.recipient.amount,
        });
    }
    total.ok_or(())
}

#[cfg(test)]
mod tests {
    use crate::error::LaminarError;
    use crate::types::Network;
    use crate::types::{Recipient, Zatoshi};
    use crate::validation::{RecipientAddressType, ValidatedBatch, ValidatedRecipient};

    use super::{segment_batch, segment_recipients};

    const MAINNET_TADDR: &str = "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs";

    fn taxonomy_code(err: &LaminarError) -> Option<u16> {
        match err {
            LaminarError::Taxonomy(t) => Some(t.code()),
            LaminarError::BatchValidation(batch) => {
                batch.issues.first().map(|issue| issue.code.code())
            }
            _ => None,
        }
    }

    fn recipient(index: usize) -> ValidatedRecipient {
        ValidatedRecipient {
            row_number: index + 1,
            address_type: RecipientAddressType::Transparent,
            recipient: Recipient {
                address: MAINNET_TADDR.to_string(),
                amount: Zatoshi::new(1).unwrap(),
                memo: Some(format!("memo-{index}")),
                label: None,
            },
        }
    }

    fn batch_with_count(count: usize) -> ValidatedBatch {
        let recipients: Vec<_> = (0..count).map(recipient).collect();
        let mut total = recipients[0].recipient.amount;
        for validated in recipients.iter().skip(1) {
            total = total.checked_add(validated.recipient.amount).unwrap();
        }

        ValidatedBatch {
            recipients,
            total,
            network: Network::Mainnet,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn segmentation_splits_large_batches() {
        let batch = batch_with_count(100);
        let segmented = segment_batch(&batch, 350).unwrap();
        assert!(segmented.segments.len() > 1);

        let recipient_count: usize = segmented
            .segments
            .iter()
            .map(|segment| segment.recipients.len())
            .sum();
        assert_eq!(recipient_count, 100);
        assert!(segmented
            .segments
            .iter()
            .all(|segment| segment.payload_bytes <= 350));
    }

    #[test]
    fn segmentation_rejects_zero_payload_limit() {
        let batch = batch_with_count(1);
        let err = segment_batch(&batch, 0).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1005));
    }

    #[test]
    fn segmentation_rejects_empty_batch() {
        let empty_batch = ValidatedBatch {
            recipients: Vec::new(),
            total: Zatoshi::new(1).unwrap(),
            network: Network::Mainnet,
            warnings: Vec::new(),
        };
        let err = segment_batch(&empty_batch, 256).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1012));
    }

    #[test]
    fn segmentation_rejects_single_recipient_that_exceeds_limit() {
        let row = ValidatedRecipient {
            row_number: 1,
            address_type: RecipientAddressType::Transparent,
            recipient: Recipient {
                address: MAINNET_TADDR.to_string(),
                amount: Zatoshi::new(1).unwrap(),
                memo: Some("x".repeat(400)),
                label: None,
            },
        };
        let batch = ValidatedBatch {
            recipients: vec![row],
            total: Zatoshi::new(1).unwrap(),
            network: Network::Mainnet,
            warnings: Vec::new(),
        };
        let err = segment_batch(&batch, 80).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1010));
    }

    #[test]
    fn segmentation_rejects_when_rollover_recipient_alone_exceeds_limit() {
        let first = ValidatedRecipient {
            row_number: 1,
            address_type: RecipientAddressType::Transparent,
            recipient: Recipient {
                address: MAINNET_TADDR.to_string(),
                amount: Zatoshi::new(1).unwrap(),
                memo: None,
                label: None,
            },
        };
        let second = ValidatedRecipient {
            row_number: 2,
            address_type: RecipientAddressType::Transparent,
            recipient: Recipient {
                address: MAINNET_TADDR.to_string(),
                amount: Zatoshi::new(1).unwrap(),
                memo: Some("y".repeat(300)),
                label: None,
            },
        };
        let batch = ValidatedBatch {
            recipients: vec![first, second],
            total: Zatoshi::new(2).unwrap(),
            network: Network::Mainnet,
            warnings: Vec::new(),
        };
        let err = segment_batch(&batch, 120).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1010));
    }

    #[test]
    fn segmentation_rejects_construct_segment_overflow() {
        let recipients = vec![
            ValidatedRecipient {
                row_number: 1,
                address_type: RecipientAddressType::Transparent,
                recipient: Recipient {
                    address: MAINNET_TADDR.to_string(),
                    amount: Zatoshi::new(crate::types::ZATOSHI_MAX).unwrap(),
                    memo: None,
                    label: None,
                },
            },
            ValidatedRecipient {
                row_number: 2,
                address_type: RecipientAddressType::Transparent,
                recipient: Recipient {
                    address: MAINNET_TADDR.to_string(),
                    amount: Zatoshi::new(1).unwrap(),
                    memo: None,
                    label: None,
                },
            },
        ];
        let batch = ValidatedBatch {
            recipients,
            total: Zatoshi::new(crate::types::ZATOSHI_MAX).unwrap(),
            network: Network::Mainnet,
            warnings: Vec::new(),
        };
        let err = segment_batch(&batch, 10_000).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1013));
    }

    #[test]
    fn segment_recipients_chunks_and_rejects_zero() {
        let recipients = vec![
            Recipient {
                address: MAINNET_TADDR.to_string(),
                amount: Zatoshi::new(1).unwrap(),
                memo: None,
                label: None,
            },
            Recipient {
                address: MAINNET_TADDR.to_string(),
                amount: Zatoshi::new(2).unwrap(),
                memo: None,
                label: None,
            },
            Recipient {
                address: MAINNET_TADDR.to_string(),
                amount: Zatoshi::new(3).unwrap(),
                memo: None,
                label: None,
            },
        ];

        let chunks = segment_recipients(&recipients, 2).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[1].len(), 1);

        let err = segment_recipients(&recipients, 0).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(1005));
    }
}
