use keyring::Entry;
use serde::{Deserialize, Serialize};

/// Struct representing sensitive SSH credentials stored in system keychain
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SensitiveData {
    /// SSH password for password-based authentication
    pub password: Option<String>,
    /// Passphrase for encrypted private keys
    pub key_passphrase: Option<String>,
}

/// Cross-platform keychain manager for storing SSH credentials
/// Uses system Keychain on macOS and Credential Manager on Windows
pub struct KeychainManager;

impl KeychainManager {
    const SERVICE_NAME: &'static str = "NexaShell";

    /// Save sensitive credentials to system keychain
    ///
    /// # Arguments
    /// * `session_id` - Unique session identifier
    /// * `data` - Sensitive data containing password and/or key passphrase
    ///
    /// # Returns
    /// Result indicating success or error message
    pub fn save_credentials(session_id: &str, data: &SensitiveData) -> Result<(), String> {
        // Save password if present
        if let Some(password) = &data.password {
            let entry = Entry::new(Self::SERVICE_NAME, &format!("ssh_password_{}", session_id))
                .map_err(|e| format!("Failed to create keychain entry for password: {}", e))?;
            entry
                .set_password(password)
                .map_err(|e| format!("Failed to save password to keychain: {}", e))?;
        }

        // Save key passphrase if present
        if let Some(passphrase) = &data.key_passphrase {
            let entry = Entry::new(
                Self::SERVICE_NAME,
                &format!("ssh_passphrase_{}", session_id),
            )
            .map_err(|e| {
                format!(
                    "Failed to create keychain entry for passphrase: {}",
                    e
                )
            })?;
            entry.set_password(passphrase).map_err(|e| {
                format!("Failed to save passphrase to keychain: {}", e)
            })?;
        }

        Ok(())
    }

    /// Retrieve sensitive credentials from system keychain
    ///
    /// # Arguments
    /// * `session_id` - Unique session identifier
    ///
    /// # Returns
    /// SensitiveData struct with retrieved credentials (None for missing items)
    pub fn retrieve_credentials(session_id: &str) -> Result<SensitiveData, String> {
        let password = Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh_password_{}", session_id),
        )
        .ok()
        .and_then(|e| e.get_password().ok());

        let key_passphrase = Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh_passphrase_{}", session_id),
        )
        .ok()
        .and_then(|e| e.get_password().ok());

        Ok(SensitiveData {
            password,
            key_passphrase,
        })
    }

    /// Delete all stored credentials for a session from keychain
    ///
    /// # Arguments
    /// * `session_id` - Unique session identifier
    ///
    /// # Returns
    /// Result indicating success or error message
    pub fn delete_credentials(session_id: &str) -> Result<(), String> {
        // Attempt to delete password entry (ignore if not found)
        let _ = Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh_password_{}", session_id),
        )
        .ok()
        .and_then(|e| e.delete_password().ok());

        // Attempt to delete passphrase entry (ignore if not found)
        let _ = Entry::new(
            Self::SERVICE_NAME,
            &format!("ssh_passphrase_{}", session_id),
        )
        .ok()
        .and_then(|e| e.delete_password().ok());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitive_data_serialization() {
        let data = SensitiveData {
            password: Some("test123".to_string()),
            key_passphrase: Some("passphrase".to_string()),
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("password"));
        assert!(json.contains("keyPassphrase")); // camelCase due to serde attribute
    }
}
