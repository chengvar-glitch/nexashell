//! Host key verification using a Trust-On-First-Use (TOFU) policy.
//!
//! Split out of `ssh.rs` to keep the SSH manager focused on connection and
//! I/O lifecycle. All functions here are free functions invoked at connect
//! time (inside `spawn_blocking`), never across an await point.

use crate::ssh::SshError;
use ssh2::{HashType, Session};
use std::collections::HashMap;

// Serializes host-key verification + known_hosts persistence so two concurrent
// first-time connections cannot race on a read-modify-write of the file.
// Held for the duration of verify_host_key only (it is called inside
// spawn_blocking), never across any await or network operation.
static KNOWN_HOSTS_LOCK: once_cell::sync::Lazy<std::sync::Mutex<()>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(()));

fn known_hosts_path() -> Result<std::path::PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?
        .join("NexaShell");
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok(data_dir.join("known_hosts"))
}

/// Returns the SHA256 fingerprint of the session's host key, base64-encoded
/// (without the `SHA256:` prefix to match OpenSSH's visual representation
/// minus the prefix).
fn host_key_fingerprint(sess: &Session) -> Result<String, String> {
    let key = sess
        .host_key_hash(HashType::Sha256)
        .ok_or_else(|| "No host key available".to_string())?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        key,
    ))
}

/// Verify the host key for `host` using a TOFU policy.
///
/// - First connection: store the fingerprint and accept.
/// - Subsequent connections: reject if the fingerprint differs (possible MITM).
pub fn verify_host_key(sess: &Session, host: &str) -> Result<(), SshError> {
    // Serialize the read-modify-write of known_hosts so two concurrent
    // first-time connections cannot clobber each other's entry (or interleave
    // a partial file write). Cheap: verify_host_key only runs at connect time.
    let _guard = KNOWN_HOSTS_LOCK
        .lock()
        .map_err(|e| SshError::LockPoisoned(format!("known_hosts lock poisoned: {}", e)))?;

    let fingerprint =
        host_key_fingerprint(sess).map_err(|e| SshError::HostKeyVerificationFailed {
            host: host.to_string(),
            reason: e,
        })?;

    let path = known_hosts_path().map_err(|e| SshError::HostKeyVerificationFailed {
        host: host.to_string(),
        reason: e,
    })?;

    let mut known: HashMap<String, String> = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => content
                .lines()
                .filter_map(|line| {
                    let (h, fp) = line.split_once(' ')?;

                    Some((h.to_string(), fp.to_string()))
                })
                .collect(),
            Err(e) => {
                log::warn!("Failed to read known_hosts ({}): {}", path.display(), e);
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    if let Some(existing) = known.get(host) {
        if existing != &fingerprint {
            return Err(SshError::HostKeyVerificationFailed {
                host: host.to_string(),
                reason: format!(
                    "Host key mismatch!\nStored: {}\nGot:    {}",
                    existing, fingerprint
                ),
            });
        }
        return Ok(());
    }

    // First time seeing this host — store and accept
    known.insert(host.to_string(), fingerprint);
    let serialized: String = known
        .iter()
        .map(|(h, fp)| format!("{} {}\n", h, fp))
        .collect();
    if let Err(e) = std::fs::write(&path, serialized) {
        log::warn!("Failed to write known_hosts: {}", e);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}
