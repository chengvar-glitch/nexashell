use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;
use rusqlite::types::ToSql;
use std::path::PathBuf;
use once_cell::sync::Lazy;

/// Platform-specific app data directory path for the SQLite database.
/// Initialized once on first access, then cached.
static DB_PATH: Lazy<Result<PathBuf, String>> = Lazy::new(|| {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?
        .join("NexaShell");
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok(data_dir.join("nexashell.db"))
});

/// Get the cached database path, creating the app data directory if needed.
fn db_path() -> Result<&'static PathBuf, String> {
    DB_PATH.as_ref().map_err(|e| e.clone())
}

#[derive(Serialize)]
pub struct Session {
    pub id: String,
    pub addr: String,
    pub port: i64,
    pub server_name: String,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: Option<String>,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub fn init_db() -> Result<String, String> {
    let db_path = db_path()?;
    let existed = db_path.exists();
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Ensure sessions table exists.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            addr TEXT NOT NULL,
            port INTEGER NOT NULL,
            server_name TEXT NOT NULL,
            username TEXT NOT NULL,
            auth_type TEXT NOT NULL,
            private_key_path TEXT,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Migration for is_favorite if it doesn't exist
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0", []);

    // Ensure groups/tags and junction tables exist.
    ensure_groups_and_tags(&conn)?;

    // Create useful indexes to speed up common queries (no foreign-key
    // constraints; indexes only).
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
        "CREATE INDEX IF NOT EXISTS idx_session_tags_tag_id ON session_tags(tag_id)",
        [],
    )
    .map_err(|e| e.to_string())?;

    if !existed {
        // Database file was just created — return a distinct message.
        Ok("created".into())
    } else {
        Ok("ok".into())
    }
}

#[tauri::command]
pub fn add_session(
    addr: String,
    port: i64,
    server_name: String,
    username: String,
    auth_type: String,
    private_key_path: Option<String>,
) -> Result<String, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        params![id, addr, port, server_name, username, auth_type, private_key_path],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Save a new SSH session with groups and tags associations.
/// This command saves session metadata without storing sensitive data (passwords, passphrases).
/// 
/// # Arguments
/// * `addr` - SSH server address (host or IP)
/// * `port` - SSH server port
/// * `server_name` - Human-friendly session name
/// * `username` - SSH username
/// * `auth_type` - Authentication type ('password' or 'key')
/// * `private_key_path` - Path to private key file (optional)
/// * `is_favorite` - Whether the session is favorited (optional)
/// * `group_ids` - List of group IDs to associate with this session (optional)
/// * `tag_ids` - List of tag IDs to associate with this session (optional)
/// 
/// # Returns
/// The UUID of the newly created session
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
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    
    let is_update = id.is_some();
    let session_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
    
    println!("[save_session_with_credentials] {} session: {}", if is_update { "Updating" } else { "Saving new" }, session_id);
    
    // 1. Save session metadata to database (without sensitive information)
    if is_update {
        let mut sql = "UPDATE sessions SET addr = ?1, port = ?2, server_name = ?3, username = ?4, auth_type = ?5, private_key_path = ?6, updated_at = CURRENT_TIMESTAMP".to_string();
        let mut params_vec: Vec<Box<dyn ToSql>> = vec![
            Box::new(addr),
            Box::new(port),
            Box::new(server_name),
            Box::new(username),
            Box::new(auth_type),
            Box::new(private_key_path),
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
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        
        // Clear existing associations to reset them
        conn.execute("DELETE FROM session_groups WHERE session_id = ?1", params![session_id]).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM session_tags WHERE session_id = ?1", params![session_id]).map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![session_id, addr, port, server_name, username, auth_type, private_key_path, if is_favorite.unwrap_or(false) { 1 } else { 0 }],
        ).map_err(|e| e.to_string())?;
    }
    
    // 2. Save sensitive information to system keychain only if changed
    if password.is_some() || key_passphrase.is_some() {
        let should_save = match crate::keychain::KeychainManager::retrieve_credentials(&session_id) {
            Ok(existing) => {
                existing.password != password || existing.key_passphrase != key_passphrase
            },
            Err(_) => true,
        };

        if should_save {
            println!("[save_session_with_credentials] Credentials changed or new, saving to keychain...");
            crate::keychain::KeychainManager::save_credentials(
                &session_id,
                &crate::keychain::SensitiveData {
                    password: password.clone(),
                    key_passphrase: key_passphrase.clone(),
                },
            )?;
        } else {
            println!("[save_session_with_credentials] Credentials unchanged, skipping keychain write to avoid system prompts");
        }
    }
    
    // 3. Associate with groups
    if let Some(groups) = group_ids {
        for group_id in groups {
            conn.execute(
                "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
                params![session_id, group_id],
            ).ok();
        }
    }
    
    // 4. Associate with tags
    if let Some(tags) = tag_ids {
        for tag_id in tags {
            conn.execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![session_id, tag_id],
            ).ok();
        }
    }
    
    Ok(session_id)
}

/// Retrieve sensitive credentials (password and key passphrase) from system keychain
///
/// # Arguments
/// * `session_id` - Session UUID
///
/// # Returns
/// Tuple of (session_id, password_option, key_passphrase_option)
#[tauri::command]
#[allow(non_snake_case)]
pub fn get_session_credentials(sessionId: String) -> Result<(String, Option<String>, Option<String>), String> {
    let credentials = crate::keychain::KeychainManager::retrieve_credentials(&sessionId)?;
    Ok((sessionId, credentials.password, credentials.key_passphrase))
}

/// Save a new SSH session with groups and tags associations.
/// This command saves session metadata without storing sensitive data (passwords, passphrases).
/// 
/// # Arguments
/// * `addr` - SSH server address (host or IP)
/// * `port` - SSH server port
/// * `server_name` - Human-friendly session name
/// * `username` - SSH username
/// * `auth_type` - Authentication type ('password' or 'key')
/// * `private_key_path` - Path to private key file (optional)
/// * `is_favorite` - Whether the session is favorited (optional)
/// * `group_ids` - List of group IDs to associate with this session (optional)
/// * `tag_ids` - List of tag IDs to associate with this session (optional)
/// 
/// # Returns
/// The UUID of the newly created session
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
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    
    let id = Uuid::new_v4().to_string();
    
    // Insert the session
    conn.execute(
        "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path, is_favorite)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, addr, port, server_name, username, auth_type, private_key_path, if is_favorite.unwrap_or(false) { 1 } else { 0 }],
    )
    .map_err(|e| e.to_string())?;
    
    // Associate with groups
    if let Some(groups) = group_ids {
        for group_id in groups {
            conn.execute(
                "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
                params![id, group_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    
    // Associate with tags
    if let Some(tags) = tag_ids {
        for tag_id in tags {
            conn.execute(
                "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    
    Ok(id)
}

#[tauri::command]
pub fn toggle_favorite(id: String, is_favorite: bool) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sessions SET is_favorite = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![if is_favorite { 1 } else { 0 }, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_session_timestamp(id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_sessions() -> Result<Vec<Session>, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, addr, port, server_name, username, auth_type, private_key_path, is_favorite, created_at, updated_at FROM sessions",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                addr: row.get(1)?,
                port: row.get(2)?,
                server_name: row.get(3)?,
                username: row.get(4)?,
                auth_type: row.get(5)?,
                private_key_path: row.get(6)?,
                is_favorite: row.get::<_, i64>(7)? != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|e| e.to_string())?);
    }
    Ok(v)
}

/// Retrieve sessions with optional filters.
///
/// All parameters are optional; when none are provided the full table is
/// returned. Filters:
/// - `group_id`: returns sessions belonging to the specified group
/// - `tag_id`: returns sessions tagged with the specified tag
/// - `id`: filter by primary key
/// - `server_name`: partial match on `server_name` (LIKE)
/// - `host_addr`: partial match on `addr` (LIKE)
#[tauri::command]
pub fn get_sessions(
    group_id: Option<String>,
    tag_id: Option<String>,
    id: Option<String>,
    server_name: Option<String>,
    host_addr: Option<String>,
) -> Result<Vec<Session>, String> {
    let db_path = db_path()?;
    let mut sql = String::from("SELECT DISTINCT s.id, s.addr, s.port, s.server_name, s.username, s.auth_type, s.private_key_path, s.is_favorite, s.created_at, s.updated_at FROM sessions s");
    if group_id.is_some() {
        sql.push_str(" JOIN session_groups sg ON s.id = sg.session_id");
    }
    if tag_id.is_some() {
        sql.push_str(" JOIN session_tags st ON s.id = st.session_id");
    }

    let mut where_clauses: Vec<String> = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(gid) = group_id {
        where_clauses.push("sg.group_id = ?".to_string());
        params_vec.push(Box::new(gid));
    }
    if let Some(tid) = tag_id {
        where_clauses.push("st.tag_id = ?".to_string());
        params_vec.push(Box::new(tid));
    }
    if let Some(pid) = id {
        where_clauses.push("s.id = ?".to_string());
        params_vec.push(Box::new(pid));
    }
    if let Some(name) = server_name {
        where_clauses.push("s.server_name LIKE ?".to_string());
        params_vec.push(Box::new(format!("%{}%", name)));
    }
    if let Some(addr) = host_addr {
        where_clauses.push("s.addr LIKE ?".to_string());
        params_vec.push(Box::new(format!("%{}%", addr)));
    }

    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    // Convert boxed params to &[&dyn ToSql]
    let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(Session {
                id: row.get(0)?,
                addr: row.get(1)?,
                port: row.get(2)?,
                server_name: row.get(3)?,
                username: row.get(4)?,
                auth_type: row.get(5)?,
                private_key_path: row.get(6)?,
                is_favorite: row.get::<_, i64>(7)? != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|e| e.to_string())?);
    }
    Ok(v)
}

/// Edit an existing group. Only provided fields are updated.
#[tauri::command]
pub fn edit_group(id: String, name: Option<String>, sort: Option<i64>) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(n) = name {
        sets.push("name = ?".to_string());
        params_vec.push(Box::new(n));
    }
    if let Some(s) = sort {
        sets.push("sort = ?".to_string());
        params_vec.push(Box::new(s));
    }
    if sets.is_empty() {
        return Ok(());
    }
    // always update updated_at
    sets.push("updated_at = CURRENT_TIMESTAMP".to_string());
    let sql = format!("UPDATE groups SET {} WHERE id = ?", sets.join(", "));
    params_vec.push(Box::new(id));
    let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a group and its logical associations.
#[tauri::command]
pub fn delete_group(id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM session_groups WHERE group_id = ?1", params![id.clone()]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM groups WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

/// Edit an existing tag. Only provided fields are updated.
#[tauri::command]
pub fn edit_tag(id: String, name: Option<String>, color: Option<String>, sort: Option<i64>) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(n) = name {
        sets.push("name = ?".to_string());
        params_vec.push(Box::new(n));
    }
    if let Some(c) = color {
        sets.push("color = ?".to_string());
        params_vec.push(Box::new(c));
    }
    if let Some(s) = sort {
        sets.push("sort = ?".to_string());
        params_vec.push(Box::new(s));
    }
    if sets.is_empty() {
        return Ok(());
    }
    sets.push("updated_at = CURRENT_TIMESTAMP".to_string());
    let sql = format!("UPDATE tags SET {} WHERE id = ?", sets.join(", "));
    params_vec.push(Box::new(id));
    let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a tag and its logical associations.
#[tauri::command]
pub fn delete_tag(id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM session_tags WHERE tag_id = ?1", params![id.clone()]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tags WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

/// Edit an existing session record. Only provided fields are updated.
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
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut sets: Vec<String> = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(a) = addr { sets.push("addr = ?".to_string()); params_vec.push(Box::new(a)); }
    if let Some(p) = port { sets.push("port = ?".to_string()); params_vec.push(Box::new(p)); }
    if let Some(s) = server_name { sets.push("server_name = ?".to_string()); params_vec.push(Box::new(s)); }
    if let Some(u) = username { sets.push("username = ?".to_string()); params_vec.push(Box::new(u)); }
    if let Some(at) = auth_type { sets.push("auth_type = ?".to_string()); params_vec.push(Box::new(at)); }
    if let Some(pk_opt) = private_key_path {
        sets.push("private_key_path = ?".to_string());
        params_vec.push(Box::new(pk_opt));
    }
    if let Some(fav) = is_favorite {
        sets.push("is_favorite = ?".to_string());
        params_vec.push(Box::new(if fav { 1 } else { 0 }));
    }
    if sets.is_empty() { return Ok(()); }
    sets.push("updated_at = CURRENT_TIMESTAMP".to_string());
    let sql = format!("UPDATE sessions SET {} WHERE id = ?", sets.join(", "));
    params_vec.push(Box::new(id));
    let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a session and its logical associations.
#[tauri::command]
pub fn delete_session(id: String) -> Result<(), String> {
    println!("delete_session called with id: {}", id);
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // Delete session_groups
    let rows1 = conn.execute("DELETE FROM session_groups WHERE session_id = ?1", params![id.clone()]).map_err(|e| e.to_string())?;
    println!("Deleted {} rows from session_groups", rows1);
    
    // Delete session_tags
    let rows2 = conn.execute("DELETE FROM session_tags WHERE session_id = ?1", params![id.clone()]).map_err(|e| e.to_string())?;
    println!("Deleted {} rows from session_tags", rows2);
    
    // Delete session
    let rows3 = conn.execute("DELETE FROM sessions WHERE id = ?1", params![id.clone()]).map_err(|e| e.to_string())?;
    println!("Deleted {} rows from sessions table", rows3);
    
    // Also delete sensitive credentials from keychain
    let _ = crate::keychain::KeychainManager::delete_credentials(&id);
    
    println!("Session {} deleted successfully", id);
    Ok(())
}

/// Represents a persisted group for organizing sessions.
#[derive(Serialize)]
pub struct Group {
    /// UUID primary key (string)
    pub id: String,
    /// Group name (default: "默认分组")
    pub name: String,
    /// Sort order (default: 1)
    pub sort: i64,
    /// Creation timestamp (set by SQLite DEFAULT CURRENT_TIMESTAMP)
    pub created_at: String,
    /// Last update timestamp (set by SQLite DEFAULT CURRENT_TIMESTAMP)
    pub updated_at: String,
}

/// Represents a persisted tag.
#[derive(Serialize)]
pub struct Tag {
    /// UUID primary key (string)
    pub id: String,
    /// Tag name (default: empty string)
    pub name: String,
    /// Tag color (hex string, e.g., "#FF0000")
    pub color: Option<String>,
    /// Sort order (default: 1)
    pub sort: i64,
    /// Creation timestamp (set by SQLite DEFAULT CURRENT_TIMESTAMP)
    pub created_at: String,
    /// Last update timestamp (set by SQLite DEFAULT CURRENT_TIMESTAMP)
    pub updated_at: String,
}

/// Create the `groups` and `tags` tables if they do not exist.
fn ensure_groups_and_tags(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '默认分组',
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

    // Try to add color column if it doesn't exist (for existing databases)
    let _ = conn.execute("ALTER TABLE tags ADD COLUMN color TEXT", []);

    // Junction table for sessions <-> groups (logical association only)
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

    // Junction table for sessions <-> tags (logical association only)
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

/// Create a new group and return its UUID.
#[tauri::command]
pub fn add_group(name: Option<String>, sort: Option<i64>) -> Result<String, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_else(|| "默认分组".to_string());
    let sort = sort.unwrap_or(1);
    conn.execute(
        "INSERT INTO groups (id, name, sort) VALUES (?1, ?2, ?3)",
        params![id, name, sort],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Return all groups ordered by `sort` then `created_at`.
#[tauri::command]
pub fn list_groups() -> Result<Vec<Group>, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
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
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|e| e.to_string())?);
    }
    Ok(v)
}

/// Associate a session with a group (logical join). Duplicate associations
/// are ignored.
#[tauri::command]
pub fn link_session_group(session_id: String, group_id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    conn.execute(
        "INSERT OR IGNORE INTO session_groups (session_id, group_id) VALUES (?1, ?2)",
        params![session_id, group_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove the association between a session and a group.
#[tauri::command]
pub fn unlink_session_group(session_id: String, group_id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM session_groups WHERE session_id = ?1 AND group_id = ?2",
        params![session_id, group_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// List groups associated with a given session.
#[tauri::command]
pub fn list_groups_for_session(session_id: String) -> Result<Vec<Group>, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
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
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|e| e.to_string())?);
    }
    Ok(v)
}

/// Create a new tag and return its UUID.
#[tauri::command]
pub fn add_tag(name: Option<String>, color: Option<String>, sort: Option<i64>) -> Result<String, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_else(|| "".to_string());
    let sort = sort.unwrap_or(1);
    conn.execute(
        "INSERT INTO tags (id, name, color, sort) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, color, sort],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Return all tags ordered by `sort` then `created_at`.
#[tauri::command]
pub fn list_tags() -> Result<Vec<Tag>, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
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
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|e| e.to_string())?);
    }
    Ok(v)
}

/// Associate a session with a tag (logical join). Duplicate associations
/// are ignored.
#[tauri::command]
pub fn link_session_tag(session_id: String, tag_id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    conn.execute(
        "INSERT OR IGNORE INTO session_tags (session_id, tag_id) VALUES (?1, ?2)",
        params![session_id, tag_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove the association between a session and a tag.
#[tauri::command]
pub fn unlink_session_tag(session_id: String, tag_id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM session_tags WHERE session_id = ?1 AND tag_id = ?2",
        params![session_id, tag_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// List tags associated with a given session.
#[tauri::command]
pub fn list_tags_for_session(session_id: String) -> Result<Vec<Tag>, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
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
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|e| e.to_string())?);
    }
    Ok(v)
}
