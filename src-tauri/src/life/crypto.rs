use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use std::sync::Arc;

use crate::settings::KEYRING_SERVICE;

pub const KEYRING_LIFE_USER: &str = "life-encryption-key";
const ENCRYPTED_PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("encryption key is not available in keyring: {0}")]
    KeyUnavailable(String),
    #[error("keyring access error: {0}")]
    Keyring(String),
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed (invalid key or corrupted ciphertext)")]
    DecryptionFailed,
    #[error("invalid ciphertext format")]
    InvalidFormat,
    #[error("invalid utf-8 payload")]
    InvalidUtf8,
    #[error("random generation error: {0}")]
    Random(String),
}

#[derive(Clone)]
pub struct LifeCrypto {
    cipher: Arc<Aes256Gcm>,
    key_id: String,
}

impl LifeCrypto {
    /// Create a crypto handler from an explicit 32-byte key (used for tests or memory instances).
    pub fn from_key(key: [u8; 32], key_id: impl Into<String>) -> Self {
        let cipher = Aes256Gcm::new_from_slice(&key).expect("32-byte key is valid for AES-256");
        Self {
            cipher: Arc::new(cipher),
            key_id: key_id.into(),
        }
    }

    /// Generate an in-memory random instance (for tests and temporary stores).
    pub fn in_memory_random() -> Self {
        let mut key_bytes = [0u8; 32];
        getrandom::fill(&mut key_bytes).expect("OS random generation should succeed");
        Self::from_key(key_bytes, "in-memory-test-key")
    }

    /// Load or create the life encryption key in the OS keyring.
    pub fn from_keyring_or_create() -> Result<Self, CryptoError> {
        let entry = crate::credentials::Entry::new(KEYRING_SERVICE, KEYRING_LIFE_USER)
            .map_err(|e| CryptoError::Keyring(e.to_string()))?;

        match entry.get_password() {
            Ok(hex_str) => {
                let key_bytes = hex_to_key32(&hex_str)?;
                Ok(Self::from_key(key_bytes, "os-keyring-v1"))
            }
            Err(keyring::Error::NoEntry) => {
                // Generate fresh 256-bit key
                let mut key_bytes = [0u8; 32];
                getrandom::fill(&mut key_bytes).map_err(|e| CryptoError::Random(e.to_string()))?;
                let hex_str = key_to_hex(&key_bytes);
                entry
                    .set_password(&hex_str)
                    .map_err(|e| CryptoError::Keyring(e.to_string()))?;
                Ok(Self::from_key(key_bytes, "os-keyring-v1"))
            }
            Err(err) => Err(CryptoError::Keyring(err.to_string())),
        }
    }

    /// Attempt to load the key without creating a new one (detecting key loss).
    pub fn from_keyring_existing() -> Result<Self, CryptoError> {
        let entry = crate::credentials::Entry::new(KEYRING_SERVICE, KEYRING_LIFE_USER)
            .map_err(|e| CryptoError::Keyring(e.to_string()))?;

        match entry.get_password() {
            Ok(hex_str) => {
                let key_bytes = hex_to_key32(&hex_str)?;
                Ok(Self::from_key(key_bytes, "os-keyring-v1"))
            }
            Err(keyring::Error::NoEntry) => Err(CryptoError::KeyUnavailable(
                "No life encryption key found in OS keyring".to_string(),
            )),
            Err(err) => Err(CryptoError::Keyring(err.to_string())),
        }
    }

    /// Delete the key from the OS keyring (for fresh resets).
    pub fn delete_keyring_key() -> Result<(), CryptoError> {
        let entry = crate::credentials::Entry::new(KEYRING_SERVICE, KEYRING_LIFE_USER)
            .map_err(|e| CryptoError::Keyring(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(CryptoError::Keyring(err.to_string())),
        }
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Encrypt plaintext into an authenticated `enc:v1:<base64>` envelope.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce_bytes).map_err(|e| CryptoError::Random(e.to_string()))?;
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        payload.extend_from_slice(&nonce_bytes);
        payload.extend_from_slice(&ciphertext);

        let b64 = BASE64_STANDARD.encode(&payload);
        Ok(format!("{ENCRYPTED_PREFIX}{b64}"))
    }

    /// Decrypt an authenticated `enc:v1:<base64>` envelope, or return non-encrypted strings as is.
    pub fn decrypt(&self, input: &str) -> Result<String, CryptoError> {
        if !input.starts_with(ENCRYPTED_PREFIX) {
            return Ok(input.to_string());
        }

        let b64 = &input[ENCRYPTED_PREFIX.len()..];
        let payload = BASE64_STANDARD
            .decode(b64)
            .map_err(|_| CryptoError::InvalidFormat)?;

        if payload.len() < NONCE_LEN {
            return Err(CryptoError::InvalidFormat);
        }

        let (nonce_slice, ciphertext) = payload.split_at(NONCE_LEN);
        let mut nonce_bytes = [0u8; NONCE_LEN];
        nonce_bytes.copy_from_slice(nonce_slice);
        let nonce = Nonce::from(nonce_bytes);

        let decrypted_bytes = self
            .cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)?;

        String::from_utf8(decrypted_bytes).map_err(|_| CryptoError::InvalidUtf8)
    }

    /// Check if a string is in encrypted envelope format.
    pub fn is_encrypted(text: &str) -> bool {
        text.starts_with(ENCRYPTED_PREFIX)
    }
}

fn key_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn hex_to_key32(hex_str: &str) -> Result<[u8; 32], CryptoError> {
    if hex_str.len() != 64 || !hex_str.is_ascii() {
        return Err(CryptoError::InvalidFormat);
    }
    let mut key = [0u8; 32];
    for i in 0..32 {
        let byte_str = &hex_str[i * 2..i * 2 + 2];
        key[i] = u8::from_str_radix(byte_str, 16).map_err(|_| CryptoError::InvalidFormat)?;
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encryption_decryption() {
        let crypto = LifeCrypto::in_memory_random();
        let secret = "sensitive medical checkup: blood pressure 120/80";
        let encrypted = crypto.encrypt(secret).unwrap();
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert_ne!(encrypted, secret);
        assert!(!encrypted.contains("blood pressure"));

        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn wrong_key_fails_cleanly() {
        let crypto1 = LifeCrypto::in_memory_random();
        let crypto2 = LifeCrypto::in_memory_random();
        let secret = "classified financial transaction";
        let encrypted = crypto1.encrypt(secret).unwrap();

        let err = crypto2.decrypt(&encrypted).unwrap_err();
        assert_eq!(err, CryptoError::DecryptionFailed);
    }

    #[test]
    fn unencrypted_string_passes_through() {
        let crypto = LifeCrypto::in_memory_random();
        let plain = "just a general note";
        let decrypted = crypto.decrypt(plain).unwrap();
        assert_eq!(decrypted, plain);
    }
    #[test]
    fn keyring_lifecycle_uses_memory_in_tests() {
        LifeCrypto::delete_keyring_key().unwrap();
        assert!(matches!(
            LifeCrypto::from_keyring_existing(),
            Err(CryptoError::KeyUnavailable(_))
        ));
        let first = LifeCrypto::from_keyring_or_create().unwrap();
        let encrypted = first.encrypt("private").unwrap();
        let loaded = LifeCrypto::from_keyring_existing().unwrap();
        assert_eq!(loaded.decrypt(&encrypted).unwrap(), "private");
        LifeCrypto::delete_keyring_key().unwrap();
        let replacement = LifeCrypto::from_keyring_or_create().unwrap();
        assert_eq!(
            replacement.decrypt(&encrypted),
            Err(CryptoError::DecryptionFailed)
        );
        LifeCrypto::delete_keyring_key().unwrap();
    }

    #[test]
    fn malformed_unicode_key_is_an_error_not_a_panic() {
        assert_eq!(
            hex_to_key32(&"aéa".repeat(16)),
            Err(CryptoError::InvalidFormat)
        );
    }
}
