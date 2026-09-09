use tauri::{AppHandle, Window, command};

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
pub fn toggle_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())?;
    } else {
        window.maximize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub fn minimize_window(window: Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn close_window(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())?;
    Ok(())
}

/// Read a UTF-8 text file chosen by the user via the native dialog. Used for
/// backup-file import; the path comes from `plugin-dialog`, never typed input.
#[command]
pub fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Write a UTF-8 text file to a path chosen via the native save dialog.
#[command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| e.to_string())
}
