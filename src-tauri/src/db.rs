use once_cell::sync::Lazy;
use rusqlite::types::ToSql;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

/// Platform-specific app data directory path for the SQLite database.
static DB_PATH: Lazy<Result<PathBuf, String>> = Lazy::new(|| {
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
static DB: Lazy<Mutex<Option<Connection>>> = Lazy::new(|| Mutex::new(None));

fn with_db<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&Connection) -> Result<T, String>,
{
    let guard = DB.lock().map_err(|e| format!("DB lock poisoned: {}", e))?;
    let conn = guard.as_ref().ok_or_else(|| "DB not initialized".to_string())?;
    f(conn)
}

#[allow(dead_code)]
fn with_db_mut<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&mut Connection) -> Result<T, String>,
{
    let mut guard = DB.lock().map_err(|e| format!("DB lock poisoned: {}", e))?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    f(conn)
}

fn db_conn() -> Result<std::sync::MutexGuard<'static, Option<Connection>>, String> {
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
        if !table
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
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

    let _ = add_column_if_not_exists(conn, "tags", "color", "TEXT");

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
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

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
    ).map_err(|e| e.to_string())?;

    add_column_if_not_exists(&conn, "sessions", "is_favorite", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_not_exists(&conn, "sessions", "encrypted_credentials", "TEXT")?;
    add_column_if_not_exists(&conn, "sessions", "last_connected_at", "TEXT")?;

    let _ = conn.execute(
        "UPDATE sessions SET last_connected_at = updated_at WHERE last_connected_at IS NULL",
        [],
    );

    ensure_groups_and_tags(&conn)?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_sessions_addr ON sessions(addr)", []).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_server_name ON sessions(server_name)",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_groups_group_id ON session_groups(group_id)",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_groups_session_id ON session_groups(session_id)",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_tags_tag_id ON session_tags(tag_id)",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_session_tags_session_id ON session_tags(session_id)",
        [],
    ).map_err(|e| e.to_string())?;

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

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
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

const SESSION_COLUMNS: &str =
    "id, addr, port, server_name, username, auth_type, private_key_path, is_favorite, last_connected_at, created_at, updated_at";

#[tauri::command]
pub fn add_session(
    addr: String,
    port: i64,
    server_name: String,
    username: String,
    auth_type: String,
    private_key_path: Option<String>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    with_db(|conn| {
        conn.execute(
            "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![id, addr, port, server_name, username, auth_type, private_key_path],
        ).map_err(|e| e.to_string())
    })?;
    Ok(id)
}

#[tauri::command]
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
) -> Result<String, String> {
    let is_update = id.is_some();
    let session_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let encrypted_credentials = if password.is_some() || key_passphrase.is_some() {
        let sensitive = crate::encryption::SensitiveData {
            password,
            key_passphrase,
        };
        Some(crate::encryption::EncryptionManager::encrypt(&sensitive)?)
    } else {
        None
    };

    let mut guard = db_conn()?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    if is_update {
        let mut sql = "UPDATE sessions SET addr = ?1, port = ?2, server_name = ?3, username = ?4, auth_type = ?5, private_key_path = ?6, encrypted_credentials = ?7, updated_at = CURRENT_TIMESTAMP".to_string();
        let mut params_vec: Vec<Box<dyn ToSql>> = vec![
            Box::new(addr),
            Box::new(port),
            Box::new(server_name),
            Box::new(username),
            Box::new(auth_type),
            Box::new(private_key_path),
            Box::new(encrypted_credentials),
        ];

        if let Some(fav) = is_favorite {
            sql.push_str(", is_favorite = ?");
            sql.push_str(&(params_vec.len() + 1).to_string());
            params_vec.push(Box::new(if fav { 1 } else { 0 }));
        }

        sql.push_str(" WHERE id = ?");
        sql.push_str(&(params_vec.len() + 1).to_string());
        params_vec.push(Box::new(session_id.clone()));

        let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
        tx.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;

        tx.execute(
            "DELETE FROM session_groups WHERE session_id = ?1",
            params![session_id],
        ).map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM session_tags WHERE session_id = ?1",
            params![session_id],
        ).map_err(|e| e.to_string())?;
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
                encrypted_credentials
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
#[allow(dead_code)]
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
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
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
        wheres.push("s.server_name LIKE ?".to_string());
        params_vec.push(Box::new(format!("%{}%", n)));
    }
    if let Some(a) = host_addr {
        wheres.push("s.addr LIKE ?".to_string());
        params_vec.push(Box::new(format!("%{}%", a)));
    }

    if !wheres.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&wheres.join(" AND "));
    }

    let guard = db_conn()?;
    let conn = guard.as_ref().ok_or_else(|| "DB not initialized".to_string())?;
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let p: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    let rows = stmt.query_map(p.as_slice(), row_to_session).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
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
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let mut sets = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(n) = name {
        sets.push("name = ?");
        params_vec.push(Box::new(n));
    }
    if let Some(s) = sort {
        sets.push("sort = ?");
        params_vec.push(Box::new(s));
    }
    if sets.is_empty() {
        return Ok(());
    }
    sets.push("updated_at = CURRENT_TIMESTAMP");
    let sql = format!("UPDATE groups SET {} WHERE id = ?", sets.join(", "));
    params_vec.push(Box::new(id));
    let p: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(&sql, p.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_group(id: String) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM session_groups WHERE group_id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM groups WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
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
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let mut sets = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(n) = name {
        sets.push("name = ?");
        params_vec.push(Box::new(n));
    }
    if let Some(c) = color {
        sets.push("color = ?");
        params_vec.push(Box::new(c));
    }
    if let Some(s) = sort {
        sets.push("sort = ?");
        params_vec.push(Box::new(s));
    }
    if sets.is_empty() {
        return Ok(());
    }
    sets.push("updated_at = CURRENT_TIMESTAMP");
    let sql = format!("UPDATE tags SET {} WHERE id = ?", sets.join(", "));
    params_vec.push(Box::new(id));
    let p: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(&sql, p.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_tag(id: String) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM session_tags WHERE tag_id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM tags WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
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
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let mut sets = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(a) = addr {
        sets.push("addr = ?");
        params_vec.push(Box::new(a));
    }
    if let Some(p) = port {
        sets.push("port = ?");
        params_vec.push(Box::new(p));
    }
    if let Some(s) = server_name {
        sets.push("server_name = ?");
        params_vec.push(Box::new(s));
    }
    if let Some(u) = username {
        sets.push("username = ?");
        params_vec.push(Box::new(u));
    }
    if let Some(at) = auth_type {
        sets.push("auth_type = ?");
        params_vec.push(Box::new(at));
    }
    if let Some(pk) = private_key_path {
        sets.push("private_key_path = ?");
        params_vec.push(Box::new(pk));
    }
    if let Some(fav) = is_favorite {
        sets.push("is_favorite = ?");
        params_vec.push(Box::new(if fav { 1 } else { 0 }));
    }
    if sets.is_empty() {
        return Ok(());
    }
    sets.push("updated_at = CURRENT_TIMESTAMP");
    let sql = format!("UPDATE sessions SET {} WHERE id = ?", sets.join(", "));
    params_vec.push(Box::new(id));
    let p: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(&sql, p.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_session(id: String) -> Result<(), String> {
    let mut guard = db_conn()?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM session_groups WHERE session_id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM session_tags WHERE session_id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM sessions WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    log::debug!("Deleted session {}", id);
    Ok(())
}

#[tauri::command]
pub fn add_group(name: Option<String>, sort: Option<i64>) -> Result<String, String> {
    let mut guard = db_conn()?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_else(|| "Default Group".to_string());
    let sort = sort.unwrap_or(1);
    conn.execute(
        "INSERT INTO groups (id, name, sort) VALUES (?1, ?2, ?3)",
        params![id, name, sort],
    ).map_err(|e| e.to_string())?;
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
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
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
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn add_tag(
    name: Option<String>,
    color: Option<String>,
    sort: Option<i64>,
) -> Result<String, String> {
    let mut guard = db_conn()?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_default();
    let sort = sort.unwrap_or(1);
    conn.execute(
        "INSERT INTO tags (id, name, color, sort) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, color, sort],
    ).map_err(|e| e.to_string())?;
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
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
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
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    })
}

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
            .prepare("SELECT session_id, group_id FROM session_groups").map_err(|e| e.to_string())?;
        let rows = g.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;
        for r in rows {
            let (sid, gid): (String, String) = r.map_err(|e| e.to_string())?;
            group_map
                .entry(sid)
                .or_insert_with(Vec::new)
                .push(gid);
        }
        drop(g);

        let mut tag_map = std::collections::HashMap::new();
        let mut t = conn
            .prepare("SELECT session_id, tag_id FROM session_tags").map_err(|e| e.to_string())?;
        let rows = t.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;
        for r in rows {
            let (sid, tid): (String, String) = r.map_err(|e| e.to_string())?;
            tag_map.entry(sid).or_insert_with(Vec::new).push(tid);
        }
        drop(t);

        let groups = {
            let mut s = conn.prepare(
                "SELECT id, name, sort, created_at, updated_at FROM groups",
            ).map_err(|e| e.to_string())?;
            let rows = s.query_map([], |row| {
                Ok(Group {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sort: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            }).map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let tags = {
            let mut s = conn.prepare(
                "SELECT id, name, color, sort, created_at, updated_at FROM tags",
            ).map_err(|e| e.to_string())?;
            let rows = s.query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    sort: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            }).map_err(|e| e.to_string())?;
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
    let export_data: ExportData = serde_json::from_str(&json_data).map_err(|e| e.to_string())?;

    let mut guard = db_conn()?;
    let conn = guard.as_mut().ok_or_else(|| "DB not initialized".to_string())?;
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

    for session in export_data.sessions {
        let metadata = session.metadata;
        let re_encrypted = if let Some(creds) = session.encrypted_credentials {
            let sensitive =
                crate::encryption::EncryptionManager::decrypt_with_key(&creds, &password)?;
            Some(crate::encryption::EncryptionManager::encrypt(&sensitive)?)
        } else {
            None
        };

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

        let _ = tx.execute(
            "DELETE FROM session_groups WHERE session_id = ?1",
            params![metadata.id],
        );
        for gid in session.group_ids {
            let _ = tx.execute(
                "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
                params![metadata.id, gid],
            );
        }

        let _ = tx.execute(
            "DELETE FROM session_tags WHERE session_id = ?1",
            params![metadata.id],
        );
        for tid in session.tag_ids {
            let _ = tx.execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![metadata.id, tid],
            );
        }
    }

    tx.commit().map_err(|e| e.to_string())
}
