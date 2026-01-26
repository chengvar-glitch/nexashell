mod db;
mod encryption;
mod ssh;
mod system;
mod terminal;

use ssh::SshManager;
use tauri::Manager;
use terminal::TerminalManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(SshManager::default())
        .manage(TerminalManager::default())
        .setup(|app| {
            // Initialize database before app is fully started. This ensures
            // schema and indexes exist even if the DB file was absent.
            match db::init_db() {
                Ok(v) => {
                    // log to stdout for visibility during startup
                    println!("db init: {}", v);
                }
                Err(e) => {
                    eprintln!("db init error: {}", e);
                }
            }
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::NSWindow;
                use cocoa::base::{id, NO, YES};

                if let Some(window) = app.get_webview_window("main") {
                    // Ensure the webview itself is transparent
                    let _ = window.set_shadow(true);

                    if let Ok(ns_window) = window.ns_window() {
                        let ns_window = ns_window as id;

                        // Configure transparent window for macOS (non-movable)
                        unsafe {
                            ns_window.setOpaque_(NO);
                            ns_window.setBackgroundColor_(cocoa::appkit::NSColor::clearColor(
                                cocoa::base::nil,
                            ));
                            ns_window.setTitlebarAppearsTransparent_(YES);
                            ns_window.setMovableByWindowBackground_(NO);

                            // Force refresh shadow to prevent square black corners
                            ns_window.setHasShadow_(NO);
                            ns_window.setHasShadow_(YES);
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
            system::read_file_preview,
            ssh::connect_ssh,
            ssh::disconnect_ssh,
            ssh::send_ssh_input,
            ssh::get_ssh_output,
            ssh::get_buffered_ssh_output,
            ssh::upload_file_sftp,
            ssh::probe_remote_path,
            terminal::connect_local,
            terminal::disconnect_local,
            db::init_db,
            db::add_session,
            db::save_session,
            db::save_session_with_credentials,
            db::update_session_timestamp,
            db::list_sessions,
            db::get_session_credentials,
            db::add_group,
            db::list_groups,
            db::add_tag,
            db::list_tags,
            db::link_session_group,
            db::unlink_session_group,
            db::list_groups_for_session,
            db::link_session_tag,
            db::unlink_session_tag,
            db::list_tags_for_session,
            db::get_sessions,
            db::edit_group,
            db::delete_group,
            db::edit_tag,
            db::delete_tag,
            db::edit_session,
            db::delete_session,
            db::toggle_favorite,
            db::export_sessions,
            db::import_sessions,
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
