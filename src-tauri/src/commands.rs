use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};
use tracing::info;

use crate::config::import_export;
use crate::ssh::auth::AuthStatus;
use crate::state::AppState;
use crate::store::credential;
use crate::connection::types::*;

// ─── Connection CRUD ──────────────────────────────────────────────

#[tauri::command]
pub async fn list_connections(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<ConnectionInfo>, String> {
    let state = state.read().await;
    let statuses = state.connection_manager.get_statuses();

    let infos: Vec<ConnectionInfo> = state
        .connections_file
        .connections
        .iter()
        .map(|c| {
            let (status, error, uptime, running_fwd_ids) = statuses
                .get(&c.id)
                .cloned()
                .unwrap_or((ConnectionStatus::Disconnected, None, None, Vec::new()));
            ConnectionInfo {
                config: c.clone(),
                status,
                error_message: error,
                uptime_secs: uptime,
                running_forward_ids: running_fwd_ids,
            }
        })
        .collect();

    Ok(infos)
}

#[derive(Debug, Deserialize)]
pub struct CreateForwardRuleRequest {
    pub name: String,
    pub local_port: u16,
    pub target_host: String,
    pub target_port: u16,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequest {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub private_key_path: Option<String>,
    pub forwards: Vec<CreateForwardRuleRequest>,
    pub auto_connect: bool,
    pub tag_ids: Vec<String>,
}

#[tauri::command]
pub async fn create_connection(
    state: State<'_, Arc<RwLock<AppState>>>,
    req: CreateConnectionRequest,
) -> Result<Connection, String> {
    let mut state = state.write().await;

    let mut config = Connection::new(req.name, req.host, req.port, req.username);
    config.auto_connect = req.auto_connect;
    config.tag_ids = req.tag_ids;
    config.auth_method = req.auth_method;
    config.private_key_path = req.private_key_path;
    config.forwards = req
        .forwards
        .into_iter()
        .map(|f| ForwardRule::new(f.name, f.local_port, f.target_host, f.target_port))
        .collect();

    // Save password to Windows Credential Manager
    credential::save_password(&config.id, &req.password).map_err(|e| e.to_string())?;

    // Add to in-memory state
    state.connections_file.connections.push(config.clone());

    // Persist to disk
    state
        .json_store
        .save("connections.json", &state.connections_file)
        .await
        .map_err(|e| e.to_string())?;

    // Audit log
    let fwd_summary: Vec<String> = config.forwards.iter().map(|f| {
        format!("localhost:{} -> {}:{}", f.local_port, f.target_host, f.target_port)
    }).collect();
    let _ = state
        .audit
        .append(&AuditEntry {
            connection_id: config.id.clone(),
            connection_name: config.name.clone(),
            event: AuditEvent::Created,
            message: format!("Connection created with {} forwards: {}", config.forwards.len(), fwd_summary.join(", ")),
            ts: chrono::Utc::now().to_rfc3339(),
        })
        .await;

    Ok(config)
}

#[tauri::command]
pub async fn update_connection(
    state: State<'_, Arc<RwLock<AppState>>>,
    connection: Connection,
) -> Result<(), String> {
    let mut state = state.write().await;

    let connection_id = connection.id.clone();

    if let Some(existing) = state.connections_file.connections.iter_mut().find(|c| c.id == connection_id) {
        *existing = connection.clone();
        existing.updated_at = chrono::Utc::now().to_rfc3339();
    } else {
        return Err("Connection not found".to_string());
    }

    state
        .json_store
        .save("connections.json", &state.connections_file)
        .await
        .map_err(|e| e.to_string())?;

    // If the connection is currently running, hot-sync forward rules
    let statuses = state.connection_manager.get_statuses();
    if let Some((ConnectionStatus::Connected, _, _, _)) = statuses.get(&connection_id) {
        if let Err(e) = state.connection_manager.sync_forwards(&connection_id, &connection.forwards).await {
            info!("Forward sync for {}: {}", connection_id, e);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_connection(
    state: State<'_, Arc<RwLock<AppState>>>,
    connection_id: String,
) -> Result<(), String> {
    let mut state = state.write().await;

    // Stop if running
    let _ = state.connection_manager.stop(&connection_id).await;

    // Remove credential
    let _ = credential::delete_password(&connection_id);

    // Remove from config
    let name = state
        .connections_file
        .connections
        .iter()
        .find(|c| c.id == connection_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();

    state.connections_file.connections.retain(|c| c.id != connection_id);

    state
        .json_store
        .save("connections.json", &state.connections_file)
        .await
        .map_err(|e| e.to_string())?;

    let _ = state
        .audit
        .append(&AuditEntry {
            connection_id: connection_id.clone(),
            connection_name: name,
            event: AuditEvent::Deleted,
            message: "Connection deleted".into(),
            ts: chrono::Utc::now().to_rfc3339(),
        })
        .await;

    Ok(())
}

// ─── Connection Control ───────────────────────────────────────────

#[tauri::command]
pub async fn start_connection(
    app: AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
    connection_id: String,
    password: Option<String>,
) -> Result<(), String> {
    let mut state = state.write().await;

    let config = state
        .connections_file
        .connections
        .iter()
        .find(|c| c.id == connection_id)
        .cloned()
        .ok_or_else(|| "Connection not found".to_string())?;

    // Get password: from parameter or from credential store
    // For key-based auth, password is optional (used as passphrase if the key is encrypted)
    let pwd = match password {
        Some(p) => p,
        None => credential::load_password(&connection_id)
            .map_err(|e| e.to_string())?
            .unwrap_or_default(),
    };

    // Create auth status channel to update UI during Duo Push
    let (auth_tx, mut auth_rx) = mpsc::channel::<AuthStatus>(16);

    // Forward auth status events to the frontend via Tauri events
    let app_handle = app.clone();
    let cid = connection_id.clone();
    tokio::spawn(async move {
        while let Some(status) = auth_rx.recv().await {
            let payload = serde_json::json!({
                "connectionId": cid,
                "status": match &status {
                    AuthStatus::PromptingPassword => "prompting_password",
                    AuthStatus::WaitingDuoPush => "waiting_duo_push",
                    AuthStatus::Success => "success",
                    AuthStatus::Failed(_) => "failed",
                },
                "message": match &status {
                    AuthStatus::Failed(msg) => msg.clone(),
                    _ => String::new(),
                },
            });
            let _ = app_handle.emit("connection-auth-status", payload);
        }
    });

    let max_reconnect = state.settings.max_reconnect_attempts;

    state
        .connection_manager
        .start(config, pwd, auth_tx, max_reconnect)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn stop_connection(
    state: State<'_, Arc<RwLock<AppState>>>,
    connection_id: String,
) -> Result<(), String> {
    let mut state = state.write().await;
    state
        .connection_manager
        .stop(&connection_id)
        .await
        .map_err(|e| e.to_string())
}

// ─── Tags ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_tags(state: State<'_, Arc<RwLock<AppState>>>) -> Result<Vec<Tag>, String> {
    let state = state.read().await;
    Ok(state.tags_file.tags.clone())
}

#[tauri::command]
pub async fn create_tag(
    state: State<'_, Arc<RwLock<AppState>>>,
    name: String,
    color: String,
) -> Result<Tag, String> {
    let mut state = state.write().await;
    let tag = Tag::new(name, color);
    state.tags_file.tags.push(tag.clone());
    state
        .json_store
        .save("tags.json", &state.tags_file)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tag)
}

#[tauri::command]
pub async fn delete_tag(
    state: State<'_, Arc<RwLock<AppState>>>,
    tag_id: String,
) -> Result<(), String> {
    let mut state = state.write().await;
    state.tags_file.tags.retain(|t| t.id != tag_id);

    // Remove tag from all connections
    for conn in &mut state.connections_file.connections {
        conn.tag_ids.retain(|id| id != &tag_id);
    }

    state.json_store.save("tags.json", &state.tags_file).await.map_err(|e| e.to_string())?;
    state.json_store.save("connections.json", &state.connections_file).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Password Management ─────────────────────────────────────────

#[tauri::command]
pub async fn save_connection_password(connection_id: String, password: String) -> Result<(), String> {
    credential::save_password(&connection_id, &password).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn has_stored_password(connection_id: String) -> Result<bool, String> {
    let pwd = credential::load_password(&connection_id).map_err(|e| e.to_string())?;
    Ok(pwd.is_some())
}

// ─── Audit Logs ───────────────────────────────────────────────────

#[tauri::command]
pub async fn get_audit_logs(
    state: State<'_, Arc<RwLock<AppState>>>,
    days: Option<u32>,
) -> Result<Vec<AuditEntry>, String> {
    let state = state.read().await;
    state
        .audit
        .read_recent(days.unwrap_or(7))
        .await
        .map_err(|e| e.to_string())
}

// ─── Settings ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_settings(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<AppSettings, String> {
    let state = state.read().await;
    Ok(state.settings.clone())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, Arc<RwLock<AppState>>>,
    settings: AppSettings,
) -> Result<(), String> {
    let mut state = state.write().await;
    state.settings = settings.clone();
    state
        .json_store
        .save("settings.json", &state.settings)
        .await
        .map_err(|e| e.to_string())
}

// ─── Import / Export ──────────────────────────────────────────────

#[tauri::command]
pub async fn export_config(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<String, String> {
    let state = state.read().await;
    import_export::export_config(&state.connections_file, &state.tags_file).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_config(
    state: State<'_, Arc<RwLock<AppState>>>,
    json: String,
) -> Result<u32, String> {
    let data = import_export::import_config(&json).map_err(|e| e.to_string())?;
    let mut state = state.write().await;

    let count = data.connections.len() as u32;

    // Merge: add connections that don't already exist (by name)
    for conn in data.connections {
        if !state.connections_file.connections.iter().any(|c| c.name == conn.name) {
            state.connections_file.connections.push(conn);
        }
    }

    // Merge tags
    for tag in data.tags.tags {
        if !state.tags_file.tags.iter().any(|t| t.name == tag.name) {
            state.tags_file.tags.push(tag);
        }
    }

    state.json_store.save("connections.json", &state.connections_file).await.map_err(|e| e.to_string())?;
    state.json_store.save("tags.json", &state.tags_file).await.map_err(|e| e.to_string())?;

    Ok(count)
}

// ─── Terminal ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn open_terminal(
    app: AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
    connection_id: String,
    cols: u32,
    rows: u32,
) -> Result<String, String> {
    let mut state = state.write().await;
    state
        .connection_manager
        .open_terminal(&connection_id, cols, rows, app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_terminal(
    state: State<'_, Arc<RwLock<AppState>>>,
    terminal_id: String,
    data: String,
) -> Result<(), String> {
    let state = state.read().await;
    state
        .connection_manager
        .write_terminal(&terminal_id, data.into_bytes())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resize_terminal(
    state: State<'_, Arc<RwLock<AppState>>>,
    terminal_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let state = state.read().await;
    state
        .connection_manager
        .resize_terminal(&terminal_id, cols, rows)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_terminal(
    state: State<'_, Arc<RwLock<AppState>>>,
    terminal_id: String,
) -> Result<(), String> {
    let should_disconnect;
    let connection_id;
    {
        let mut s = state.write().await;
        connection_id = s
            .connection_manager
            .close_terminal(&terminal_id)
            .map_err(|e| e.to_string())?;
        should_disconnect = s.connection_manager.should_auto_disconnect(&connection_id);
    } // Release the write lock before spawning

    if should_disconnect {
        info!("Scheduling auto-disconnect for {} (no terminals or forwards remaining)", connection_id);
        let state_arc = state.inner().clone();
        tokio::spawn(async move {
            // Brief delay to let terminal task fully exit and clean up
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let mut s = state_arc.write().await;
            // Re-check: another terminal might have been opened in the meantime
            if s.connection_manager.should_auto_disconnect(&connection_id) {
                info!("Auto-disconnecting {}", connection_id);
                let _ = s.connection_manager.stop(&connection_id).await;
            }
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn exit_app(
    app: AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<(), String> {
    {
        let mut s = state.write().await;
        s.connection_manager.stop_all().await;
    }
    app.exit(0);
    Ok(())
}
