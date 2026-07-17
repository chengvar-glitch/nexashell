use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::OnceCell;
use pbkdf2::pbkdf2_hmac;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SensitiveData {
    pub password: Option<String>,
    pub key_passphrase: Option<String>,
}

pub struct EncryptionManager;

impl EncryptionManager {
    const ITERATIONS: u32 = 100_000;
    const KEY_FILE_NAME: &str = ".encryption_master_key";

    pub fn init() {
        let _ = Self::get_master_key();
    }

    fn key_file_path() -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| "Failed to determine app data directory".to_string())?
            .join("NexaShell");
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        Ok(data_dir.join(Self::KEY_FILE_NAME))
    }

    /// Load or create a persistent random master key (cached after first call).
    fn get_master_key() -> Result<&'static [u8; 32], String> {
        static MASTER_KEY: OnceCell<[u8; 32]> = OnceCell::new();
        MASTER_KEY.get_or_try_init(|| {
            let key_path = Self::key_file_path()?;

            if key_path.exists() {
                let bytes = fs::read(&key_path).map_err(|e| e.to_string())?;
                if bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    return Ok(key);
                }
            }

            let machine_seed = machine_uid::get().unwrap_or_else(|_| {
                let fallback = format!("nexashell-{}", rand::random::<u64>());
                eprintln!(
                    "Warning: machine_uid unavailable, using fallback seed. \
                     Existing encrypted data may be lost if this changes."
                );
                fallback
            });

            let mut salt = [0u8; 16];
            thread_rng().fill_bytes(&mut salt);
            let mut key = [0u8; 32];
            pbkdf2_hmac::<Sha256>(machine_seed.as_bytes(), &salt, Self::ITERATIONS, &mut key);

            fs::write(&key_path, &key).map_err(|e| e.to_string())?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
            }

            Ok(key)
        }).map_err(|e: String| e)
    }

    pub fn encrypt(data: &SensitiveData) -> Result<String, String> {
        let key = Self::get_master_key()?;
        Self::encrypt_with_key_bytes(data, key)
    }

    pub fn decrypt(encrypted_base64: &str) -> Result<SensitiveData, String> {
        let key = Self::get_master_key()?;
        Self::decrypt_with_key_bytes(encrypted_base64, key)
    }

    pub fn encrypt_with_key(data: &SensitiveData, key_str: &str) -> Result<String, String> {
        let mut key = [0u8; 32];
        let salt = b"nexashell-export";
        pbkdf2_hmac::<Sha256>(key_str.as_bytes(), salt, Self::ITERATIONS, &mut key);
        Self::encrypt_with_key_bytes(data, &key)
    }

    pub fn decrypt_with_key(
        encrypted_base64: &str,
        key_str: &str,
    ) -> Result<SensitiveData, String> {
        let mut key = [0u8; 32];
        let salt = b"nexashell-export";
        pbkdf2_hmac::<Sha256>(key_str.as_bytes(), salt, Self::ITERATIONS, &mut key);
        Self::decrypt_with_key_bytes(encrypted_base64, &key)
    }

    fn encrypt_with_key_bytes(data: &SensitiveData, key_bytes: &[u8; 32]) -> Result<String, String> {
        let json = serde_json::to_vec(data).map_err(|e| e.to_string())?;

        let mut iv = [0u8; 12];
        thread_rng().fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);

        let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;
        let ciphertext = cipher
            .encrypt(nonce, json.as_ref())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut combined = iv.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    fn decrypt_with_key_bytes(
        encrypted_base64: &str,
        key_bytes: &[u8; 32],
    ) -> Result<SensitiveData, String> {
        let combined = general_purpose::STANDARD
            .decode(encrypted_base64)
            .map_err(|e| format!("Invalid base64: {}", e))?;

        if combined.len() < 12 {
            return Err("Invalid encrypted data format".to_string());
        }

        let iv = &combined[0..12];
        let ciphertext = &combined[12..];

        let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;
        let nonce = Nonce::from_slice(iv);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed (possibly wrong key): {}", e))?;

        let data: SensitiveData = serde_json::from_slice(&plaintext).map_err(|e| e.to_string())?;
        Ok(data)
    }
}
