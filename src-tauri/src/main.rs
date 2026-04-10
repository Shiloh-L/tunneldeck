#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tauri::{Emitter, Manager};

use shelldeck_lib::commands;
use shelldeck_lib::logging::audit::init_logging;
use shelldeck_lib::state::AppState;
use shelldeck_lib::store::audit_logger::AuditLogger;
use shelldeck_lib::store::json_store::JsonStore;
use shelldeck_lib::connection::manager::ConnectionManager;
use shelldeck_lib::connection::types::*;

fn main() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            let state = tauri::async_runtime::block_on(async {
                let json_store = JsonStore::new(app_dir.clone());
                json_store.init().await.expect("Failed to init data directory");

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
                let connection_manager = ConnectionManager::new(audit.clone(), status_tx);

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

            app.manage(state);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running ShellDeck");
}
