use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ImageEncoder, Luma};
use qrcode::{EcLevel, QrCode};

use crate::batch::PAYLOAD_LIMIT_QR_STATIC;
use crate::error::{LaminarError, Result, TaxonomyCode};
use crate::types::TransactionIntent;
use crate::ur_encoder::{encode_ur_fragments, DEFAULT_UR_FRAGMENT_LEN};

pub const MIN_QR_DIMENSION: u32 = 300;
pub const UR_FRAME_INTERVAL_MS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrMode {
    Static,
    AnimatedUr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrFrame {
    pub index: usize,
    pub png_bytes: Vec<u8>,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrOutput {
    pub mode: QrMode,
    pub frames: Vec<QrFrame>,
    pub total_frames: usize,
    pub payload_bytes: usize,
}

pub fn generate_qr(intent: &TransactionIntent) -> Result<QrOutput> {
    let payload = intent.zip321_uri.as_bytes();
    let payload_bytes = payload.len();

    if payload_bytes <= PAYLOAD_LIMIT_QR_STATIC {
        let data = intent.zip321_uri.clone();
        let png_bytes = generate_qr_png(&data, MIN_QR_DIMENSION)?;
        let frame = QrFrame {
            index: 0,
            png_bytes,
            data,
        };
        return Ok(QrOutput {
            mode: QrMode::Static,
            frames: vec![frame],
            total_frames: 1,
            payload_bytes,
        });
    }

    let fragments = encode_ur_fragments(payload, DEFAULT_UR_FRAGMENT_LEN)?;
    let mut frames = Vec::with_capacity(fragments.len());
    for (index, data) in fragments.into_iter().enumerate() {
        let png_bytes = generate_qr_png(&data, MIN_QR_DIMENSION)?;
        frames.push(QrFrame {
            index,
            png_bytes,
            data,
        });
    }

    let total_frames = frames.len();
    Ok(QrOutput {
        mode: QrMode::AnimatedUr,
        frames,
        total_frames,
        payload_bytes,
    })
}

pub fn generate_qr_png(data: &str, size: u32) -> Result<Vec<u8>> {
    let code = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M).map_err(|err| {
        LaminarError::taxonomy(
            TaxonomyCode::Handoff5003,
            format!("failed to encode QR data: {err}"),
        )
    })?;

    let dimension = size.max(MIN_QR_DIMENSION);
    let image = code
        .render::<Luma<u8>>()
        .min_dimensions(dimension, dimension)
        .dark_color(Luma([0u8]))
        .light_color(Luma([255u8]))
        .quiet_zone(true)
        .build();

    let (width, height) = image.dimensions();
    let raw = image.into_raw();
    let mut png_bytes = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut png_bytes, CompressionType::Best, FilterType::Adaptive);
    encoder
        .write_image(&raw, width, height, image::ColorType::L8.into())
        .map_err(|err| {
            LaminarError::taxonomy(
                TaxonomyCode::Handoff5004,
                format!("failed to encode PNG bytes: {err}"),
            )
        })?;

    Ok(png_bytes)
}

#[cfg(test)]
mod tests {
    use crate::batch::PAYLOAD_LIMIT_QR_STATIC;
    use crate::types::{Network, Recipient, TransactionIntent, Zatoshi};
    use crate::validation::{RecipientAddressType, ValidatedBatch, ValidatedRecipient};
    use crate::zip321::construct_zip321;

    use super::{generate_qr, QrMode, MIN_QR_DIMENSION};

    const MAINNET_TADDR: &str = "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs";
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    fn intent_with_recipients(count: usize) -> TransactionIntent {
        let recipients: Vec<ValidatedRecipient> = (0..count)
            .map(|index| ValidatedRecipient {
                row_number: index + 1,
                address_type: RecipientAddressType::Transparent,
                recipient: Recipient {
                    address: MAINNET_TADDR.to_string(),
                    amount: Zatoshi::new(1).unwrap(),
                    memo: Some(format!("memo-{index:03}")),
                    label: None,
                },
            })
            .collect();

        let mut total = recipients[0].recipient.amount;
        for validated in recipients.iter().skip(1) {
            total = total.checked_add(validated.recipient.amount).unwrap();
        }

        let batch = ValidatedBatch {
            recipients,
            total,
            network: Network::Mainnet,
            warnings: Vec::new(),
        };
        construct_zip321(&batch).unwrap()
    }

    fn decode_qr_data(png_bytes: &[u8]) -> String {
        let gray = image::load_from_memory(png_bytes).unwrap().to_luma8();
        let mut prepared = rqrr::PreparedImage::prepare(gray);
        let mut grids = prepared.detect_grids();
        assert!(!grids.is_empty());
        let (_, content) = grids.remove(0).decode().unwrap();
        content
    }

    #[test]
    fn static_qr_for_ten_recipients_is_single_frame_valid_png() {
        let intent = intent_with_recipients(10);
        assert!(intent.payload_bytes <= PAYLOAD_LIMIT_QR_STATIC);

        let output = generate_qr(&intent).unwrap();
        assert_eq!(output.mode, QrMode::Static);
        assert_eq!(output.total_frames, 1);
        assert_eq!(output.frames.len(), 1);
        assert_eq!(output.payload_bytes, intent.payload_bytes);

        let frame = &output.frames[0];
        assert_eq!(frame.index, 0);
        assert_eq!(frame.data, intent.zip321_uri);
        assert!(frame.png_bytes.starts_with(&PNG_SIGNATURE));

        let decoded = decode_qr_data(&frame.png_bytes);
        assert_eq!(decoded, intent.zip321_uri);
    }

    #[test]
    fn animated_ur_for_large_payload_uses_multiple_frames_and_roundtrips() {
        let intent = intent_with_recipients(100);
        assert!(intent.payload_bytes > PAYLOAD_LIMIT_QR_STATIC);

        let output = generate_qr(&intent).unwrap();
        assert_eq!(output.mode, QrMode::AnimatedUr);
        assert!(output.total_frames > 1);
        assert_eq!(output.total_frames, output.frames.len());
        assert_eq!(output.payload_bytes, intent.payload_bytes);

        let mut decoder = ur::Decoder::default();
        for (index, frame) in output.frames.iter().enumerate() {
            assert_eq!(frame.index, index);
            assert!(frame.data.starts_with("ur:bytes/"));
            assert!(frame.png_bytes.starts_with(&PNG_SIGNATURE));

            let decoded_qr = decode_qr_data(&frame.png_bytes);
            assert_eq!(decoded_qr, frame.data);

            decoder.receive(&frame.data).unwrap();
        }
        assert!(decoder.complete());
        assert_eq!(
            decoder.message().unwrap(),
            Some(intent.zip321_uri.as_bytes().to_vec())
        );
    }

    #[test]
    fn qr_png_is_at_least_300x300() {
        let intent = intent_with_recipients(10);
        let output = generate_qr(&intent).unwrap();
        let image = image::load_from_memory(&output.frames[0].png_bytes).unwrap();
        assert!(image.width() >= MIN_QR_DIMENSION);
        assert!(image.height() >= MIN_QR_DIMENSION);
    }

    #[test]
    fn qr_generation_is_deterministic_for_same_input() {
        let intent = intent_with_recipients(10);
        let first = generate_qr(&intent).unwrap();
        let second = generate_qr(&intent).unwrap();
        assert_eq!(first, second);
    }
}
