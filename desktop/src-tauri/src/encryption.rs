use std::sync::{Mutex, OnceLock};

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use zeroize::Zeroize;

const SALT: &[u8] = b"laminar-desktop-v1";
const NONCE_LEN: usize = 12;
#[cfg(not(test))]
const ARGON2_ITERATIONS: u32 = 100_000;
#[cfg(test)]
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_MEMORY_KIB: u32 = 8;

fn key_store() -> &'static Mutex<Option<[u8; 32]>> {
    static STORE: OnceLock<Mutex<Option<[u8; 32]>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn test_key_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn derive_key_from_passphrase(passphrase: &str) -> Result<[u8; 32], String> {
    let params = Params::new(ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, 1, Some(32))
        .map_err(|err| format!("failed to configure Argon2id params: {err}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), SALT, &mut key)
        .map_err(|err| format!("failed to derive key with Argon2id: {err}"))?;
    Ok(key)
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn from_hex(input: &str) -> Result<Vec<u8>, String> {
    if input.len() % 2 != 0 {
        return Err("hex data must have an even length".to_string());
    }

    let mut out = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();

    for i in (0..bytes.len()).step_by(2) {
        let high = hex_nibble(bytes[i]).ok_or_else(|| "invalid hex character".to_string())?;
        let low = hex_nibble(bytes[i + 1]).ok_or_else(|| "invalid hex character".to_string())?;
        out.push((high << 4) | low);
    }

    Ok(out)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn with_session_key<T>(f: impl FnOnce(&[u8; 32]) -> Result<T, String>) -> Result<T, String> {
    let guard = key_store()
        .lock()
        .map_err(|_| "failed to acquire encryption key lock".to_string())?;
    let Some(key) = guard.as_ref() else {
        return Err("storage passphrase is not set".to_string());
    };
    f(key)
}

pub fn set_passphrase(mut passphrase: String) -> Result<(), String> {
    let mut derived = derive_key_from_passphrase(&passphrase)?;
    passphrase.zeroize();

    let mut guard = key_store()
        .lock()
        .map_err(|_| "failed to acquire encryption key lock".to_string())?;
    if let Some(existing) = guard.as_mut() {
        existing.zeroize();
    }
    *guard = Some(derived);
    derived.zeroize();
    Ok(())
}

pub fn is_unlocked() -> bool {
    match key_store().lock() {
        Ok(guard) => guard.is_some(),
        Err(_) => false,
    }
}

pub fn clear_session_key() {
    if let Ok(mut guard) = key_store().lock() {
        if let Some(existing) = guard.as_mut() {
            existing.zeroize();
        }
        *guard = None;
    }
}

pub fn encrypt_string(plaintext: &str) -> Result<String, String> {
    with_session_key(|key| {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|err| format!("failed to initialize cipher: {err}"))?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut plaintext_owned = plaintext.to_string();
        let mut plaintext_bytes = plaintext_owned.as_bytes().to_vec();
        let ciphertext = cipher
            .encrypt(nonce, plaintext_bytes.as_ref())
            .map_err(|_| "encryption failed".to_string())?;

        plaintext_bytes.zeroize();
        plaintext_owned.zeroize();

        let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        let encoded = to_hex(&combined);
        combined.zeroize();

        Ok(encoded)
    })
}

pub fn decrypt_string(data: &str) -> Result<String, String> {
    with_session_key(|key| {
        let mut combined = from_hex(data)?;
        if combined.len() <= NONCE_LEN {
            combined.zeroize();
            return Err("ciphertext is too short".to_string());
        }

        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|err| format!("failed to initialize cipher: {err}"))?;

        let mut plaintext_bytes = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|_| "decryption failed".to_string())?;
        combined.zeroize();

        let plaintext = String::from_utf8(plaintext_bytes.clone())
            .map_err(|err| format!("utf-8 decode failed: {err}"))?;
        plaintext_bytes.zeroize();
        Ok(plaintext)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        clear_session_key, decrypt_string, encrypt_string, from_hex, set_passphrase, test_key_lock,
    };

    #[test]
    fn encrypt_decrypt_roundtrip_with_nonce_prefix() {
        let _guard = test_key_lock().lock().unwrap();
        clear_session_key();
        set_passphrase("test-passphrase".to_string()).unwrap();

        let encrypted = encrypt_string("sensitive-value").unwrap();
        let decoded = from_hex(&encrypted).unwrap();
        assert!(decoded.len() > 12);

        let decrypted = decrypt_string(&encrypted).unwrap();
        assert_eq!(decrypted, "sensitive-value");
        clear_session_key();
    }

    #[test]
    fn decrypt_fails_when_key_not_set() {
        let _guard = test_key_lock().lock().unwrap();
        clear_session_key();
        let result = decrypt_string("00");
        assert!(result.is_err());
    }
}
