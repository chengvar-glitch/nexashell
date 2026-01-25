use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use pbkdf2::pbkdf2_hmac;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Sensitive SSH credentials
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SensitiveData {
    /// SSH password
    pub password: Option<String>,
    /// Passphrase for private keys
    pub key_passphrase: Option<String>,
}

pub struct EncryptionManager;

impl EncryptionManager {
    const ITERATIONS: u32 = 100_000;

    /// Gets a unique machine-specific ID to use as a master key.
    /// This is transparent to the user and avoids system keychain prompts.
    fn get_machine_id() -> String {
        machine_uid::get().unwrap_or_else(|_| "nexashell-fallback-id".into())
    }

    /// Encrypt sensitive data using the machine-specific ID.
    pub fn encrypt(data: &SensitiveData) -> Result<String, String> {
        Self::encrypt_with_key(data, &Self::get_machine_id())
    }

    /// Decrypt sensitive data using the machine-specific ID.
    pub fn decrypt(encrypted_base64: &str) -> Result<SensitiveData, String> {
        Self::decrypt_with_key(encrypted_base64, &Self::get_machine_id())
    }

    /// Encrypt sensitive data with a custom key (useful for export).
    pub fn encrypt_with_key(data: &SensitiveData, key_str: &str) -> Result<String, String> {
        let json = serde_json::to_string(data).map_err(|e| e.to_string())?;

        // 1. Generate random Salt
        let mut salt = [0u8; 16];
        thread_rng().fill_bytes(&mut salt);

        // 2. Derive key using PBKDF2
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(key_str.as_bytes(), &salt, Self::ITERATIONS, &mut key);

        // 3. Generate random IV (Nonce)
        let mut iv = [0u8; 12];
        thread_rng().fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);

        // 4. Encrypt
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
        let ciphertext = cipher
            .encrypt(nonce, json.as_bytes().as_ref())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // 5. Package: Salt(16) + IV(12) + Ciphertext
        let mut combined = salt.to_vec();
        combined.extend_from_slice(&iv);
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    /// Decrypt sensitive data with a custom key (useful for import).
    pub fn decrypt_with_key(
        encrypted_base64: &str,
        key_str: &str,
    ) -> Result<SensitiveData, String> {
        let combined = general_purpose::STANDARD
            .decode(encrypted_base64)
            .map_err(|e| format!("Invalid base64: {}", e))?;

        if combined.len() < 16 + 12 {
            return Err("Invalid encrypted data format".to_string());
        }

        // 1. Extract Salt, IV and Ciphertext
        let salt = &combined[0..16];
        let iv = &combined[16..28];
        let ciphertext = &combined[28..];

        // 2. Derive key
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(key_str.as_bytes(), salt, Self::ITERATIONS, &mut key);

        // 3. Decrypt
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
        let nonce = Nonce::from_slice(iv);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed (possibly wrong key): {}", e))?;

        let data: SensitiveData = serde_json::from_slice(&plaintext).map_err(|e| e.to_string())?;
        Ok(data)
    }
}
