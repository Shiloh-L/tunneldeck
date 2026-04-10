use std::sync::Arc;
use tokio::sync::mpsc;

use crate::store::audit_logger::AuditLogger;
use crate::store::json_store::JsonStore;
use crate::connection::manager::ConnectionManager;
use crate::connection::types::*;

/// Shared application state, held behind Arc<RwLock<>> in Tauri.
pub struct AppState {
    pub json_store: JsonStore,
    pub audit: Arc<AuditLogger>,
    pub connection_manager: ConnectionManager,
    pub connections_file: ConnectionsFile,
    pub tags_file: TagsFile,
    pub settings: AppSettings,
    /// Receiver for connection status updates from background tasks.
    pub status_rx: Option<mpsc::Receiver<(String, ConnectionStatus, Option<String>)>>,
}
