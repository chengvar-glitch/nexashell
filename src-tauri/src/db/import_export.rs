//! Session export/import (portable encrypted backups).
//!
//! Split out of `mod.rs` to keep the session/relation CRUD focused. Both
//! functions are careful to avoid holding the shared DB lock during the
//! expensive PBKDF2 key-derivation (390k iterations per session).

use super::{
    ExportData, ExportSession, Group, SESSION_COLUMNS, Session, Tag, db_conn, row_to_session,
    with_db,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[tauri::command]
pub fn export_sessions(password: String) -> Result<String, String> {
    // Read all raw data under the lock, then release it before doing the
    // expensive PBKDF2 key derivation (hundreds of ms per session).
    struct RawData {
        sessions: Vec<(Session, Option<String>)>,
        group_map: std::collections::HashMap<String, Vec<String>>,
        tag_map: std::collections::HashMap<String, Vec<String>>,
        groups: Vec<Group>,
        tags: Vec<Tag>,
    }

    let raw = with_db(|conn| {
        let sql = format!(
            "SELECT {}, encrypted_credentials FROM sessions",
            SESSION_COLUMNS
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let session_rows = stmt
            .query_map([], |row| {
                let mut s = row_to_session(row)?;
                // When using SELECT with extra column at end, indices shift:
                // encrypted_credentials is column 11.
                let _ = &mut s;
                let encrypted: Option<String> = row.get(11)?;
                Ok((s, encrypted))
            })
            .map_err(|e| e.to_string())?;
        let sessions = session_rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);

        let mut group_map = std::collections::HashMap::new();
        let mut g = conn
            .prepare("SELECT session_id, group_id FROM session_groups")
            .map_err(|e| e.to_string())?;
        let rows = g
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (sid, gid): (String, String) = r.map_err(|e| e.to_string())?;
            group_map.entry(sid).or_insert_with(Vec::new).push(gid);
        }
        drop(g);

        let mut tag_map = std::collections::HashMap::new();
        let mut t = conn
            .prepare("SELECT session_id, tag_id FROM session_tags")
            .map_err(|e| e.to_string())?;
        let rows = t
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (sid, tid): (String, String) = r.map_err(|e| e.to_string())?;
            tag_map.entry(sid).or_insert_with(Vec::new).push(tid);
        }
        drop(t);

        let groups = {
            let mut s = conn
                .prepare("SELECT id, name, sort, created_at, updated_at FROM groups")
                .map_err(|e| e.to_string())?;
            let rows = s
                .query_map([], |row| {
                    Ok(Group {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        sort: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let tags = {
            let mut s = conn
                .prepare("SELECT id, name, color, sort, created_at, updated_at FROM tags")
                .map_err(|e| e.to_string())?;
            let rows = s
                .query_map([], |row| {
                    Ok(Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        color: row.get(2)?,
                        sort: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        Ok(RawData {
            sessions,
            group_map,
            tag_map,
            groups,
            tags,
        })
    })?;

    let mut group_map = raw.group_map;
    let mut tag_map = raw.tag_map;
    let mut export_sessions = Vec::with_capacity(raw.sessions.len());
    for (metadata, encrypted_creds) in raw.sessions {
        let re_encrypted = if let Some(creds) = encrypted_creds {
            let sensitive = crate::encryption::EncryptionManager::decrypt(&creds)?;
            Some(crate::encryption::EncryptionManager::encrypt_with_key(
                &sensitive, &password,
            )?)
        } else {
            None
        };

        export_sessions.push(ExportSession {
            group_ids: group_map.remove(&metadata.id).unwrap_or_default(),
            tag_ids: tag_map.remove(&metadata.id).unwrap_or_default(),
            metadata,
            encrypted_credentials: re_encrypted,
        });
    }

    let data = ExportData {
        sessions: export_sessions,
        groups: raw.groups,
        tags: raw.tags,
    };
    serde_json::to_string(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_sessions(json_data: String, password: String) -> Result<(), String> {
    // Guard against unbounded payloads: the raw string is parsed in the main
    // process, so a corrupted/malicious multi-gigabyte file would force the
    // whole thing into memory. 50 MB comfortably covers any realistic backup.
    const MAX_IMPORT_BYTES: usize = 50 * 1024 * 1024;
    if json_data.len() > MAX_IMPORT_BYTES {
        return Err(format!(
            "Import payload too large ({} bytes, max {} bytes)",
            json_data.len(),
            MAX_IMPORT_BYTES
        ));
    }

    let export_data: ExportData = serde_json::from_str(&json_data).map_err(|e| e.to_string())?;

    // Reject duplicate session ids inside a single file up front: INSERT OR
    // REPLACE would silently overwrite the earlier copy — including its
    // relation set — which is silent data loss the user never asked for.
    let mut seen_session_ids = std::collections::HashSet::new();
    for session in &export_data.sessions {
        if !seen_session_ids.insert(session.metadata.id.clone()) {
            return Err(format!(
                "Duplicate session id in import payload: {}",
                session.metadata.id
            ));
        }
    }

    // Decrypt/re-encrypt all credentials BEFORE taking the DB lock — PBKDF2
    // (390k iterations per session) must not block every other DB call.
    let mut prepared_sessions = Vec::with_capacity(export_data.sessions.len());
    for mut session in export_data.sessions {
        let re_encrypted = if let Some(creds) = session.encrypted_credentials.take() {
            let sensitive =
                crate::encryption::EncryptionManager::decrypt_with_key(&creds, &password)?;
            Some(crate::encryption::EncryptionManager::encrypt(&sensitive)?)
        } else {
            None
        };
        prepared_sessions.push((session, re_encrypted));
    }

    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for group in export_data.groups {
        tx.execute(
            "INSERT OR IGNORE INTO groups (id, name, sort, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![group.id, group.name, group.sort, group.created_at, group.updated_at],
        ).map_err(|e| e.to_string())?;
    }

    for tag in export_data.tags {
        tx.execute(
            "INSERT OR IGNORE INTO tags (id, name, color, sort, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![tag.id, tag.name, tag.color, tag.sort, tag.created_at, tag.updated_at],
        ).map_err(|e| e.to_string())?;
    }

    for (session, re_encrypted) in prepared_sessions {
        let metadata = session.metadata;

        tx.execute(
            "INSERT OR REPLACE INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite, encrypted_credentials, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                metadata.id, metadata.addr, metadata.port, metadata.server_name,
                metadata.username, metadata.auth_type, metadata.private_key_path,
                if metadata.is_favorite { 1 } else { 0 }, re_encrypted,
                metadata.created_at, metadata.updated_at
            ],
        ).map_err(|e| e.to_string())?;

        // Relation writes MUST propagate errors: a partially-applied link set
        // committing silently would leave the import inconsistent. Every other
        // statement here rolls back on failure via `?`.
        tx.execute(
            "DELETE FROM session_groups WHERE session_id = ?1",
            params![metadata.id],
        )
        .map_err(|e| e.to_string())?;
        for gid in session.group_ids {
            tx.execute(
                "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
                params![metadata.id, gid],
            )
            .map_err(|e| format!("Failed to link group {}: {}", gid, e))?;
        }

        tx.execute(
            "DELETE FROM session_tags WHERE session_id = ?1",
            params![metadata.id],
        )
        .map_err(|e| e.to_string())?;
        for tid in session.tag_ids {
            tx.execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![metadata.id, tid],
            )
            .map_err(|e| format!("Failed to link tag {}: {}", tid, e))?;
        }
    }

    tx.commit().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// XTerminal format import
// ---------------------------------------------------------------------------
//
// XTerminal's export ("复制到剪贴板" / server list) uses one labeled block per
// session, e.g.:
//
//   名称: fofo
//   地址: 8.166.133.7
//   端口: 22
//   用户: root
//   密码: fofo0898.
//
// Its documented *import* text modes are also accepted for convenience:
//   - key=value lines: host=... port=... user=... pass=... title=... auth=...
//   - pipe format:     host[:port] | user | pass | title | note
//
// Blocks are separated by blank lines; a second `名称`/`title` line inside an
// in-progress block also starts a new record, so sessions pasted back-to-back
// without blank lines still parse correctly.

/// One parsed session from XTerminal text (password still plaintext here; it is
/// encrypted before it ever touches the database).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedXTerminalSession {
    pub server_name: String,
    pub addr: String,
    pub port: i64,
    pub username: String,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
}

/// Result of an XTerminal import, reported back to the frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub failed: Vec<String>,
}

/// Recognized label keys inside a key-value line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Addr,
    Port,
    User,
    Password,
    PrivateKey,
    /// Parsed and ignored (note/tags/auth/... — nothing to store).
    Ignored,
}

/// Map a label to a field. Accepts XTerminal's Chinese labels, English labels
/// and the English keys of its documented import format.
fn classify_key(key: &str) -> Option<Field> {
    match key.trim().to_lowercase().as_str() {
        "名称" | "名字" | "主机名" | "name" | "title" | "hostname" => Some(Field::Name),
        "地址" | "主机" | "ip" | "host" | "address" | "服务器地址" => Some(Field::Addr),
        "端口" | "port" => Some(Field::Port),
        "用户" | "用户名" | "账号" | "user" | "username" => Some(Field::User),
        "密码" | "口令" | "pass" | "password" => Some(Field::Password),
        "密钥" | "私钥" | "私钥路径" | "key" | "privatekey" | "private_key" => {
            Some(Field::PrivateKey)
        }
        "备注" | "note" | "标签" | "tag" | "tags" | "auth" | "认证" | "认证方式" | "identityid"
        | "defaultpath" | "timeout" | "initscript" | "passphrase" => Some(Field::Ignored),
        _ => None,
    }
}

/// Split `名称: value`, `名称：value` or `key=value` into (key, value).
fn split_kv(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    for (idx, ch) in trimmed.char_indices() {
        if ch == ':' || ch == '：' || ch == '=' {
            let key = trimmed[..idx].trim();
            let value = trimmed[idx + ch.len_utf8()..].trim();
            if !key.is_empty() {
                return Some((key.to_string(), value.to_string()));
            }
            return None;
        }
    }
    None
}

/// A single in-progress record while scanning lines.
#[derive(Default)]
struct Record {
    name: Option<String>,
    addr: Option<String>,
    port: Option<String>,
    username: Option<String>,
    password: Option<String>,
    private_key_path: Option<String>,
}

fn parse_port(raw: &str) -> Result<i64, String> {
    match raw.trim().parse::<i64>() {
        Ok(p) if (1..=65535).contains(&p) => Ok(p),
        Ok(_) => Err(format!("Port out of range (1-65535): {}", raw.trim())),
        Err(_) => Err(format!("Invalid port: {}", raw.trim())),
    }
}

/// Finalize `record` into a session, or push an error when it cannot be used.
fn finish_record(
    record: Record,
    errors: &mut Vec<String>,
    sessions: &mut Vec<ParsedXTerminalSession>,
) {
    let addr = match record.addr {
        Some(addr) => addr.trim().to_string(),
        None => {
            let label = record.name.unwrap_or_else(|| "(unnamed)".to_string());
            errors.push(format!("Missing address, skipped: {}", label));
            return;
        }
    };
    if addr.is_empty() {
        errors.push("Empty address, skipped".to_string());
        return;
    }

    let port = match record.port.as_deref() {
        Some(raw) => match parse_port(raw) {
            Ok(p) => p,
            Err(e) => {
                let label = record
                    .name
                    .clone()
                    .unwrap_or_else(|| addr.clone());
                errors.push(format!("{} ({})", e, label));
                return;
            }
        },
        None => 22,
    };

    let username = record
        .username
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "root".to_string());

    let server_name = record
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| addr.clone());

    let password = record
        .password
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());
    let private_key_path = record
        .private_key_path
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());

    sessions.push(ParsedXTerminalSession {
        server_name,
        addr,
        port,
        username,
        password,
        private_key_path,
    });
}

/// Parse XTerminal text into sessions. Returns `(sessions, per-record errors)`.
/// Unknown/comment/header lines are ignored silently; lines that look like a
/// record but are malformed produce an error entry.
pub fn parse_xterminal_text(text: &str) -> (Vec<ParsedXTerminalSession>, Vec<String>) {
    let mut sessions = Vec::new();
    let mut errors = Vec::new();
    let mut record = Record::default();
    let mut record_started = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        // Blank line ends the current record.
        if line.is_empty() {
            flush_record(&mut record, &mut record_started, &mut sessions, &mut errors);
            continue;
        }
        // Comment lines (XTerminal import format).
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // Pipe format: `host[:port] | user | pass | title | note` (one per line).
        // Checked before key-value parsing because a host may itself contain
        // ':' (e.g. `user@10.0.0.3:2222 | ...`).
        if line.contains('|') {
            flush_record(&mut record, &mut record_started, &mut sessions, &mut errors);
            match parse_pipe_line(line) {
                Some(session) => sessions.push(session),
                None => errors.push(format!("Unparseable line, skipped: {}", line)),
            }
            continue;
        }

        // Key-value line.
        if let Some((key, value)) = split_kv(line) {
            match classify_key(&key) {
                // A name line while a record is already in progress starts a
                // new one (handles sessions pasted without blank lines). Only
                // flush when the record already carries a name — XTerminal's
                // key=value import format may put `title` at the END of a
                // record, so a single `title` line must not split it.
                Some(Field::Name) => {
                    if record_started && record.name.is_some() {
                        flush_record(&mut record, &mut record_started, &mut sessions, &mut errors);
                    }
                    record.name = Some(value);
                    record_started = true;
                }
                Some(Field::Addr) => {
                    record.addr = Some(value);
                    record_started = true;
                }
                Some(Field::Port) => {
                    record.port = Some(value);
                    record_started = true;
                }
                Some(Field::User) => {
                    record.username = Some(value);
                    record_started = true;
                }
                Some(Field::Password) => {
                    record.password = Some(value);
                    record_started = true;
                }
                Some(Field::PrivateKey) => {
                    record.private_key_path = Some(value);
                    record_started = true;
                }
                Some(Field::Ignored) => {}
                None => {
                    // Not a recognized label — ignore (export headers etc.).
                }
            }
            continue;
        }

        // Unrecognized non-empty line (e.g. an export header): ignore.
    }

    flush_record(&mut record, &mut record_started, &mut sessions, &mut errors);
    (sessions, errors)
}

/// Flush a finished record into `sessions` (or `errors` when malformed).
fn flush_record(
    record: &mut Record,
    started: &mut bool,
    sessions: &mut Vec<ParsedXTerminalSession>,
    errors: &mut Vec<String>,
) {
    if *started {
        finish_record(std::mem::take(record), errors, sessions);
        *started = false;
    }
}

/// Parse one pipe-format line: `host[:port] | user | pass | title | note`.
/// Also accepts `user@host[:port]` as the first field (username embedded).
fn parse_pipe_line(line: &str) -> Option<ParsedXTerminalSession> {
    let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return None;
    }
    let (host_part, user_part, pass_part, title_part) = (
        parts[0],
        parts[1],
        parts.get(2).copied().unwrap_or(""),
        parts.get(3).copied().unwrap_or(""),
    );

    // `user@host[:port]` takes precedence over the dedicated user column.
    let (embedded_user, host) = match host_part.rfind('@') {
        Some(idx) => (&host_part[..idx], host_part[idx + 1..].trim()),
        None => ("", host_part),
    };

    let (addr, port) = match host.rfind(':') {
        Some(idx) => (
            host[..idx].trim(),
            match host[idx + 1..].trim().parse::<i64>() {
                Ok(p) if (1..=65535).contains(&p) => Some(p),
                _ => None,
            },
        ),
        None => (host, None),
    };

    if addr.is_empty() {
        return None;
    }

    let username = if !embedded_user.is_empty() {
        embedded_user.to_string()
    } else {
        user_part.to_string()
    };

    Some(ParsedXTerminalSession {
        server_name: if title_part.is_empty() {
            addr.to_string()
        } else {
            title_part.to_string()
        },
        addr: addr.to_string(),
        port: port.unwrap_or(22),
        username,
        password: if pass_part.is_empty() {
            None
        } else {
            Some(pass_part.to_string())
        },
        private_key_path: None,
    })
}

/// Import sessions from XTerminal-format text. Returns how many sessions were
/// imported, how many were skipped as duplicates, and any per-record failures.
///
/// Duplicates are detected by (addr, port, username) against both the existing
/// database and sessions already inserted by this call, so re-importing the
/// same export does not create duplicates.
#[tauri::command]
pub fn import_xterminal_sessions(text: String) -> Result<ImportResult, String> {
    let (parsed, mut errors) = parse_xterminal_text(&text);

    // Existing keys so re-imports never duplicate.
    let mut seen: HashSet<(String, i64, String)> = with_db(|conn| {
        let mut stmt = conn
            .prepare("SELECT addr, port, username FROM sessions")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    })?;

    let mut imported = 0usize;
    let mut skipped = 0usize;

    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for session in parsed {
        let key = (session.addr.clone(), session.port, session.username.clone());
        if seen.contains(&key) {
            skipped += 1;
            continue;
        }

        let auth_type = if session.private_key_path.is_some() {
            "key"
        } else {
            "password"
        };

        // Encrypt the plaintext password with the app master key. Failures are
        // per-session: report and skip, never abort the whole import. When a
        // private key is present, the `密码` value is treated as the key
        // passphrase rather than a login password.
        let encrypted = match session.password.as_ref() {
            Some(password) => {
                let sensitive = if session.private_key_path.is_some() {
                    crate::encryption::SensitiveData {
                        password: None,
                        key_passphrase: Some(password.clone()),
                    }
                } else {
                    crate::encryption::SensitiveData {
                        password: Some(password.clone()),
                        key_passphrase: None,
                    }
                };
                match crate::encryption::EncryptionManager::encrypt(&sensitive) {
                    Ok(enc) => Some(enc),
                    Err(e) => {
                        errors.push(format!(
                            "Failed to encrypt credentials ({}): {}",
                            session.server_name, e
                        ));
                        continue;
                    }
                }
            }
            None => None,
        };

        let id = Uuid::new_v4().to_string();
        let inserted = tx
            .execute(
                "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite, encrypted_credentials)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
                params![
                    id,
                    session.addr,
                    session.port,
                    session.server_name,
                    session.username,
                    auth_type,
                    session.private_key_path,
                    encrypted,
                ],
            )
            .map_err(|e| e.to_string());

        match inserted {
            Ok(_) => {
                imported += 1;
                seen.insert(key);
            }
            Err(e) => errors.push(format!("Failed to insert ({}): {}", session.server_name, e)),
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(ImportResult {
        imported,
        skipped,
        failed: errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_session_from_user_sample() {
        let text = "名称: fofo\n地址: 8.166.133.7\n端口: 22\n用户: root\n密码: fofo0898.\n";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.server_name, "fofo");
        assert_eq!(s.addr, "8.166.133.7");
        assert_eq!(s.port, 22);
        assert_eq!(s.username, "root");
        assert_eq!(s.password.as_deref(), Some("fofo0898."));
        assert_eq!(s.private_key_path, None);
    }

    #[test]
    fn parses_multiple_blocks_separated_by_blank_lines() {
        let text = "\
名称: a
地址: 1.1.1.1
端口: 2222
用户: admin
密码: p1

名称: b
地址: 2.2.2.2
用户: root
密码: p2
";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].addr, "1.1.1.1");
        assert_eq!(sessions[0].port, 2222);
        assert_eq!(sessions[0].username, "admin");
        assert_eq!(sessions[1].addr, "2.2.2.2");
        assert_eq!(sessions[1].port, 22); // default
        assert_eq!(sessions[1].username, "root");
    }

    #[test]
    fn handles_fullwidth_colon_and_english_aliases() {
        let text = "名称：生产\n地址：10.0.0.9\n端口：2201\n用户：deploy\n密码：s3cret";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.server_name, "生产");
        assert_eq!(s.addr, "10.0.0.9");
        assert_eq!(s.port, 2201);
        assert_eq!(s.username, "deploy");
        assert_eq!(s.password.as_deref(), Some("s3cret"));
    }

    #[test]
    fn parses_english_key_value_import_format() {
        let text = "host=10.0.0.5\nuser=root\nauth=privateKey\nprivateKey=/Users/me/.ssh/id_rsa\ntitle=KeyLogin";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.server_name, "KeyLogin");
        assert_eq!(s.addr, "10.0.0.5");
        assert_eq!(s.username, "root");
        assert_eq!(s.private_key_path.as_deref(), Some("/Users/me/.ssh/id_rsa"));
    }

    #[test]
    fn name_line_without_blank_separator_starts_new_record() {
        let text = "名称: one\n地址: 1.1.1.1\n名称: two\n地址: 2.2.2.2\n";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].server_name, "one");
        assert_eq!(sessions[1].server_name, "two");
    }

    #[test]
    fn parses_pipe_format_line() {
        let text = "192.168.1.10 | root | 123456 | Dev Server | note\nuser@10.0.0.3:2222 |  |  | 测试机\n";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].addr, "192.168.1.10");
        assert_eq!(sessions[0].port, 22);
        assert_eq!(sessions[0].username, "root");
        assert_eq!(sessions[0].password.as_deref(), Some("123456"));
        assert_eq!(sessions[0].server_name, "Dev Server");
        assert_eq!(sessions[1].addr, "10.0.0.3");
        assert_eq!(sessions[1].port, 2222);
        assert_eq!(sessions[1].username, "user");
        assert_eq!(sessions[1].server_name, "测试机");
    }

    #[test]
    fn private_key_field_sets_key_path_and_ignores_notes() {
        let text = "名称: keyhost\n地址: 3.3.3.3\n密钥: /Users/me/.ssh/id_rsa\n备注: prod\n标签: x\n";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.private_key_path.as_deref(), Some("/Users/me/.ssh/id_rsa"));
        assert_eq!(s.password, None);
        assert_eq!(s.server_name, "keyhost");
    }

    #[test]
    fn missing_addr_is_reported_as_error() {
        let text = "名称: orphan\n端口: 22\n";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(sessions.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Missing address"));
    }

    #[test]
    fn invalid_port_is_reported_as_error() {
        let text = "名称: bad\n地址: 4.4.4.4\n端口: notaport\n";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(sessions.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Invalid port"));
    }

    #[test]
    fn comments_and_unknown_lines_are_ignored() {
        let text = "# comment\n// also comment\nXTerminal 导出文件\n名称: ok\n地址: 5.5.5.5\n";
        let (sessions, errors) = parse_xterminal_text(text);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].addr, "5.5.5.5");
    }

    #[test]
    fn name_falls_back_to_addr() {
        let text = "地址: 6.6.6.6\n用户: root\n";
        let (sessions, _) = parse_xterminal_text(text);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].server_name, "6.6.6.6");
    }
}
