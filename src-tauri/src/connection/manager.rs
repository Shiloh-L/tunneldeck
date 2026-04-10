use anyhow::{anyhow, Context, Result};
use russh::client;
use russh::client::Handle;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, watch, Mutex};
use tracing::{error, info};

use crate::ssh::auth::{AuthHandler, AuthStatus};
use crate::ssh::client::SshClient;
use crate::ssh::terminal::{self, TerminalInput};
use crate::ssh::tunnel::start_multi_forward;
use crate::store::audit_logger::AuditLogger;
use crate::connection::types::*;

/// Handle to a running connection, used to stop it.
struct ConnectionHandle {
    config: Connection,
    shutdown_tx: watch::Sender<bool>,
    status: ConnectionStatus,
    error_message: Option<String>,
    connected_at: Option<std::time::Instant>,
    session: Arc<Mutex<Option<Arc<Handle<SshClient>>>>>,
}

/// Handle to an open terminal session.
struct TerminalHandle {
    connection_id: String,
    write_tx: mpsc::UnboundedSender<TerminalInput>,
}

/// Manages all connection lifecycles.
pub struct ConnectionManager {
    connections: HashMap<String, ConnectionHandle>,
    terminals: HashMap<String, TerminalHandle>,
    audit: Arc<AuditLogger>,
    /// Channel to send status updates to the frontend.
    status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
    /// IDs of connections whose background tasks have finished (need cleanup).
    finished_ids: Arc<Mutex<Vec<String>>>,
}

impl ConnectionManager {
    pub fn new(
        audit: Arc<AuditLogger>,
        status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            terminals: HashMap::new(),
            audit,
            status_tx,
            finished_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Remove handles for connections whose background tasks have already finished.
    fn cleanup_finished(&mut self) {
        if let Ok(mut finished) = self.finished_ids.try_lock() {
            for id in finished.drain(..) {
                self.connections.remove(&id);
            }
        }
    }

    /// Start a connection. Returns immediately; the connection runs in the background.
    pub async fn start(
        &mut self,
        config: Connection,
        password: String,
        auth_status_tx: mpsc::Sender<AuthStatus>,
    ) -> Result<()> {
        // Clean up any handles from previously finished (errored/disconnected) tasks
        self.cleanup_finished();

        let id = config.id.clone();

        if self.connections.contains_key(&id) {
            return Err(anyhow!("Connection {} is already running", config.name));
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Mark as connecting
        self.update_status(&id, ConnectionStatus::Connecting, None).await;

        let session_holder: Arc<Mutex<Option<Arc<Handle<SshClient>>>>> =
            Arc::new(Mutex::new(None));

        let handle = ConnectionHandle {
            config: config.clone(),
            shutdown_tx,
            status: ConnectionStatus::Connecting,
            error_message: None,
            connected_at: None,
            session: session_holder.clone(),
        };
        self.connections.insert(id.clone(), handle);

        // Spawn the connection task
        let audit = self.audit.clone();
        let status_tx = self.status_tx.clone();
        let finished_ids = self.finished_ids.clone();

        tokio::spawn(async move {
            let result = connect_and_forward(
                config.clone(),
                password,
                shutdown_rx,
                auth_status_tx,
                status_tx.clone(),
                session_holder,
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

            // Mark this connection as finished so the manager can clean up the handle
            finished_ids.lock().await.push(config.id.clone());
        });

        Ok(())
    }

    /// Stop a running connection.
    pub async fn stop(&mut self, connection_id: &str) -> Result<()> {
        // Close all terminals for this connection
        self.terminals
            .retain(|_, h| h.connection_id != connection_id);

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
        self.terminals.clear();
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

    // ─── Terminal Management ──────────────────────────────────────

    /// Open an interactive terminal on an active connection.
    pub async fn open_terminal(
        &mut self,
        connection_id: &str,
        cols: u32,
        rows: u32,
        app: AppHandle,
    ) -> Result<String> {
        let conn = self
            .connections
            .get(connection_id)
            .ok_or_else(|| anyhow!("Connection {} is not running", connection_id))?;

        let session_guard = conn.session.lock().await;
        let session = session_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Connection {} has no active session", connection_id))?
            .clone();
        drop(session_guard);

        let terminal_id = uuid::Uuid::new_v4().to_string();
        let write_tx = terminal::spawn_terminal(
            terminal_id.clone(),
            connection_id.to_string(),
            session,
            cols,
            rows,
            app,
        );

        self.terminals.insert(
            terminal_id.clone(),
            TerminalHandle {
                connection_id: connection_id.to_string(),
                write_tx,
            },
        );

        info!("Opened terminal {} for connection {}", terminal_id, connection_id);
        Ok(terminal_id)
    }

    /// Write data to a terminal.
    pub fn write_terminal(&self, terminal_id: &str, data: Vec<u8>) -> Result<()> {
        let handle = self
            .terminals
            .get(terminal_id)
            .ok_or_else(|| anyhow!("Terminal {} not found", terminal_id))?;
        handle
            .write_tx
            .send(TerminalInput::Data(data))
            .map_err(|_| anyhow!("Terminal {} is closed", terminal_id))
    }

    /// Resize a terminal.
    pub fn resize_terminal(&self, terminal_id: &str, cols: u32, rows: u32) -> Result<()> {
        let handle = self
            .terminals
            .get(terminal_id)
            .ok_or_else(|| anyhow!("Terminal {} not found", terminal_id))?;
        handle
            .write_tx
            .send(TerminalInput::Resize { cols, rows })
            .map_err(|_| anyhow!("Terminal {} is closed", terminal_id))
    }

    /// Close a terminal.
    pub fn close_terminal(&mut self, terminal_id: &str) -> Result<()> {
        self.terminals
            .remove(terminal_id)
            .ok_or_else(|| anyhow!("Terminal {} not found", terminal_id))?;
        info!("Closed terminal {}", terminal_id);
        Ok(())
    }
}

/// The actual connection + forwarding logic, runs in a spawned task.
async fn connect_and_forward(
    config: Connection,
    password: String,
    shutdown_rx: watch::Receiver<bool>,
    auth_status_tx: mpsc::Sender<AuthStatus>,
    status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
    session_holder: Arc<Mutex<Option<Arc<Handle<SshClient>>>>>,
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

    // Wrap session in Arc and store for terminal access
    let session = Arc::new(session);
    *session_holder.lock().await = Some(session.clone());

    // Start all port forwards on this single SSH session (or wait if none)
    let enabled_count = config.forwards.iter().filter(|f| f.enabled).count();
    if enabled_count > 0 {
        start_multi_forward(session, config.forwards, shutdown_rx).await?;
    } else {
        // No enabled forwards — keep connection alive for terminal use
        let mut shutdown_rx = shutdown_rx;
        let _ = shutdown_rx.changed().await;
    }

    // Clean up session reference
    *session_holder.lock().await = None;

    Ok(())
}
