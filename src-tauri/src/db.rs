use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Serialize)]
pub struct Session {
    pub id: i64,
    pub name: String,
    pub host: String,
}

#[tauri::command]
pub fn init_db() -> Result<String, String> {
    let conn = Connection::open("nexashell.db").map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            host TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok("ok".into())
}

#[tauri::command]
pub fn add_session(name: String, host: String) -> Result<i64, String> {
    let conn = Connection::open("nexashell.db").map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sessions (name, host) VALUES (?1, ?2)",
        params![name, host],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn list_sessions() -> Result<Vec<Session>, String> {
    let conn = Connection::open("nexashell.db").map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, host FROM sessions")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|e| e.to_string())?);
    }
    Ok(v)
}
