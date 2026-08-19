use rusqlite::types::ToSql;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use uuid::Uuid;

pub mod import_export;

/// Platform-specific app data directory path for the SQLite database.
static DB_PATH: LazyLock<Result<PathBuf, String>> = LazyLock::new(|| {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?
        .join("NexaShell");
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok(data_dir.join("nexashell.db"))
});

fn db_path() -> Result<&'static PathBuf, String> {
    DB_PATH.as_ref().map_err(|e| e.clone())
}

/// Singleton connection shared across all DB operations.
static DB: LazyLock<Mutex<Option<Connection>>> = LazyLock::new(|| Mutex::new(None));

pub(super) fn with_db<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&Connection) -> Result<T, String>,
{
    let guard = DB.lock().map_err(|e| format!("DB lock poisoned: {}", e))?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB not initialized".to_string())?;
    f(conn)
}

pub(super) fn db_conn() -> Result<std::sync::MutexGuard<'static, Option<Connection>>, String> {
    DB.lock().map_err(|e| format!("DB lock poisoned: {}", e))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub addr: String,
    pub port: i64,
    pub server_name: String,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: Option<String>,
    pub is_favorite: bool,
    pub last_connected_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub sort: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub sort: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExportSession {
    pub metadata: Session,
    pub encrypted_credentials: Option<String>,
    pub group_ids: Vec<String>,
    pub tag_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionWithRelations {
    #[serde(flatten)]
    pub session: Session,
    pub group_ids: Vec<String>,
    pub groups: Vec<String>,
    pub tag_ids: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportData {
    pub sessions: Vec<ExportSession>,
    pub groups: Vec<Group>,
    pub tags: Vec<Tag>,
}

fn add_column_if_not_exists(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
            params![table, column],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if count == 0 {
        if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            || !column
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(format!(
                "Invalid table/column identifier: {}.{}",
                table, column
            ));
        }
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A single dynamic column assignment in an UPDATE statement.
struct SetClause<'a> {
    column: &'a str,
    value: Box<dyn ToSql>,
}

/// Build the `SET ... WHERE ...` SQL and positional `?` parameter list from
/// optional fields, so the "edit" commands (`save_session_with_credentials`,
/// `edit_group`, `edit_tag`, `edit_session`) don't each hand-roll the fragile
/// `Vec<Box<dyn ToSql>>` + string-concatenation boilerplate.
///
/// Column/table names are caller-controlled (never user input), but are still
/// validated against `[A-Za-z0-9_]` as defense in depth.
fn build_update(
    table: &str,
    sets: Vec<SetClause<'_>>,
    where_column: &str,
    where_value: Box<dyn ToSql>,
) -> Result<(String, Vec<Box<dyn ToSql>>), String> {
    let ident_ok = |s: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');

    if !ident_ok(table) || !ident_ok(where_column) {
        return Err(format!(
            "Invalid identifier in UPDATE: {}.{}",
            table, where_column
        ));
    }
    for set in &sets {
        if !ident_ok(set.column) {
            return Err(format!("Invalid column identifier: {}", set.column));
        }
    }

    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::with_capacity(sets.len() + 1);
    let mut assignments = Vec::with_capacity(sets.len());
    for set in sets {
        assignments.push(format!("{} = ?", set.column));
        params_vec.push(set.value);
    }
    // `updated_at` is always bumped on edit.
    assignments.push("updated_at = CURRENT_TIMESTAMP".to_string());

    let sql = format!(
        "UPDATE {} SET {} WHERE {} = ?",
        table,
        assignments.join(", "),
        where_column
    );
    params_vec.push(where_value);

    Ok((sql, params_vec))
}

fn exec_update(conn: &Connection, sql: &str, params_vec: &[Box<dyn ToSql>]) -> Result<(), String> {
    let p: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(sql, p.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

fn ensure_groups_and_tags(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT 'Default Group',
            sort INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            color TEXT,
            sort INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    // The `color` column is required by list_tags/list_tags_for_session; a
    // silent failure here would leave those SELECTs failing at runtime with a
    // confusing "no such column" error. Propagate instead.
    add_column_if_not_exists(conn, "tags", "color", "TEXT")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS session_groups (
            session_id TEXT NOT NULL,
            group_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE(session_id, group_id)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS session_tags (
            session_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE(session_id, tag_id)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn init_db() -> Result<String, String> {
    let db_path = db_path()?;
    let existed = db_path.exists();
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;",
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            addr TEXT NOT NULL,
            port INTEGER NOT NULL CHECK (port >= 0 AND port <= 65535),
            server_name TEXT NOT NULL,
            username TEXT NOT NULL,
            auth_type TEXT NOT NULL,
            private_key_path TEXT,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            last_connected_at TEXT,
            encrypted_credentials TEXT,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    add_column_if_not_exists(
        &conn,
        "sessions",
        "is_favorite",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_not_exists(&conn, "sessions", "encrypted_credentials", "TEXT")?;
    // Backfill last_connected_at ONLY when we just introduced the column
    // (first migration to a schema that has it). Re-running this backfill on
    // every startup would stamp every "never connected" session with
    // updated_at and fabricate a (misleading) connection history.
    let had_last_connected = table_has_column(&conn, "sessions", "last_connected_at")?;
    add_column_if_not_exists(&conn, "sessions", "last_connected_at", "TEXT")?;
    if !had_last_connected {
        let _ = conn.execute(
            "UPDATE sessions SET last_connected_at = updated_at WHERE last_connected_at IS NULL",
            [],
        );
    }

    ensure_groups_and_tags(&conn)?;

    // SSH port-forwarding rules persisted per session.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tunnel_rules (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            direction TEXT NOT NULL CHECK (direction IN ('local', 'dynamic')),
            listen_host TEXT NOT NULL DEFAULT '127.0.0.1',
            listen_port INTEGER NOT NULL CHECK (listen_port >= 0 AND listen_port <= 65535),
            target_host TEXT NOT NULL DEFAULT '',
            target_port INTEGER NOT NULL DEFAULT 0 CHECK (target_port >= 0 AND target_port <= 65535),
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tunnel_rules_session ON tunnel_rules(session_id)",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Reusable command snippets (command-palette / snippet library).
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snippets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            command TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            sort INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_snippets_sort ON snippets(sort)",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_addr ON sessions(addr)",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_server_name ON sessions(server_name)",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_groups_group_id ON session_groups(group_id)",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_groups_session_id ON session_groups(session_id)",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_tags_tag_id ON session_tags(tag_id)",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_tags_session_id ON session_tags(session_id)",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Enforce real foreign keys ON DELETE CASCADE on the relation tables so
    // deleted sessions can never orphan tunnel_rules (and imports cannot leave
    // dangling links). Runs after every related table exists, only for schemas
    // that still lack the constraints.
    ensure_relation_fks(&mut conn)?;

    {
        let mut guard = DB.lock().map_err(|e| format!("DB lock poisoned: {}", e))?;
        *guard = Some(conn);
    }

    if !existed {
        Ok("created".into())
    } else {
        Ok("ok".into())
    }
}

/// Rebuild a relation table with proper FOREIGN KEY ... ON DELETE CASCADE
/// constraints. The tables were originally created without FKs (referential
/// integrity was hand-maintained in the delete_* commands), which let
/// delete_session orphan tunnel_rules and allowed imports to create dangling
/// links. Implements the 12-step table-rebuild migration inside a transaction.
fn rebuild_table_with_fk(conn: &mut Connection, table: &str, create_sql: &str) -> Result<(), String> {
    if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("Invalid table identifier: {}", table));
    }
    // Skip when FK constraints are already present.
    let fk_count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM pragma_foreign_key_list('{}')", table),
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if fk_count > 0 {
        return Ok(());
    }

    let tmp = format!("{}_new", table);
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(&format!("DROP TABLE IF EXISTS {}", tmp), [])
            .map_err(|e| e.to_string())?;
        tx.execute(create_sql, []).map_err(|e| e.to_string())?;
        tx.execute(&format!("INSERT INTO {} SELECT * FROM {}", tmp, table), [])
            .map_err(|e| e.to_string())?;
        tx.execute(&format!("DROP TABLE {}", table), [])
            .map_err(|e| e.to_string())?;
        tx.execute(&format!("ALTER TABLE {} RENAME TO {}", tmp, table), [])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");
    result?;

    // Verify referential integrity of the migrated data.
    let bad: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if bad > 0 {
        return Err(format!(
            "Foreign key check failed for {} ({} dangling references)",
            table, bad
        ));
    }
    Ok(())
}

fn ensure_relation_fks(conn: &mut Connection) -> Result<(), String> {
    rebuild_table_with_fk(
        conn,
        "session_groups",
        "CREATE TABLE session_groups_new (
            session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE(session_id, group_id)
        )",
    )?;
    rebuild_table_with_fk(
        conn,
        "session_tags",
        "CREATE TABLE session_tags_new (
            session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE(session_id, tag_id)
        )",
    )?;
    rebuild_table_with_fk(
        conn,
        "tunnel_rules",
        "CREATE TABLE tunnel_rules_new (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            direction TEXT NOT NULL CHECK (direction IN ('local', 'dynamic')),
            listen_host TEXT NOT NULL DEFAULT '127.0.0.1',
            listen_port INTEGER NOT NULL CHECK (listen_port >= 0 AND listen_port <= 65535),
            target_host TEXT NOT NULL DEFAULT '',
            target_port INTEGER NOT NULL DEFAULT 0 CHECK (target_port >= 0 AND target_port <= 65535),
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !column.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!(
            "Invalid table/column identifier: {}.{}",
            table, column
        ));
    }
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
            params![table, column],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

/// Escape `%`, `_` and `\` so user search terms are matched literally in a
/// `LIKE ... ESCAPE '\'` pattern instead of acting as wildcards.
fn escape_like(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch == '\\' || ch == '%' || ch == '_' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub(super) fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        addr: row.get(1)?,
        port: row.get(2)?,
        server_name: row.get(3)?,
        username: row.get(4)?,
        auth_type: row.get(5)?,
        private_key_path: row.get(6)?,
        is_favorite: row.get::<_, i64>(7)? != 0,
        last_connected_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(super) const SESSION_COLUMNS: &str = "id, addr, port, server_name, username, auth_type, private_key_path, is_favorite, last_connected_at, created_at, updated_at";

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_session_with_credentials(
    id: Option<String>,
    addr: String,
    port: i64,
    server_name: String,
    username: String,
    auth_type: String,
    private_key_path: Option<String>,
    password: Option<String>,
    key_passphrase: Option<String>,
    is_favorite: Option<bool>,
    group_ids: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
    clear_credentials: bool,
) -> Result<String, String> {
    let is_update = id.is_some();
    let session_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

    // `None` means "unchanged" (preserve any stored ciphertext), while an
    // explicit clear request writes NULL so the stored secret is removed.
    let credential_value_to_write: Option<String> = if clear_credentials {
        None
    } else if password.is_some() || key_passphrase.is_some() {
        let sensitive = crate::encryption::SensitiveData {
            password,
            key_passphrase,
        };
        Some(crate::encryption::EncryptionManager::encrypt(&sensitive)?)
    } else {
        None
    };

    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    if is_update {
        // Only overwrite encrypted_credentials when a new password/passphrase
        // was provided. Otherwise a save where the caller omitted the fields
        // (e.g. credential fetch failed) would silently wipe stored secrets.
        let mut sets = vec![
            SetClause {
                column: "addr",
                value: Box::new(addr),
            },
            SetClause {
                column: "port",
                value: Box::new(port),
            },
            SetClause {
                column: "server_name",
                value: Box::new(server_name),
            },
            SetClause {
                column: "username",
                value: Box::new(username),
            },
            SetClause {
                column: "auth_type",
                value: Box::new(auth_type),
            },
            SetClause {
                column: "private_key_path",
                value: Box::new(private_key_path),
            },
        ];
        if clear_credentials {
            // Explicit request to remove stored credentials: write NULL so the
            // ciphertext is dropped (not treated as "unchanged").
            sets.push(SetClause {
                column: "encrypted_credentials",
                value: Box::new(None::<String>),
            });
        } else if let Some(enc) = credential_value_to_write {
            sets.push(SetClause {
                column: "encrypted_credentials",
                value: Box::new(enc),
            });
        }
        if let Some(fav) = is_favorite {
            sets.push(SetClause {
                column: "is_favorite",
                value: Box::new(if fav { 1 } else { 0 }),
            });
        }

        let (sql, params) = build_update("sessions", sets, "id", Box::new(session_id.clone()))?;
        let p: Vec<&dyn ToSql> = params.iter().map(|b| &**b as &dyn ToSql).collect();
        tx.execute(&sql, p.as_slice()).map_err(|e| e.to_string())?;

        // Only re-sync relations when the caller supplied them. Treating `None`
        // as "unchanged" mirrors the credential guard above — otherwise an
        // update that omits group_ids/tag_ids would silently wipe every
        // group/tag link the session has (exactly the symmetric data-loss the
        // credential guard at the top of this branch protects against).
        if group_ids.is_some() {
            tx.execute(
                "DELETE FROM session_groups WHERE session_id = ?1",
                params![session_id],
            )
            .map_err(|e| e.to_string())?;
        }
        if tag_ids.is_some() {
            tx.execute(
                "DELETE FROM session_tags WHERE session_id = ?1",
                params![session_id],
            )
            .map_err(|e| e.to_string())?;
        }
    } else {
        tx.execute(
            "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite, encrypted_credentials)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session_id,
                addr,
                port,
                server_name,
                username,
                auth_type,
                private_key_path,
                if is_favorite.unwrap_or(false) { 1 } else { 0 },
                credential_value_to_write
            ],
        ).map_err(|e| e.to_string())?;
    }

    if let Some(groups) = group_ids {
        for group_id in groups {
            tx.execute(
                "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
                params![session_id, group_id],
            )
            .map_err(|e| format!("Failed to link group: {}", e))?;
        }
    }

    if let Some(tags) = tag_ids {
        for tag_id in tags {
            tx.execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![session_id, tag_id],
            )
            .map_err(|e| format!("Failed to link tag: {}", e))?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(session_id)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn get_session_credentials(
    sessionId: String,
) -> Result<(String, Option<String>, Option<String>), String> {
    let encrypted: Option<String> = with_db(|conn| {
        conn.query_row(
            "SELECT encrypted_credentials FROM sessions WHERE id = ?1",
            params![sessionId],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    })?;

    if let Some(enc) = encrypted {
        let creds = crate::encryption::EncryptionManager::decrypt(&enc)?;
        Ok((
            sessionId,
            creds.password.clone(),
            creds.key_passphrase.clone(),
        ))
    } else {
        Ok((sessionId, None, None))
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_session(
    addr: String,
    port: i64,
    server_name: String,
    username: String,
    auth_type: String,
    private_key_path: Option<String>,
    is_favorite: Option<bool>,
    group_ids: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            addr,
            port,
            server_name,
            username,
            auth_type,
            private_key_path,
            if is_favorite.unwrap_or(false) { 1 } else { 0 }
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(groups) = group_ids {
        for gid in groups {
            tx.execute(
                "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
                params![id, gid],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    if let Some(tags) = tag_ids {
        for tid in tags {
            tx.execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![id, tid],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn toggle_favorite(id: String, is_favorite: bool) -> Result<(), String> {
    with_db(|conn| {
        conn.execute(
            "UPDATE sessions SET is_favorite = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![if is_favorite { 1 } else { 0 }, id],
        )
        .map_err(|e| e.to_string())
    })?;
    Ok(())
}

#[tauri::command]
pub fn update_session_timestamp(id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute(
            "UPDATE sessions SET last_connected_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())
    })?;
    Ok(())
}

#[tauri::command]
pub fn list_sessions() -> Result<Vec<Session>, String> {
    with_db(|conn| {
        let sql = format!("SELECT {} FROM sessions", SESSION_COLUMNS);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], row_to_session)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn get_sessions(
    group_id: Option<String>,
    tag_id: Option<String>,
    id: Option<String>,
    server_name: Option<String>,
    host_addr: Option<String>,
) -> Result<Vec<Session>, String> {
    let mut sql = format!(
        "SELECT DISTINCT s.{} FROM sessions s",
        SESSION_COLUMNS.replace(", ", ", s.")
    );
    if group_id.is_some() {
        sql.push_str(" JOIN session_groups sg ON s.id = sg.session_id");
    }
    if tag_id.is_some() {
        sql.push_str(" JOIN session_tags st ON s.id = st.session_id");
    }

    let mut wheres = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(gid) = group_id {
        wheres.push("sg.group_id = ?".to_string());
        params_vec.push(Box::new(gid));
    }
    if let Some(tid) = tag_id {
        wheres.push("st.tag_id = ?".to_string());
        params_vec.push(Box::new(tid));
    }
    if let Some(pid) = id {
        wheres.push("s.id = ?".to_string());
        params_vec.push(Box::new(pid));
    }
    if let Some(n) = server_name {
        wheres.push("s.server_name LIKE ? ESCAPE '\\'".to_string());
        params_vec.push(Box::new(format!("%{}%", escape_like(&n))));
    }
    if let Some(a) = host_addr {
        wheres.push("s.addr LIKE ? ESCAPE '\\'".to_string());
        params_vec.push(Box::new(format!("%{}%", escape_like(&a))));
    }

    if !wheres.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&wheres.join(" AND "));
    }

    let guard = db_conn()?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let p: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    let rows = stmt
        .query_map(p.as_slice(), row_to_session)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sessions_with_relations() -> Result<Vec<SessionWithRelations>, String> {
    with_db(|conn| {
        let sql = format!("SELECT {} FROM sessions", SESSION_COLUMNS);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], row_to_session)
            .map_err(|e| e.to_string())?;
        let mut sessions = Vec::new();
        for r in rows {
            sessions.push(r.map_err(|e| e.to_string())?);
        }
        drop(stmt);

        if sessions.is_empty() {
            return Ok(Vec::new());
        }

        let mut group_map: std::collections::HashMap<String, (Vec<String>, Vec<String>)> =
            std::collections::HashMap::new();
        let mut gstmt = conn
            .prepare(
                "SELECT sg.session_id, g.id, g.name FROM session_groups sg JOIN groups g ON g.id = sg.group_id",
            )
            .map_err(|e| e.to_string())?;
        let grows = gstmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for r in grows {
            let (sid, gid, gname) = r.map_err(|e| e.to_string())?;
            let entry = group_map.entry(sid).or_default();
            entry.0.push(gid);
            entry.1.push(gname);
        }
        drop(gstmt);

        let mut tag_map: std::collections::HashMap<String, (Vec<String>, Vec<String>)> =
            std::collections::HashMap::new();
        let mut tstmt = conn
            .prepare(
                "SELECT st.session_id, t.id, t.name FROM session_tags st JOIN tags t ON t.id = st.tag_id",
            )
            .map_err(|e| e.to_string())?;
        let trows = tstmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for r in trows {
            let (sid, tid, tname) = r.map_err(|e| e.to_string())?;
            let entry = tag_map.entry(sid).or_default();
            entry.0.push(tid);
            entry.1.push(tname);
        }

        Ok(sessions
            .into_iter()
            .map(|s| {
                let (gids, gnames) = group_map.remove(&s.id).unwrap_or_default();
                let (tids, tnames) = tag_map.remove(&s.id).unwrap_or_default();
                SessionWithRelations {
                    session: s,
                    group_ids: gids,
                    groups: gnames,
                    tag_ids: tids,
                    tags: tnames,
                }
            })
            .collect())
    })
}

#[tauri::command]
pub fn edit_group(id: String, name: Option<String>, sort: Option<i64>) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let mut sets = Vec::new();
    if let Some(n) = name {
        sets.push(SetClause {
            column: "name",
            value: Box::new(n),
        });
    }
    if let Some(s) = sort {
        sets.push(SetClause {
            column: "sort",
            value: Box::new(s),
        });
    }
    if sets.is_empty() {
        return Ok(());
    }
    let (sql, params) = build_update("groups", sets, "id", Box::new(id))?;
    exec_update(conn, &sql, &params)
}

#[tauri::command]
pub fn delete_group(id: String) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM session_groups WHERE group_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM groups WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn edit_tag(
    id: String,
    name: Option<String>,
    color: Option<String>,
    sort: Option<i64>,
) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let mut sets = Vec::new();
    if let Some(n) = name {
        sets.push(SetClause {
            column: "name",
            value: Box::new(n),
        });
    }
    if let Some(c) = color {
        sets.push(SetClause {
            column: "color",
            value: Box::new(c),
        });
    }
    if let Some(s) = sort {
        sets.push(SetClause {
            column: "sort",
            value: Box::new(s),
        });
    }
    if sets.is_empty() {
        return Ok(());
    }
    let (sql, params) = build_update("tags", sets, "id", Box::new(id))?;
    exec_update(conn, &sql, &params)
}

#[tauri::command]
pub fn delete_tag(id: String) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM session_tags WHERE tag_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM tags WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn edit_session(
    id: String,
    addr: Option<String>,
    port: Option<i64>,
    server_name: Option<String>,
    username: Option<String>,
    auth_type: Option<String>,
    private_key_path: Option<Option<String>>,
    is_favorite: Option<bool>,
) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let mut sets = Vec::new();
    if let Some(a) = addr {
        sets.push(SetClause {
            column: "addr",
            value: Box::new(a),
        });
    }
    if let Some(p) = port {
        sets.push(SetClause {
            column: "port",
            value: Box::new(p),
        });
    }
    if let Some(s) = server_name {
        sets.push(SetClause {
            column: "server_name",
            value: Box::new(s),
        });
    }
    if let Some(u) = username {
        sets.push(SetClause {
            column: "username",
            value: Box::new(u),
        });
    }
    if let Some(at) = auth_type {
        sets.push(SetClause {
            column: "auth_type",
            value: Box::new(at),
        });
    }
    if let Some(pk) = private_key_path {
        sets.push(SetClause {
            column: "private_key_path",
            value: Box::new(pk),
        });
    }
    if let Some(fav) = is_favorite {
        sets.push(SetClause {
            column: "is_favorite",
            value: Box::new(if fav { 1 } else { 0 }),
        });
    }
    if sets.is_empty() {
        return Ok(());
    }
    let (sql, params) = build_update("sessions", sets, "id", Box::new(id))?;
    exec_update(conn, &sql, &params)
}

#[tauri::command]
pub fn delete_session(id: String) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM session_groups WHERE session_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM session_tags WHERE session_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    // Also remove this session's port-forwarding rules. (With the FK migration
    // in place this also cascades automatically, but an explicit delete keeps
    // pre-migration schemas correct and is harmless either way.)
    tx.execute(
        "DELETE FROM tunnel_rules WHERE session_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM sessions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    log::debug!("Deleted session {}", id);
    Ok(())
}

#[tauri::command]
pub fn add_group(name: Option<String>, sort: Option<i64>) -> Result<String, String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_else(|| "Default Group".to_string());
    let sort = sort.unwrap_or(1);
    conn.execute(
        "INSERT INTO groups (id, name, sort) VALUES (?1, ?2, ?3)",
        params![id, name, sort],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn list_groups() -> Result<Vec<Group>, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, sort, created_at, updated_at FROM groups ORDER BY sort, created_at")
            .map_err(|e| e.to_string())?;
        let rows = stmt
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
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn link_session_group(session_id: String, group_id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
            params![session_id, group_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command]
pub fn unlink_session_group(session_id: String, group_id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute(
            "DELETE FROM session_groups WHERE session_id = ?1 AND group_id = ?2",
            params![session_id, group_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command]
pub fn list_groups_for_session(session_id: String) -> Result<Vec<Group>, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT g.id, g.name, g.sort, g.created_at, g.updated_at
                 FROM groups g
                 JOIN session_groups sg ON g.id = sg.group_id
                 WHERE sg.session_id = ?1
                 ORDER BY g.sort, g.created_at",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |row| {
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
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn add_tag(
    name: Option<String>,
    color: Option<String>,
    sort: Option<i64>,
) -> Result<String, String> {
    let mut guard = db_conn()?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| "DB not initialized".to_string())?;
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_default();
    let sort = sort.unwrap_or(1);
    conn.execute(
        "INSERT INTO tags (id, name, color, sort) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, color, sort],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn list_tags() -> Result<Vec<Tag>, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, color, sort, created_at, updated_at FROM tags ORDER BY sort, created_at")
            .map_err(|e| e.to_string())?;
        let rows = stmt
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
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn link_session_tag(session_id: String, tag_id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
            params![session_id, tag_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command]
pub fn unlink_session_tag(session_id: String, tag_id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute(
            "DELETE FROM session_tags WHERE session_id = ?1 AND tag_id = ?2",
            params![session_id, tag_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command]
pub fn list_tags_for_session(session_id: String) -> Result<Vec<Tag>, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.name, t.color, t.sort, t.created_at, t.updated_at
                 FROM tags t
                 JOIN session_tags st ON t.id = st.tag_id
                 WHERE st.session_id = ?1
                 ORDER BY t.sort, t.created_at",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |row| {
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
            .map_err(|e| e.to_string())
    })
}

// ----------------------------------------------------------------------------
// Tunnel rules (SSH port forwarding) persistence
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TunnelRuleRow {
    pub id: String,
    pub session_id: String,
    pub direction: String, // "local" | "dynamic"
    pub listen_host: String,
    pub listen_port: i64,
    pub target_host: String,
    pub target_port: i64,
    pub enabled: bool,
}

#[tauri::command]
pub fn add_tunnel_rule(
    session_id: String,
    direction: String,
    listen_host: String,
    listen_port: i64,
    target_host: String,
    target_port: i64,
    enabled: bool,
) -> Result<String, String> {
    if direction != "local" && direction != "dynamic" {
        return Err("direction must be 'local' or 'dynamic'".to_string());
    }
    let id = Uuid::new_v4().to_string();
    with_db(|conn| {
        conn.execute(
            "INSERT INTO tunnel_rules
                (id, session_id, direction, listen_host, listen_port, target_host, target_port, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                session_id,
                direction,
                listen_host,
                listen_port,
                target_host,
                target_port,
                enabled as i64
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(id)
    })
}

#[tauri::command]
pub fn list_tunnel_rules(session_id: String) -> Result<Vec<TunnelRuleRow>, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, direction, listen_host, listen_port, target_host, target_port, enabled
                 FROM tunnel_rules WHERE session_id = ?1 ORDER BY created_at, id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(TunnelRuleRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    direction: row.get(2)?,
                    listen_host: row.get(3)?,
                    listen_port: row.get(4)?,
                    target_host: row.get(5)?,
                    target_port: row.get(6)?,
                    enabled: row.get::<_, i64>(7)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn update_tunnel_rule(
    id: String,
    direction: Option<String>,
    listen_host: Option<String>,
    listen_port: Option<i64>,
    target_host: Option<String>,
    target_port: Option<i64>,
    enabled: Option<bool>,
) -> Result<(), String> {
    if let Some(ref d) = direction
        && d != "local"
        && d != "dynamic"
    {
        return Err("direction must be 'local' or 'dynamic'".to_string());
    }
    let sets: Vec<SetClause<'_>> = [
        direction.map(|v| SetClause { column: "direction", value: Box::new(v) }),
        listen_host.map(|v| SetClause { column: "listen_host", value: Box::new(v) }),
        listen_port.map(|v| SetClause { column: "listen_port", value: Box::new(v) }),
        target_host.map(|v| SetClause { column: "target_host", value: Box::new(v) }),
        target_port.map(|v| SetClause { column: "target_port", value: Box::new(v) }),
        enabled.map(|v| SetClause { column: "enabled", value: Box::new(v as i64) }),
    ]
    .into_iter()
    .flatten()
    .collect();
    // No-op edit should not bump updated_at (matches edit_group/edit_tag).
    if sets.is_empty() {
        return Ok(());
    }
    let (sql, params_vec) =
        build_update("tunnel_rules", sets, "id", Box::new(id))?;
    with_db(|conn| exec_update(conn, &sql, &params_vec))
}

#[tauri::command]
pub fn delete_tunnel_rule(id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute("DELETE FROM tunnel_rules WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command]
pub fn delete_tunnel_rules_for_session(session_id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute("DELETE FROM tunnel_rules WHERE session_id = ?1", params![session_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

// ----------------------------------------------------------------------------
// Snippets persistence
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: String,
    pub sort: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub fn add_snippet(name: String, command: String, description: Option<String>) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    with_db(|conn| {
        let sort: i64 = conn
            .query_row("SELECT COALESCE(MAX(sort), 0) + 1 FROM snippets", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO snippets (id, name, command, description, sort) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, &name, &command, description.unwrap_or_default(), sort],
        )
        .map_err(|e| e.to_string())?;
        Ok(id)
    })
}

#[tauri::command]
pub fn list_snippets() -> Result<Vec<Snippet>, String> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, command, description, sort, created_at, updated_at
                 FROM snippets ORDER BY sort, created_at",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Snippet {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    sort: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn update_snippet(
    id: String,
    name: Option<String>,
    command: Option<String>,
    description: Option<String>,
    sort: Option<i64>,
) -> Result<(), String> {
    let sets: Vec<SetClause<'_>> = [
        name.map(|v| SetClause { column: "name", value: Box::new(v) }),
        command.map(|v| SetClause { column: "command", value: Box::new(v) }),
        description.map(|v| SetClause { column: "description", value: Box::new(v) }),
        sort.map(|v| SetClause { column: "sort", value: Box::new(v) }),
    ]
    .into_iter()
    .flatten()
    .collect();
    // No-op edit should not bump updated_at (matches edit_group/edit_tag).
    if sets.is_empty() {
        return Ok(());
    }
    let (sql, params_vec) = build_update("snippets", sets, "id", Box::new(id))?;
    with_db(|conn| exec_update(conn, &sql, &params_vec))
}

#[tauri::command]
pub fn delete_snippet(id: String) -> Result<(), String> {
    with_db(|conn| {
        conn.execute("DELETE FROM snippets WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_update_generates_correct_sql_and_params() {
        let sets = vec![
            SetClause {
                column: "name",
                value: Box::new("prod db".to_string()),
            },
            SetClause {
                column: "port",
                value: Box::new(22i64),
            },
        ];
        let (sql, params) = build_update("sessions", sets, "id", Box::new("abc".to_string()))
            .expect("build_update should succeed");

        assert_eq!(
            sql,
            "UPDATE sessions SET name = ?, port = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        );
        assert_eq!(params.len(), 3, "two set columns + one where value");
    }

    #[test]
    fn build_update_always_bumps_updated_at() {
        let (sql, _) = build_update("groups", Vec::new(), "id", Box::new("g1".to_string()))
            .expect("build_update should succeed");
        assert!(sql.contains("updated_at = CURRENT_TIMESTAMP"));
    }

    #[test]
    fn build_update_rejects_invalid_identifiers() {
        let sets = vec![SetClause {
            column: "name; DROP TABLE sessions",
            value: Box::new(1i64),
        }];
        let res = build_update("sessions", sets, "id", Box::new("x".to_string()));
        assert!(res.is_err(), "injection attempt must be rejected");
    }
}
