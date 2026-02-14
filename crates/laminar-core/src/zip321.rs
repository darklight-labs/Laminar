use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{LaminarError, Result, TaxonomyCode};
use crate::types::{TransactionIntent, Zatoshi};
use crate::validation::{ValidatedBatch, ValidatedRecipient};

const ZIP321_SCHEMA_VERSION: &str = "1.0";
const ZIP321_UUID_NAMESPACE: Uuid = Uuid::from_u128(0x4f68b80f_0ec5_52f4_8357_0b6b89df8754);

pub fn base64_encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

pub fn build_zip321_uri(recipients: &[ValidatedRecipient]) -> String {
    if recipients.is_empty() {
        return "zcash:".to_string();
    }

    if recipients.len() == 1 {
        let mut uri = String::from("zcash:");
        uri.push_str(&recipients[0].recipient.address);

        let mut params = Vec::new();
        params.push(format!(
            "amount={}",
            recipients[0].recipient.amount.to_zec_string()
        ));
        if let Some(memo) = recipients[0]
            .recipient
            .memo
            .as_deref()
            .filter(|memo| !memo.is_empty())
        {
            params.push(format!("memo={}", base64_encode(memo.as_bytes())));
        }

        if !params.is_empty() {
            uri.push('?');
            uri.push_str(&params.join("&"));
        }
        return uri;
    }

    let mut params = Vec::new();
    for (index, validated) in recipients.iter().enumerate() {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!(".{index}")
        };
        params.push(format!("address{suffix}={}", validated.recipient.address));
        params.push(format!(
            "amount{suffix}={}",
            validated.recipient.amount.to_zec_string()
        ));
        if let Some(memo) = validated
            .recipient
            .memo
            .as_deref()
            .filter(|memo| !memo.is_empty())
        {
            params.push(format!("memo{suffix}={}", base64_encode(memo.as_bytes())));
        }
    }

    format!("zcash:?{}", params.join("&"))
}

pub fn construct_zip321(batch: &ValidatedBatch) -> Result<TransactionIntent> {
    if batch.recipients.is_empty() {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Validation1012,
            "cannot construct ZIP-321 URI for empty recipient set",
        ));
    }

    let uri = build_zip321_uri(&batch.recipients);
    let payload_bytes = uri.len();
    let digest = Sha256::digest(uri.as_bytes());
    let payload_hash = format!("sha256:{}", to_lower_hex(&digest));

    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    let id = Uuid::new_v5(&ZIP321_UUID_NAMESPACE, &digest_bytes);

    let created_at = deterministic_created_at(&digest_bytes);
    let total_zat = sum_total(&batch.recipients).map_err(|_| {
        LaminarError::taxonomy(
            TaxonomyCode::Validation1013,
            "overflow while summing recipients for ZIP-321 construction",
        )
    })?;

    Ok(TransactionIntent {
        schema_version: ZIP321_SCHEMA_VERSION.to_string(),
        id,
        created_at,
        network: batch.network,
        recipients: batch
            .recipients
            .iter()
            .map(|validated| validated.recipient.clone())
            .collect(),
        total_zat,
        zip321_uri: uri,
        payload_bytes,
        payload_hash,
    })
}

pub fn build_payment_request(intent: &TransactionIntent) -> Result<String> {
    Ok(intent.zip321_uri.clone())
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

fn deterministic_created_at(digest: &[u8; 32]) -> DateTime<Utc> {
    let mut seconds_bytes = [0u8; 8];
    seconds_bytes.copy_from_slice(&digest[..8]);
    let raw = u64::from_be_bytes(seconds_bytes);

    // 9999-12-31T23:59:59Z in Unix seconds.
    let upper_bound = 253_402_300_799_u64;
    let bounded = (raw % upper_bound) as i64;

    DateTime::<Utc>::from_timestamp(bounded, 0).expect("bounded timestamp must be valid")
}

fn to_lower_hex(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 2);
    for byte in data {
        out.push(hex_char((byte >> 4) & 0x0f));
        out.push(hex_char(byte & 0x0f));
    }
    out
}

fn hex_char(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => unreachable!("nibble out of range"),
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{Network, Recipient, Zatoshi};
    use crate::validation::{RecipientAddressType, ValidatedBatch, ValidatedRecipient};

    use super::{base64_encode, build_zip321_uri, construct_zip321};

    const ADDR1: &str = "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs";
    const ADDR2: &str =
        "zs1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq6d8g";

    fn vr(
        row_number: usize,
        address: &str,
        amount_zat: u64,
        memo: Option<&str>,
    ) -> ValidatedRecipient {
        ValidatedRecipient {
            row_number,
            address_type: RecipientAddressType::Transparent,
            recipient: Recipient {
                address: address.to_string(),
                amount: Zatoshi::new(amount_zat).unwrap(),
                memo: memo.map(ToOwned::to_owned),
                label: None,
            },
        }
    }

    fn batch(recipients: Vec<ValidatedRecipient>) -> ValidatedBatch {
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
    fn single_recipient_uri_exact() {
        let recipients = vec![vr(1, ADDR1, 150_000_000, Some("hello"))];
        let uri = build_zip321_uri(&recipients);
        assert_eq!(
            uri,
            "zcash:t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs?amount=1.5&memo=aGVsbG8="
        );
    }

    #[test]
    fn multi_recipient_uri_param_index_format() {
        let recipients = vec![
            vr(1, ADDR1, 100_000_000, Some("a")),
            vr(2, ADDR2, 1, Some("b")),
        ];
        let uri = build_zip321_uri(&recipients);
        assert_eq!(
            uri,
            "zcash:?address=t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs&amount=1&memo=YQ==&address.1=zs1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq6d8g&amount.1=0.00000001&memo.1=Yg=="
        );
    }

    #[test]
    fn memo_base64_uses_standard_alphabet() {
        let encoded = base64_encode(&[0xfb, 0xff, 0xff]);
        assert_eq!(encoded, "+///");
    }

    #[test]
    fn memo_base64_keeps_padding() {
        let encoded = base64_encode(b"a");
        assert_eq!(encoded, "YQ==");
    }

    #[test]
    fn construct_is_deterministic_across_100_runs() {
        let batch = batch(vec![
            vr(1, ADDR1, 100_000_000, Some("memo-one")),
            vr(2, ADDR2, 250_000_000, Some("memo-two")),
        ]);

        let first = construct_zip321(&batch).unwrap();
        for _ in 0..100 {
            let next = construct_zip321(&batch).unwrap();
            assert_eq!(next.zip321_uri, first.zip321_uri);
            assert_eq!(next.payload_bytes, first.payload_bytes);
            assert_eq!(next.payload_hash, first.payload_hash);
            assert_eq!(next.id, first.id);
            assert_eq!(next.created_at, first.created_at);
            assert_eq!(next.total_zat, first.total_zat);
        }
    }

    #[test]
    fn edge_max_memo_and_empty_memo_behavior() {
        let max_memo = "x".repeat(512);
        let recipients = vec![vr(1, ADDR1, 1, Some(&max_memo)), vr(2, ADDR2, 1, Some(""))];
        let uri = build_zip321_uri(&recipients);
        assert!(uri.contains("memo="));
        assert!(!uri.contains("memo.1="));
    }
}
