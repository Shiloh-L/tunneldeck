use anyhow::{anyhow, Context, Result};
use russh::client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{error, info};

use crate::ssh::auth::{AuthHandler, AuthStatus};
use crate::ssh::client::SshClient;
use crate::ssh::tunnel::start_multi_forward;
use crate::store::audit_logger::AuditLogger;
use crate::tunnel::types::*;

/// Handle to a running connection, used to stop it.
struct ConnectionHandle {
    config: Connection,
    shutdown_tx: watch::Sender<bool>,
    status: ConnectionStatus,
    error_message: Option<String>,
    connected_at: Option<std::time::Instant>,
}

/// Manages all connection lifecycles.
pub struct TunnelManager {
    connections: HashMap<String, ConnectionHandle>,
    audit: Arc<AuditLogger>,
    /// Channel to send status updates to the frontend.
    status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
}

impl TunnelManager {
    pub fn new(
        audit: Arc<AuditLogger>,
        status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            audit,
            status_tx,
        }
    }

    /// Start a connection. Returns immediately; the connection runs in the background.
    pub async fn start(
        &mut self,
        config: Connection,
        password: String,
        auth_status_tx: mpsc::Sender<AuthStatus>,
    ) -> Result<()> {
        let id = config.id.clone();

        if self.connections.contains_key(&id) {
            return Err(anyhow!("Connection {} is already running", config.name));
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Mark as connecting
        self.update_status(&id, ConnectionStatus::Connecting, None).await;

        let handle = ConnectionHandle {
            config: config.clone(),
            shutdown_tx,
            status: ConnectionStatus::Connecting,
            error_message: None,
            connected_at: None,
        };
        self.connections.insert(id.clone(), handle);

        // Spawn the connection task
        let audit = self.audit.clone();
        let status_tx = self.status_tx.clone();

        tokio::spawn(async move {
            let result = connect_and_forward(
                config.clone(),
                password,
                shutdown_rx,
                auth_status_tx,
                status_tx.clone(),
            )
            .await;

            match &result {
                Ok(()) => {
                    info!("Connection {} stopped gracefully", config.name);
                    let _ = status_tx
                        .send((config.id.clone(), ConnectionStatus::Disconnected, None))
                        .await;
                    let _ = audit
                        .append(&AuditEntry {
                            connection_id: config.id.clone(),
                            connection_name: config.name.clone(),
                            event: AuditEvent::Disconnected,
                            message: "Connection stopped".into(),
                            ts: chrono::Utc::now().to_rfc3339(),
                        })
                        .await;
                }
                Err(e) => {
                    error!("Connection {} failed: {}", config.name, e);
                    let _ = status_tx
                        .send((
                            config.id.clone(),
                            ConnectionStatus::Error,
                            Some(e.to_string()),
                        ))
                        .await;
                    let _ = audit
                        .append(&AuditEntry {
                            connection_id: config.id.clone(),
                            connection_name: config.name.clone(),
                            event: AuditEvent::Error,
                            message: e.to_string(),
                            ts: chrono::Utc::now().to_rfc3339(),
                        })
                        .await;
                }
            }
        });

        Ok(())
    }

    /// Stop a running connection.
    pub async fn stop(&mut self, connection_id: &str) -> Result<()> {
        if let Some(handle) = self.connections.remove(connection_id) {
            info!("Stopping connection {}", handle.config.name);
            let _ = handle.shutdown_tx.send(true);
            self.update_status(connection_id, ConnectionStatus::Disconnected, None)
                .await;
            Ok(())
        } else {
            Err(anyhow!("Connection {} is not running", connection_id))
        }
    }

    /// Stop all running connections.
    pub async fn stop_all(&mut self) {
        let ids: Vec<String> = self.connections.keys().cloned().collect();
        for id in ids {
            let _ = self.stop(&id).await;
        }
    }

    /// Get info for all connections.
    pub fn get_statuses(&self) -> HashMap<String, (ConnectionStatus, Option<String>, Option<u64>)> {
        self.connections
            .iter()
            .map(|(id, h)| {
                let uptime = h
                    .connected_at
                    .map(|t| t.elapsed().as_secs());
                (id.clone(), (h.status, h.error_message.clone(), uptime))
            })
            .collect()
    }

    /// Update internal status and notify frontend.
    async fn update_status(
        &self,
        connection_id: &str,
        status: ConnectionStatus,
        error: Option<String>,
    ) {
        let _ = self
            .status_tx
            .send((connection_id.to_string(), status, error))
            .await;
    }
}

/// The actual connection + forwarding logic, runs in a spawned task.
async fn connect_and_forward(
    config: Connection,
    password: String,
    shutdown_rx: watch::Receiver<bool>,
    auth_status_tx: mpsc::Sender<AuthStatus>,
    status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
) -> Result<()> {
    let ssh_config = client::Config::default();
    let ssh_config = Arc::new(ssh_config);

    let handler = SshClient::new();

    // Connect to SSH host
    let addr = format!("{}:{}", config.host, config.port);
    info!("Connecting to SSH host: {}", addr);

    let mut session = client::connect(ssh_config, &addr, handler)
        .await
        .with_context(|| format!("Failed to connect to {}", addr))?;

    // Run keyboard-interactive authentication (password + Duo Push)
    let _ = status_tx
        .send((config.id.clone(), ConnectionStatus::Connecting, None))
        .await;

    let mut auth_handler = AuthHandler::new(password, auth_status_tx);
    auth_handler
        .run_auth(&mut session, &config.username)
        .await
        .context("Authentication failed")?;

    // Auth succeeded → mark as connected
    let _ = status_tx
        .send((config.id.clone(), ConnectionStatus::Connected, None))
        .await;

    let forward_summary: Vec<String> = config
        .forwards
        .iter()
        .filter(|f| f.enabled)
        .map(|f| format!("localhost:{} -> {}:{}", f.local_port, f.target_host, f.target_port))
        .collect();
    info!(
        "Connection {} authenticated. Forwards: [{}]",
        config.name,
        forward_summary.join(", ")
    );

    // Start all port forwards on this single SSH session
    start_multi_forward(session, config.forwards, shutdown_rx).await?;

    Ok(())
}
