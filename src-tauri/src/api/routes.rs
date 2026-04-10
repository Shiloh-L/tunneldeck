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
use crate::tunnel::types::ConnectionStatus;

pub fn create_router(state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
        .route("/api/connections", get(list_connections))
        .route("/api/connections/{id}/status", get(get_connection_status))
        .route("/api/connections/{id}/disconnect", post(disconnect_connection))
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

async fn list_connections(
    headers: HeaderMap,
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    check_token(&headers, &state).await?;

    let statuses = state.tunnel_manager.get_statuses();
    let connections: Vec<Value> = state
        .connections_file
        .connections
        .iter()
        .map(|c| {
            let (status, error, uptime) = statuses
                .get(&c.id)
                .cloned()
                .unwrap_or((ConnectionStatus::Disconnected, None, None));
            let forwards: Vec<Value> = c.forwards.iter().map(|f| {
                json!({
                    "id": f.id,
                    "name": f.name,
                    "local_port": f.local_port,
                    "target_host": f.target_host,
                    "target_port": f.target_port,
                    "enabled": f.enabled,
                })
            }).collect();
            json!({
                "id": c.id,
                "name": c.name,
                "host": c.host,
                "port": c.port,
                "forwards": forwards,
                "status": status,
                "error": error,
                "uptime_secs": uptime,
            })
        })
        .collect();

    Ok(Json(json!({ "connections": connections })))
}

async fn get_connection_status(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    check_token(&headers, &state).await?;

    let statuses = state.tunnel_manager.get_statuses();
    let status = statuses.get(&id).map(|(s, e, u)| json!({
        "status": s,
        "error": e,
        "uptime_secs": u,
    })).unwrap_or_else(|| json!({
        "status": "disconnected",
        "error": null,
        "uptime_secs": null,
    }));
    Ok(Json(json!({ "id": id, "status": status })))
}

async fn disconnect_connection(
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
