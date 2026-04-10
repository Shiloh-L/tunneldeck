use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};

use crate::config::import_export;
use crate::ssh::auth::AuthStatus;
use crate::state::AppState;
use crate::store::credential;
use crate::tunnel::types::*;

// ─── Tunnel CRUD ──────────────────────────────────────────────────

#[tauri::command]
pub async fn list_tunnels(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<TunnelInfo>, String> {
    let state = state.read().await;
    let statuses = state.tunnel_manager.get_statuses();

    let infos: Vec<TunnelInfo> = state
        .tunnels_file
        .tunnels
        .iter()
        .map(|t| {
            let (status, error, uptime) = statuses
                .get(&t.id)
                .cloned()
                .unwrap_or((TunnelStatus::Disconnected, None, None));
            TunnelInfo {
                config: t.clone(),
                status,
                error_message: error,
                uptime_secs: uptime,
            }
        })
        .collect();

    Ok(infos)
}

#[derive(Debug, Deserialize)]
pub struct CreateTunnelRequest {
    pub name: String,
    pub jump_host: String,
    pub jump_port: u16,
    pub username: String,
    pub target_host: String,
    pub target_port: u16,
    pub local_port: u16,
    pub password: String,
    pub auto_connect: bool,
    pub tag_ids: Vec<String>,
}

#[tauri::command]
pub async fn create_tunnel(
    state: State<'_, Arc<RwLock<AppState>>>,
    req: CreateTunnelRequest,
) -> Result<TunnelConfig, String> {
    let mut state = state.write().await;

    let mut config = TunnelConfig::new(
        req.name,
        req.jump_host,
        req.jump_port,
        req.username,
        req.target_host,
        req.target_port,
        req.local_port,
    );
    config.auto_connect = req.auto_connect;
    config.tag_ids = req.tag_ids;

    // Save password to Windows Credential Manager
    credential::save_password(&config.id, &req.password).map_err(|e| e.to_string())?;

    // Add to in-memory state
    state.tunnels_file.tunnels.push(config.clone());

    // Persist to disk
    state
        .json_store
        .save("tunnels.json", &state.tunnels_file)
        .await
        .map_err(|e| e.to_string())?;

    // Audit log
    let _ = state
        .audit
        .append(&AuditEntry {
            tunnel_id: config.id.clone(),
            tunnel_name: config.name.clone(),
            event: AuditEvent::Created,
            message: format!("Tunnel created: localhost:{} -> {}:{}", config.local_port, config.target_host, config.target_port),
            ts: chrono::Utc::now().to_rfc3339(),
        })
        .await;

    Ok(config)
}

#[tauri::command]
pub async fn update_tunnel(
    state: State<'_, Arc<RwLock<AppState>>>,
    tunnel: TunnelConfig,
) -> Result<(), String> {
    let mut state = state.write().await;

    if let Some(existing) = state.tunnels_file.tunnels.iter_mut().find(|t| t.id == tunnel.id) {
        *existing = tunnel.clone();
        existing.updated_at = chrono::Utc::now().to_rfc3339();
    } else {
        return Err("Tunnel not found".to_string());
    }

    state
        .json_store
        .save("tunnels.json", &state.tunnels_file)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_tunnel(
    state: State<'_, Arc<RwLock<AppState>>>,
    tunnel_id: String,
) -> Result<(), String> {
    let mut state = state.write().await;

    // Stop if running
    let _ = state.tunnel_manager.stop(&tunnel_id).await;

    // Remove credential
    let _ = credential::delete_password(&tunnel_id);

    // Remove from config
    let name = state
        .tunnels_file
        .tunnels
        .iter()
        .find(|t| t.id == tunnel_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    state.tunnels_file.tunnels.retain(|t| t.id != tunnel_id);

    state
        .json_store
        .save("tunnels.json", &state.tunnels_file)
        .await
        .map_err(|e| e.to_string())?;

    let _ = state
        .audit
        .append(&AuditEntry {
            tunnel_id: tunnel_id.clone(),
            tunnel_name: name,
            event: AuditEvent::Deleted,
            message: "Tunnel deleted".into(),
            ts: chrono::Utc::now().to_rfc3339(),
        })
        .await;

    Ok(())
}

// ─── Tunnel Control ───────────────────────────────────────────────

#[tauri::command]
pub async fn start_tunnel(
    app: AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
    tunnel_id: String,
    password: Option<String>,
) -> Result<(), String> {
    let mut state = state.write().await;

    let config = state
        .tunnels_file
        .tunnels
        .iter()
        .find(|t| t.id == tunnel_id)
        .cloned()
        .ok_or_else(|| "Tunnel not found".to_string())?;

    // Get password: from parameter or from credential store
    let pwd = match password {
        Some(p) => p,
        None => credential::load_password(&tunnel_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "No password stored for this tunnel".to_string())?,
    };

    // Create auth status channel to update UI during Duo Push
    let (auth_tx, mut auth_rx) = mpsc::channel::<AuthStatus>(16);

    // Forward auth status events to the frontend via Tauri events
    let app_handle = app.clone();
    let tid = tunnel_id.clone();
    tokio::spawn(async move {
        while let Some(status) = auth_rx.recv().await {
            let event_name = "tunnel-auth-status";
            let payload = serde_json::json!({
                "tunnelId": tid,
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
            let _ = app_handle.emit(event_name, payload);
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
pub async fn stop_tunnel(
    state: State<'_, Arc<RwLock<AppState>>>,
    tunnel_id: String,
) -> Result<(), String> {
    let mut state = state.write().await;
    state
        .tunnel_manager
        .stop(&tunnel_id)
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

    // Remove tag from all tunnels
    for tunnel in &mut state.tunnels_file.tunnels {
        tunnel.tag_ids.retain(|id| id != &tag_id);
    }

    state.json_store.save("tags.json", &state.tags_file).await.map_err(|e| e.to_string())?;
    state.json_store.save("tunnels.json", &state.tunnels_file).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Password Management ─────────────────────────────────────────

#[tauri::command]
pub async fn save_tunnel_password(tunnel_id: String, password: String) -> Result<(), String> {
    credential::save_password(&tunnel_id, &password).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn has_stored_password(tunnel_id: String) -> Result<bool, String> {
    let pwd = credential::load_password(&tunnel_id).map_err(|e| e.to_string())?;
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
    import_export::export_config(&state.tunnels_file, &state.tags_file).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_config(
    state: State<'_, Arc<RwLock<AppState>>>,
    json: String,
) -> Result<u32, String> {
    let data = import_export::import_config(&json).map_err(|e| e.to_string())?;
    let mut state = state.write().await;

    let count = data.tunnels.len() as u32;

    // Merge: add tunnels that don't already exist (by name)
    for tunnel in data.tunnels {
        if !state.tunnels_file.tunnels.iter().any(|t| t.name == tunnel.name) {
            state.tunnels_file.tunnels.push(tunnel);
        }
    }

    // Merge tags
    for tag in data.tags.tags {
        if !state.tags_file.tags.iter().any(|t| t.name == tag.name) {
            state.tags_file.tags.push(tag);
        }
    }

    state.json_store.save("tunnels.json", &state.tunnels_file).await.map_err(|e| e.to_string())?;
    state.json_store.save("tags.json", &state.tags_file).await.map_err(|e| e.to_string())?;

    Ok(count)
}
