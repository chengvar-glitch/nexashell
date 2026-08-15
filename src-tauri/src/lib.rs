mod common;
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
    let log_builder = tauri_plugin_log::Builder::default()
        .level(log::LevelFilter::Info)
        .level_for("rusqlite", log::LevelFilter::Warn)
        .target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::LogDir {
                file_name: Some("nexashell".to_string()),
            },
        ));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(log_builder.build())
        .manage(SshManager::default())
        .manage(TerminalManager::default())
        .setup(|app| {
            if let Err(e) = db::init_db() {
                log::error!("Database initialization failed: {}", e);
            }

            encryption::EncryptionManager::init();

            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::{NSWindow, NSWindowTitleVisibility};
                use cocoa::base::{NO, YES, id};

                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_shadow(true);

                    if let Ok(ns_window) = window.ns_window() {
                        let ns_window = ns_window as id;

                        unsafe {
                            ns_window
                                .setTitleVisibility_(NSWindowTitleVisibility::NSWindowTitleHidden);
                            ns_window.setOpaque_(NO);
                            ns_window.setBackgroundColor_(cocoa::appkit::NSColor::clearColor(
                                cocoa::base::nil,
                            ));
                            ns_window.setMovableByWindowBackground_(NO);
                            ns_window.setHasShadow_(NO);
                            ns_window.setHasShadow_(YES);
                        }
                    }
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_decorations(false);
                    let _ = window.center();
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
            system::get_file_size,
            ssh::connect_ssh,
            ssh::disconnect_ssh,
            ssh::send_ssh_input,
            ssh::get_buffered_ssh_output,
            ssh::upload_file_sftp,
            ssh::probe_remote_path,
            ssh::pause_upload,
            ssh::resume_upload,
            ssh::cancel_upload,
            ssh::set_ssh_status_refresh_rate,
            terminal::connect_local,
            terminal::disconnect_local,
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
            db::get_sessions_with_relations,
            db::edit_group,
            db::delete_group,
            db::edit_tag,
            db::delete_tag,
            db::edit_session,
            db::delete_session,
            db::toggle_favorite,
            db::import_export::export_sessions,
            db::import_export::import_sessions,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(manager) = app_handle.try_state::<SshManager>() {
                    manager.disconnect_all();
                }
                if let Some(manager) = app_handle.try_state::<TerminalManager>() {
                    manager.disconnect_all();
                }
            }
        });
}
