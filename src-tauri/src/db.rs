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
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

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
        "INSERT INTO sessions (id, addr, port, server_name, username, auth_type, private_key_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, addr, port, server_name, username, auth_type, private_key_path],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn list_sessions() -> Result<Vec<Session>, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, addr, port, server_name, username, auth_type, private_key_path, created_at, updated_at FROM sessions",
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
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
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
    let mut sql = String::from("SELECT DISTINCT s.id, s.addr, s.port, s.server_name, s.username, s.auth_type, s.private_key_path, s.created_at, s.updated_at FROM sessions s");
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
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
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
    let mut sql = format!("UPDATE groups SET {} WHERE id = ?", sets.join(", "));
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
pub fn edit_tag(id: String, name: Option<String>, sort: Option<i64>) -> Result<(), String> {
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
    sets.push("updated_at = CURRENT_TIMESTAMP".to_string());
    let mut sql = format!("UPDATE tags SET {} WHERE id = ?", sets.join(", "));
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
    if sets.is_empty() { return Ok(()); }
    sets.push("updated_at = CURRENT_TIMESTAMP".to_string());
    let mut sql = format!("UPDATE sessions SET {} WHERE id = ?", sets.join(", "));
    params_vec.push(Box::new(id));
    let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| &**b as &dyn ToSql).collect();
    conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a session and its logical associations.
#[tauri::command]
pub fn delete_session(id: String) -> Result<(), String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM session_groups WHERE session_id = ?1", params![id.clone()]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM session_tags WHERE session_id = ?1", params![id.clone()]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
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
            sort INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

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
pub fn add_tag(name: Option<String>, sort: Option<i64>) -> Result<String, String> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    ensure_groups_and_tags(&conn)?;
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_else(|| "".to_string());
    let sort = sort.unwrap_or(1);
    conn.execute(
        "INSERT INTO tags (id, name, sort) VALUES (?1, ?2, ?3)",
        params![id, name, sort],
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
        .prepare("SELECT id, name, sort, created_at, updated_at FROM tags ORDER BY sort, created_at")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Tag {
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
            "SELECT t.id, t.name, t.sort, t.created_at, t.updated_at
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
