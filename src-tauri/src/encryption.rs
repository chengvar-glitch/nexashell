use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};
use once_cell::sync::OnceCell;
use pbkdf2::pbkdf2_hmac;
use rand::{RngCore, thread_rng};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

const SERVICE_NAME: &str = "NexaShell";
const ACCOUNT_NAME: &str = "master-key";
const ITERATIONS: u32 = 390_000;
const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SensitiveData {
    pub password: Option<String>,
    pub key_passphrase: Option<String>,
}

impl Drop for SensitiveData {
    fn drop(&mut self) {
        if let Some(p) = self.password.as_mut() {
            p.zeroize();
        }
        if let Some(p) = self.key_passphrase.as_mut() {
            p.zeroize();
        }
    }
}

pub struct EncryptionManager;

impl EncryptionManager {
    pub fn init() {
        if let Err(e) = Self::get_master_key() {
            log::error!("Failed to initialize encryption manager: {}", e);
        }
    }

    /// Load or create the persistent random master key (cached after first call).
    ///
    /// The key is stored in the OS keychain (Keychain on macOS, DPAPI/Credential
    /// Manager on Windows, Secret Service on Linux). If the keychain is
    /// unavailable we fall back to a 0600-permissioned file inside the app data
    /// directory.
    fn get_master_key() -> Result<&'static [u8; KEY_LEN], String> {
        static MASTER_KEY: OnceCell<[u8; KEY_LEN]> = OnceCell::new();
        MASTER_KEY.get_or_try_init(|| {
            // Try OS keychain first
            match keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME) {
                Ok(entry) => match entry.get_password() {
                    Ok(b64) => {
                        let bytes = general_purpose::STANDARD
                            .decode(b64.trim())
                            .map_err(|e| format!("Invalid master key encoding: {}", e))?;
                        if bytes.len() == KEY_LEN {
                            let mut key = [0u8; KEY_LEN];
                            key.copy_from_slice(&bytes);
                            return Ok(key);
                        }
                        // Corrupt keychain entry. Refuse to fall through to
                        // "create a fresh key": that would silently overwrite
                        // the entry and permanently orphan every credential
                        // encrypted with the old key.
                        return Err(format!(
                            "Master key in keychain has invalid length ({} bytes, \
                             expected {}). Refusing to regenerate — saved credentials \
                             cannot be recovered.",
                            bytes.len(),
                            KEY_LEN
                        ));
                    }
                    Err(keyring::Error::NoEntry) => {
                        // Not found, create a new one below
                    }
                    Err(e) => {
                        log::warn!("Keychain unavailable ({}), falling back to file", e);
                    }
                },
                Err(e) => {
                    log::warn!("Keychain unavailable ({}), falling back to file", e);
                }
            }

            // Fall back to file storage
            Self::load_or_create_master_key_file()
        })
    }

    fn key_file_path() -> Result<std::path::PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| "Failed to determine app data directory".to_string())?
            .join("NexaShell");
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        Ok(data_dir.join(".encryption_master_key"))
    }

    fn load_or_create_master_key_file() -> Result<[u8; KEY_LEN], String> {
        let key_path = Self::key_file_path()?;

        if key_path.exists() {
            let bytes = std::fs::read(&key_path).map_err(|e| e.to_string())?;
            if bytes.len() == KEY_LEN {
                let mut key = [0u8; KEY_LEN];
                key.copy_from_slice(&bytes);

                // Attempt to migrate the key into the keychain for better security
                if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME) {
                    let b64 = general_purpose::STANDARD.encode(key);
                    if entry.set_password(&b64).is_ok() {
                        let _ = std::fs::remove_file(&key_path);
                    }
                }
                return Ok(key);
            }
            // Corrupt key file — refuse to overwrite, otherwise all existing
            // credentials would be permanently undecryptable.
            return Err(format!(
                "Master key file at {} is corrupt (expected {} bytes). \
                 Refusing to overwrite — saved credentials cannot be recovered.",
                key_path.display(),
                KEY_LEN
            ));
        }

        // Generate a fresh random key
        let mut key = [0u8; KEY_LEN];
        thread_rng().fill_bytes(&mut key);

        // Write to file with restrictive permissions
        std::fs::write(&key_path, key).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        }

        // Also try to store in keychain
        if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME) {
            let b64 = general_purpose::STANDARD.encode(key);
            if entry.set_password(&b64).is_ok() {
                let _ = std::fs::remove_file(&key_path);
            }
        }

        Ok(key)
    }

    pub fn encrypt(data: &SensitiveData) -> Result<String, String> {
        let key = Self::get_master_key()?;
        Self::encrypt_with_key_bytes(data, key)
    }

    pub fn decrypt(encrypted_base64: &str) -> Result<SensitiveData, String> {
        let key = Self::get_master_key()?;
        Self::decrypt_with_key_bytes(encrypted_base64, key)
    }

    /// Encrypt with a user-supplied passphrase (for export).
    ///
    /// Format: `v1$<base64(salt)><base64(nonce+ciphertext)>`.
    /// A fresh random salt is generated for every call.
    pub fn encrypt_with_key(data: &SensitiveData, key_str: &str) -> Result<String, String> {
        let mut salt = [0u8; SALT_LEN];
        thread_rng().fill_bytes(&mut salt);

        let mut key = [0u8; KEY_LEN];
        pbkdf2_hmac::<Sha256>(key_str.as_bytes(), &salt, ITERATIONS, &mut key);

        let inner = Self::encrypt_with_key_bytes(data, &key)?;
        key.zeroize();

        let salt_b64 = general_purpose::STANDARD.encode(salt);
        Ok(format!("v1${}${}", salt_b64, inner))
    }

    pub fn decrypt_with_key(encrypted: &str, key_str: &str) -> Result<SensitiveData, String> {
        // New format: v1$<salt_b64>$<nonce+ciphertext_b64>
        if let Some(rest) = encrypted.strip_prefix("v1$") {
            let mut parts = rest.splitn(2, '$');
            let salt_b64 = parts
                .next()
                .ok_or_else(|| "Invalid export format: missing salt".to_string())?;
            let inner = parts
                .next()
                .ok_or_else(|| "Invalid export format: missing payload".to_string())?;

            let salt = general_purpose::STANDARD
                .decode(salt_b64)
                .map_err(|e| format!("Invalid salt encoding: {}", e))?;
            if salt.len() != SALT_LEN {
                return Err("Invalid export salt length".to_string());
            }

            let mut key = [0u8; KEY_LEN];
            pbkdf2_hmac::<Sha256>(key_str.as_bytes(), &salt, ITERATIONS, &mut key);
            let result = Self::decrypt_with_key_bytes(inner, &key);
            key.zeroize();
            return result;
        }

        // Legacy format (v1.2.x and earlier): static salt, bare nonce+ciphertext
        let mut key = [0u8; KEY_LEN];
        let salt = b"nexashell-export";
        pbkdf2_hmac::<Sha256>(key_str.as_bytes(), salt, 100_000, &mut key);
        let result = Self::decrypt_with_key_bytes(encrypted, &key);
        key.zeroize();
        result
    }

    fn encrypt_with_key_bytes(
        data: &SensitiveData,
        key_bytes: &[u8; KEY_LEN],
    ) -> Result<String, String> {
        let json = serde_json::to_vec(data).map_err(|e| e.to_string())?;

        let mut iv = [0u8; NONCE_LEN];
        thread_rng().fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);

        let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;
        let ciphertext = cipher
            .encrypt(nonce, json.as_ref())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        combined.extend_from_slice(&iv);
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    fn decrypt_with_key_bytes(
        encrypted_base64: &str,
        key_bytes: &[u8; KEY_LEN],
    ) -> Result<SensitiveData, String> {
        let combined = general_purpose::STANDARD
            .decode(encrypted_base64)
            .map_err(|e| format!("Invalid base64: {}", e))?;

        if combined.len() < NONCE_LEN {
            return Err("Invalid encrypted data format".to_string());
        }

        let iv = &combined[..NONCE_LEN];
        let ciphertext = &combined[NONCE_LEN..];

        let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;
        let nonce = Nonce::from_slice(iv);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed (possibly wrong key): {}", e))?;

        let data: SensitiveData = serde_json::from_slice(&plaintext).map_err(|e| e.to_string())?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SensitiveData {
        SensitiveData {
            password: Some("hunter2".to_string()),
            key_passphrase: Some("pass-phrase-123".to_string()),
        }
    }

    #[test]
    fn roundtrip_with_master_key() {
        EncryptionManager::init();
        let data = sample();
        let enc = EncryptionManager::encrypt(&data).expect("encrypt");
        let dec = EncryptionManager::decrypt(&enc).expect("decrypt");
        assert_eq!(dec, data);
    }

    #[test]
    fn roundtrip_no_credentials() {
        EncryptionManager::init();
        let data = SensitiveData {
            password: None,
            key_passphrase: None,
        };
        let enc = EncryptionManager::encrypt(&data).expect("encrypt");
        let dec = EncryptionManager::decrypt(&enc).expect("decrypt");
        assert_eq!(dec, data);
    }

    #[test]
    fn export_roundtrip_new_format() {
        let data = sample();
        let enc = EncryptionManager::encrypt_with_key(&data, "export-pass").expect("encrypt");
        assert!(
            enc.starts_with("v1$"),
            "should use the v1$<salt>$<payload> format"
        );
        let dec = EncryptionManager::decrypt_with_key(&enc, "export-pass").expect("decrypt");
        assert_eq!(dec, data);
    }

    #[test]
    fn export_wrong_password_fails() {
        let data = sample();
        let enc = EncryptionManager::encrypt_with_key(&data, "correct").expect("encrypt");
        assert!(EncryptionManager::decrypt_with_key(&enc, "wrong").is_err());
    }

    #[test]
    fn legacy_export_format_still_decrypts() {
        // Simulate the pre-v1.3 format: static salt, bare nonce+ciphertext.
        let data = sample();
        let mut key = [0u8; KEY_LEN];
        let salt = b"nexashell-export";
        pbkdf2_hmac::<Sha256>(b"legacy-pass", salt, 100_000, &mut key);
        let inner = EncryptionManager::encrypt_with_key_bytes(&data, &key).expect("encrypt");
        key.zeroize();

        let dec = EncryptionManager::decrypt_with_key(&inner, "legacy-pass").expect("decrypt");
        assert_eq!(dec, data);
    }
}
