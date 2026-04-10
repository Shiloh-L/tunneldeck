use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::routes;
use crate::state::AppState;

/// Start the local REST API server on 127.0.0.1:{port}.
/// If port is 0, a random available port is assigned.
pub async fn start_api_server(state: Arc<RwLock<AppState>>, port: u16) -> anyhow::Result<u16> {
    let app = routes::create_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();

    info!("REST API listening on http://127.0.0.1:{}", actual_port);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("API server error: {}", e);
        }
    });

    Ok(actual_port)
}
