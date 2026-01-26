use tauri::{command, AppHandle, Window};

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

#[command]
pub async fn read_file_preview(path: String) -> Result<String, String> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 1024]; // 读取前 1KB 演示
    let n = file.read(&mut buffer).map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&buffer[..n]).to_string())
}

#[command]
pub async fn get_file_size(path: String) -> Result<serde_json::Value, String> {
    use std::fs;

    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "size": metadata.len() }))
}
