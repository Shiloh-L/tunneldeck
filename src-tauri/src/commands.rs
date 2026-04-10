use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};

use crate::config::import_export;
use crate::ssh::auth::AuthStatus;
use crate::state::AppState;
use crate::store::credential;
use crate::tunnel::types::*;

// ─── Connection CRUD ──────────────────────────────────────────────

#[tauri::command]
pub async fn list_connections(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<ConnectionInfo>, String> {
    let state = state.read().await;
    let statuses = state.tunnel_manager.get_statuses();

    let infos: Vec<ConnectionInfo> = state
        .connections_file
        .connections
        .iter()
        .map(|c| {
            let (status, error, uptime) = statuses
                .get(&c.id)
                .cloned()
                .unwrap_or((ConnectionStatus::Disconnected, None, None));
            ConnectionInfo {
                config: c.clone(),
                status,
                error_message: error,
                uptime_secs: uptime,
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

    if let Some(existing) = state.connections_file.connections.iter_mut().find(|c| c.id == connection.id) {
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

    Ok(())
}

#[tauri::command]
pub async fn delete_connection(
    state: State<'_, Arc<RwLock<AppState>>>,
    connection_id: String,
) -> Result<(), String> {
    let mut state = state.write().await;

    // Stop if running
    let _ = state.tunnel_manager.stop(&connection_id).await;

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
pub async fn connect_tunnel(
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
    let pwd = match password {
        Some(p) => p,
        None => credential::load_password(&connection_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "No password stored for this connection".to_string())?,
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

    state
        .tunnel_manager
        .start(config, pwd, auth_tx)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn disconnect_tunnel(
    state: State<'_, Arc<RwLock<AppState>>>,
    connection_id: String,
) -> Result<(), String> {
    let mut state = state.write().await;
    state
        .tunnel_manager
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
