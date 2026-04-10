use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::state::AppState;
use crate::tunnel::types::TunnelStatus;

pub fn create_router(state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
        .route("/api/tunnels", get(list_tunnels))
        .route("/api/tunnels/{id}/status", get(get_tunnel_status))
        .route("/api/tunnels/{id}/stop", post(stop_tunnel))
        .with_state(state)
}

async fn check_token(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    let settings = &state.settings;
    if let Some(ref expected_token) = settings.api_token {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        match provided {
            Some(token) if token == expected_token => Ok(()),
            _ => Err(StatusCode::UNAUTHORIZED),
        }
    } else {
        Ok(()) // No token configured, allow access
    }
}

async fn list_tunnels(
    headers: HeaderMap,
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    check_token(&headers, &state).await?;

    let statuses = state.tunnel_manager.get_statuses();
    let tunnels: Vec<Value> = state
        .tunnels_file
        .tunnels
        .iter()
        .map(|t| {
            let (status, error, uptime) = statuses
                .get(&t.id)
                .cloned()
                .unwrap_or((TunnelStatus::Disconnected, None, None));
            json!({
                "id": t.id,
                "name": t.name,
                "jump_host": t.jump_host,
                "target": format!("{}:{}", t.target_host, t.target_port),
                "local_port": t.local_port,
                "status": status,
                "error": error,
                "uptime_secs": uptime,
            })
        })
        .collect();

    Ok(Json(json!({ "tunnels": tunnels })))
}

async fn get_tunnel_status(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    check_token(&headers, &state).await?;

    let status = state.tunnel_manager.get_status(&id);
    Ok(Json(json!({ "id": id, "status": status })))
}

async fn stop_tunnel(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    check_token(&headers, &state).await?;

    match state.tunnel_manager.stop(&id).await {
        Ok(()) => Ok(Json(json!({ "id": id, "status": "disconnected" }))),
        Err(e) => Ok(Json(json!({ "error": e.to_string() }))),
    }
}
