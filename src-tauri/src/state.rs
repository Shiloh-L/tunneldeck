use std::sync::Arc;
use tokio::sync::mpsc;

use crate::store::audit_logger::AuditLogger;
use crate::store::json_store::JsonStore;
use crate::tunnel::manager::TunnelManager;
use crate::tunnel::types::*;

/// Shared application state, held behind Arc<RwLock<>> in Tauri.
pub struct AppState {
    pub json_store: JsonStore,
    pub audit: Arc<AuditLogger>,
    pub tunnel_manager: TunnelManager,
    pub tunnels_file: TunnelsFile,
    pub tags_file: TagsFile,
    pub settings: AppSettings,
    /// Receiver for tunnel status updates from background tasks.
    pub status_rx: Option<mpsc::Receiver<(String, TunnelStatus, Option<String>)>>,
}
