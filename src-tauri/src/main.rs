#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tauri::{Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconEvent;

use shelldeck_lib::commands;
use shelldeck_lib::logging::audit::init_logging_to;
use shelldeck_lib::state::AppState;
use shelldeck_lib::store::audit_logger::AuditLogger;
use shelldeck_lib::store::json_store::JsonStore;
use shelldeck_lib::store::known_hosts::KnownHostsStore;
use shelldeck_lib::connection::manager::ConnectionManager;
use shelldeck_lib::connection::types::*;

/// Resolve portable data directory: `<exe_dir>/data/`.
/// If the exe directory is not writable (e.g. installed to Program Files),
/// falls back to `%LOCALAPPDATA%/ShellDeck/data/`.
fn portable_data_dir(app: &tauri::App) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("data");
            // Check if we can write to the exe directory
            if is_dir_writable(parent) {
                return candidate;
            }
        }
    }
    // Fallback: use system app data dir
    app.path()
        .app_data_dir()
        .expect("Failed to resolve app data directory")
}

/// Test if a directory is writable by attempting to create a temp file.
fn is_dir_writable(dir: &std::path::Path) -> bool {
    let probe = dir.join(".shelldeck_write_test");
    match std::fs::write(&probe, b"test") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = portable_data_dir(app);

            // Ensure data directory exists before anything else
            std::fs::create_dir_all(&app_dir).expect("Failed to create data directory");

            // Initialize tracing to data/logs/
            let log_dir = app_dir.join("logs");
            std::fs::create_dir_all(&log_dir).expect("Failed to create logs directory");
            init_logging_to(&log_dir);

            let state = tauri::async_runtime::block_on(async {
                let json_store = JsonStore::new(app_dir.clone());
                json_store.init().await.expect("Failed to init data directory");

                // Initialize portable credential store
                shelldeck_lib::store::credential::init(&app_dir);

                let connections_file: ConnectionsFile =
                    json_store.load("connections.json").await.unwrap_or_default();
                let tags_file: TagsFile =
                    json_store.load("tags.json").await.unwrap_or_default();
                let settings: AppSettings =
                    json_store.load("settings.json").await.unwrap_or_default();

                let audit = Arc::new(AuditLogger::new(
                    json_store.logs_dir(),
                    settings.log_retention_days,
                ));
                let _ = audit.cleanup_old_logs().await;

                let (status_tx, status_rx) = mpsc::channel(256);
                let known_hosts = Arc::new(KnownHostsStore::new(app_dir.clone()));
                let _ = known_hosts.load().await;
                let connection_manager = ConnectionManager::new(audit.clone(), known_hosts, status_tx);

                Arc::new(RwLock::new(AppState {
                    json_store,
                    audit,
                    connection_manager,
                    connections_file,
                    tags_file,
                    settings,
                    status_rx: Some(status_rx),
                }))
            });

            // Forward connection status events to the frontend
            let state_clone = state.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut rx = {
                    let mut s = state_clone.write().await;
                    s.status_rx.take()
                };
                if let Some(ref mut rx) = rx {
                    while let Some((connection_id, status, error)) = rx.recv().await {
                        let payload = serde_json::json!({
                            "connectionId": connection_id,
                            "status": status,
                            "error": error,
                        });
                        let _ = app_handle.emit("connection-status", payload);
                    }
                }
            });

            // Start REST API if enabled
            let state_for_api = state.clone();
            tauri::async_runtime::spawn(async move {
                let s = state_for_api.read().await;
                if s.settings.api_enabled {
                    let port = s.settings.api_port;
                    drop(s);
                    if let Err(e) =
                        shelldeck_lib::api::server::start_api_server(state_for_api.clone(), port)
                            .await
                    {
                        tracing::error!("Failed to start API server: {}", e);
                    }
                }
            });

            // Auto-connect: start connections marked with auto_connect that have stored passwords
            let state_for_auto = state.clone();
            tauri::async_runtime::spawn(async move {
                let (auto_connections, max_reconnect): (Vec<Connection>, u32) = {
                    let s = state_for_auto.read().await;
                    let conns = s.connections_file
                        .connections
                        .iter()
                        .filter(|c| c.auto_connect)
                        .cloned()
                        .collect();
                    (conns, s.settings.max_reconnect_attempts)
                };

                for config in auto_connections {
                    // For key-based auth, password is optional (passphrase); for password auth, required
                    let pwd = match shelldeck_lib::store::credential::load_password(&config.id) {
                        Ok(Some(p)) => p,
                        Ok(None) if config.auth_method == shelldeck_lib::connection::types::AuthMethod::Key => {
                            String::new()
                        }
                        _ => {
                            tracing::warn!(
                                "Auto-connect skipped for {} (no stored password)",
                                config.name
                            );
                            continue;
                        }
                    };
                    let (auth_tx, _auth_rx) = mpsc::channel::<shelldeck_lib::ssh::auth::AuthStatus>(16);
                    let mut s = state_for_auto.write().await;
                    if let Err(e) = s.connection_manager.start(config.clone(), pwd, auth_tx, max_reconnect).await {
                        tracing::error!("Auto-connect failed for {}: {}", config.name, e);
                    } else {
                        tracing::info!("Auto-connecting {}", config.name);
                    }
                }
            });

            app.manage(state);

            // Set up tray menu
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            if let Some(tray) = app.tray_by_id("main") {
                tray.set_menu(Some(menu))?;
                tray.on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                });
                tray.on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::DoubleClick { .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            }

            // Close-to-tray: handled in .on_window_event() below

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_connections,
            commands::create_connection,
            commands::update_connection,
            commands::delete_connection,
            commands::start_connection,
            commands::stop_connection,
            commands::list_tags,
            commands::create_tag,
            commands::delete_tag,
            commands::save_connection_password,
            commands::has_stored_password,
            commands::get_audit_logs,
            commands::get_settings,
            commands::update_settings,
            commands::export_config,
            commands::import_config,
            commands::open_terminal,
            commands::write_terminal,
            commands::resize_terminal,
            commands::close_terminal,
            commands::exit_app,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide to tray instead of quitting
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ShellDeck");
}
