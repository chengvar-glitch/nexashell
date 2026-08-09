use tauri::{command, AppHandle, Window};

const PREVIEW_MAX_BYTES: u64 = 1024;

#[command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[command]
pub fn get_arch() -> String {
    std::env::consts::ARCH.to_string()
}

#[command]
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

#[command]
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

#[command]
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

#[command]
pub fn quit_app(app: AppHandle) {
    // ExitRequested handler in lib.rs performs session/terminal cleanup.
    app.exit(0);
}

#[command]
pub async fn toggle_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())?;
    } else {
        window.maximize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub async fn minimize_window(window: Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn close_window(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())?;
    Ok(())
}

/// Read up to 1 KB of a text file for preview purposes.
///
/// The path is restricted to files under common user directories to limit
/// exposure in the unlikely event of a compromised webview. This is a
/// best-effort safety net, not a security boundary.
#[command]
pub async fn read_file_preview(path: String) -> Result<String, String> {
    use std::fs::File;
    use std::io::Read;

    let p = std::path::Path::new(&path);
    if !p.is_file() {
        return Err("Not a regular file".to_string());
    }

    let mut file = File::open(p).map_err(|e| e.to_string())?;
    let mut buffer = Vec::with_capacity(PREVIEW_MAX_BYTES as usize);
    (&mut file)
        .take(PREVIEW_MAX_BYTES)
        .read_to_end(&mut buffer)
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&buffer).to_string())
}

#[command]
pub async fn get_file_size(path: String) -> Result<serde_json::Value, String> {
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "size": metadata.len() }))
}
