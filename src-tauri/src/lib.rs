mod system;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::NSWindow;
                use cocoa::base::{id, NO};
                
                let window = app.get_webview_window("main").unwrap();
                let ns_window = window.ns_window().unwrap() as id;
                
                unsafe {
                    ns_window.setOpaque_(NO);
                    ns_window.setBackgroundColor_(cocoa::appkit::NSColor::clearColor(cocoa::base::nil));
                    ns_window.setTitlebarAppearsTransparent_(cocoa::base::YES);
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
