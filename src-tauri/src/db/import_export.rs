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
