mod system;
mod ssh;

use tauri::Manager;
use ssh::SshManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(SshManager::default())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::NSWindow;
                use cocoa::base::{id, NO, YES};
                
                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(ns_window) = window.ns_window() {
                        let ns_window = ns_window as id;
                        
                        // Configure transparent window for macOS (non-movable)
                        unsafe {
                            ns_window.setOpaque_(NO);
                            ns_window.setBackgroundColor_(cocoa::appkit::NSColor::clearColor(cocoa::base::nil));
                            ns_window.setTitlebarAppearsTransparent_(YES);
                            ns_window.setMovableByWindowBackground_(NO);
                        }
                    }
                }
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            system::get_platform,
            system::get_arch,
            system::is_macos,
            system::is_windows,
            system::is_linux,
            system::quit_app,
            system::toggle_maximize,
            system::minimize_window,
            system::close_window,
            ssh::connect_ssh,
            ssh::disconnect_ssh,
            ssh::send_ssh_input,
            ssh::get_ssh_output,
            ssh::get_buffered_ssh_output,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let manager = app_handle.state::<SshManager>();
                manager.disconnect_all();
            }
        });
}


