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
