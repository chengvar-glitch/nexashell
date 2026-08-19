//! Host key verification using a Trust-On-First-Use (TOFU) policy.
//!
//! Split out of `ssh.rs` to keep the SSH manager focused on connection and
//! I/O lifecycle. All functions here are free functions invoked at connect
//! time (inside `spawn_blocking`), never across an await point.

use super::SshError;
use ssh2::{HashType, Session};
use std::collections::BTreeMap;
use std::io::Write;

// Serializes host-key verification + known_hosts persistence so two concurrent
// first-time connections cannot race on a read-modify-write of the file.
// Held for the duration of verify_host_key only (it is called inside
// spawn_blocking), never across any await or network operation.
static KNOWN_HOSTS_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

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

/// Atomically persist the known_hosts map with a deterministic (sorted) layout
/// and an fsync so a crash mid-write cannot leave a truncated file.
fn persist_known_hosts(path: &std::path::Path, known: &BTreeMap<String, String>) -> Result<(), String> {
    let mut serialized = String::new();
    for (host, fp) in known {
        serialized.push_str(&format!("{} {}\n", host, fp));
    }
    // Write to a temp file in the same directory, then rename over the target
    // so readers never observe a partially-written file.
    let tmp = path.with_extension("known_hosts.tmp");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    write!(file, "{}", serialized).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    // Re-apply restrictive permissions on Unix (best effort).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Verify the host key for `host` using a TOFU policy.
///
/// - First connection: store the fingerprint and accept.
/// - Subsequent connections: reject if the fingerprint differs (possible MITM).
///
/// Persistence failures are treated as a hard error (fail-closed): silently
/// accepting a key that was never pinned would disable MITM protection for the
/// host while the app claims TOFU is in effect.
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

    let mut known: BTreeMap<String, String> = if path.exists() {
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
                BTreeMap::new()
            }
        }
    } else {
        BTreeMap::new()
    };

    if let Some(existing) = known.get(host) {
        if existing != &fingerprint {
            return Err(SshError::HostKeyVerificationFailed {
                host: host.to_string(),
                reason: format!(
                    "Host key mismatch!\nStored: {}\nGot:    {}\n
To trust the new key, forget the old one (Settings -> \"Forget host key\") or
remove the entry manually from: {}",
                    existing, fingerprint, path.display()
                ),
            });
        }
        return Ok(());
    }

    // First time seeing this host — store and accept.
    known.insert(host.to_string(), fingerprint);
    if let Err(e) = persist_known_hosts(&path, &known) {
        return Err(SshError::HostKeyVerificationFailed {
            host: host.to_string(),
            reason: format!(
                "Host key was accepted but could NOT be persisted ({}). The key is \
                 not pinned, so future connections would re-trip the first-time \
                 path. Fix the file permission/disk issue and retry: {}",
                e,
                path.display()
            ),
        });
    }

    Ok(())
}

/// Remove a host's pinned key so the next connection re-enters the first-time
/// (accept-and-pin) path. Used as the remediation for a rotated host key.
pub fn forget_host_key(host: &str) -> Result<(), String> {
    let _guard = KNOWN_HOSTS_LOCK
        .lock()
        .map_err(|e| format!("known_hosts lock poisoned: {}", e))?;

    let path = known_hosts_path()?;
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut known: BTreeMap<String, String> = content
        .lines()
        .filter_map(|line| {
            let (h, fp) = line.split_once(' ')?;
            Some((h.to_string(), fp.to_string()))
        })
        .collect();
    if known.remove(host).is_none() {
        // Nothing pinned for this host — nothing to do.
        return Ok(());
    }
    persist_known_hosts(&path, &known)
}