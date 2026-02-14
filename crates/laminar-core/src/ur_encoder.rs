use crate::error::{LaminarError, Result, TaxonomyCode};

pub const DEFAULT_UR_FRAGMENT_LEN: usize = 150;

pub fn encode_ur_fragments(payload: &[u8], max_fragment_len: usize) -> Result<Vec<String>> {
    if payload.is_empty() {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Handoff5005,
            "UR payload must not be empty",
        ));
    }
    if max_fragment_len == 0 {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Handoff5005,
            "UR max fragment length must be greater than zero",
        ));
    }

    let mut encoder = ur::Encoder::bytes(payload, max_fragment_len).map_err(|err| {
        LaminarError::taxonomy(
            TaxonomyCode::Handoff5005,
            format!("failed to create UR encoder: {err}"),
        )
    })?;

    let total = encoder.fragment_count();
    let mut fragments = Vec::with_capacity(total);
    for _ in 0..total {
        fragments.push(encoder.next_part().map_err(|err| {
            LaminarError::taxonomy(
                TaxonomyCode::Handoff5006,
                format!("failed to emit UR fragment: {err}"),
            )
        })?);
    }

    Ok(fragments)
}

pub fn decode_ur_fragments(fragments: &[String]) -> Result<Vec<u8>> {
    if fragments.is_empty() {
        return Err(LaminarError::taxonomy(
            TaxonomyCode::Handoff5007,
            "UR fragment list must not be empty",
        ));
    }

    let mut decoder = ur::Decoder::default();
    for fragment in fragments {
        decoder.receive(fragment).map_err(|err| {
            LaminarError::taxonomy(
                TaxonomyCode::Handoff5007,
                format!("failed to decode UR fragment: {err}"),
            )
        })?;
    }

    decoder
        .message()
        .map_err(|err| {
            LaminarError::taxonomy(
                TaxonomyCode::Handoff5008,
                format!("failed to finalize UR decode: {err}"),
            )
        })?
        .ok_or_else(|| {
            LaminarError::taxonomy(
                TaxonomyCode::Handoff5008,
                "UR decoder is incomplete after all provided fragments",
            )
        })
}

#[cfg(test)]
mod tests {
    use crate::error::LaminarError;

    use super::{decode_ur_fragments, encode_ur_fragments, DEFAULT_UR_FRAGMENT_LEN};

    fn taxonomy_code(err: &LaminarError) -> Option<u16> {
        match err {
            LaminarError::Taxonomy(t) => Some(t.code()),
            _ => None,
        }
    }

    fn sample_payload() -> Vec<u8> {
        "laminar-ur-roundtrip".repeat(128).into_bytes()
    }

    #[test]
    fn encode_uses_multipart_ur_format() {
        let payload = sample_payload();
        let fragments = encode_ur_fragments(&payload, DEFAULT_UR_FRAGMENT_LEN).unwrap();
        assert!(fragments.len() > 1);

        for (expected_index, fragment) in fragments.iter().enumerate() {
            assert!(fragment.starts_with("ur:bytes/"));
            let remainder = fragment.strip_prefix("ur:bytes/").unwrap();
            let (indices, body) = remainder.split_once('/').unwrap();
            let (part, total) = indices.split_once('-').unwrap();
            let part = part.parse::<usize>().unwrap();
            let total = total.parse::<usize>().unwrap();

            assert_eq!(part, expected_index + 1);
            assert_eq!(total, fragments.len());
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn ur_roundtrip_encode_decode() {
        let payload = sample_payload();
        let fragments = encode_ur_fragments(&payload, DEFAULT_UR_FRAGMENT_LEN).unwrap();
        let decoded = decode_ur_fragments(&fragments).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn encode_rejects_empty_payload() {
        let err = encode_ur_fragments(&[], DEFAULT_UR_FRAGMENT_LEN).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(5005));
    }

    #[test]
    fn encode_rejects_zero_fragment_len() {
        let err = encode_ur_fragments(&sample_payload(), 0).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(5005));
    }

    #[test]
    fn decode_rejects_empty_fragment_list() {
        let err = decode_ur_fragments(&[]).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(5007));
    }

    #[test]
    fn decode_rejects_invalid_fragment_data() {
        let fragments = vec!["ur:bytes/not-valid".to_string()];
        let err = decode_ur_fragments(&fragments).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(5007));
    }

    #[test]
    fn decode_rejects_incomplete_fragment_set() {
        let payload = sample_payload();
        let fragments = encode_ur_fragments(&payload, DEFAULT_UR_FRAGMENT_LEN).unwrap();
        let incomplete = vec![fragments[0].clone()];
        let err = decode_ur_fragments(&incomplete).unwrap_err();
        assert_eq!(taxonomy_code(&err), Some(5008));
    }
}
