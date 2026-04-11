use anyhow::{anyhow, Context, Result};
use russh::client;
use russh::client::Handle;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::AppHandle;
use tokio::sync::{mpsc, watch, Mutex};
use tracing::{error, info, warn};

use crate::ssh::auth::{AuthHandler, AuthStatus};
use crate::ssh::client::SshClient;
use crate::ssh::terminal::{self, TerminalInput};
use crate::ssh::tunnel::spawn_single_forward;
use crate::store::audit_logger::AuditLogger;
use crate::store::known_hosts::KnownHostsStore;
use crate::connection::types::*;

/// SSH connection timeout (seconds).
const CONNECT_TIMEOUT_SECS: u64 = 15;
/// SSH keepalive interval (seconds).
const KEEPALIVE_INTERVAL_SECS: u64 = 30;
/// Max keepalive failures before disconnect.
const KEEPALIVE_MAX: usize = 3;

/// Shared mutable status readable by both the background task and the manager.
type LiveStatus = Arc<StdMutex<(ConnectionStatus, Option<String>, Option<std::time::Instant>)>>;

/// Handle to a running connection, used to stop it.
struct ConnectionHandle {
    config: Connection,
    shutdown_tx: watch::Sender<bool>,
    live_status: LiveStatus,
    session: Arc<Mutex<Option<Arc<Handle<SshClient>>>>>,
    /// Shutdown senders for individually running forward rules, keyed by rule ID.
    forward_shutdowns: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
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
    known_hosts: Arc<KnownHostsStore>,
    /// Channel to send status updates to the frontend.
    status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
    /// IDs of connections whose background tasks have finished (need cleanup).
    finished_ids: Arc<Mutex<Vec<String>>>,
}

impl ConnectionManager {
    pub fn new(
        audit: Arc<AuditLogger>,
        known_hosts: Arc<KnownHostsStore>,
        status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            terminals: HashMap::new(),
            audit,
            known_hosts,
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
        max_reconnect_attempts: u32,
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
        let forward_shutdowns: Arc<Mutex<HashMap<String, watch::Sender<bool>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let live_status: LiveStatus =
            Arc::new(StdMutex::new((ConnectionStatus::Connecting, None, None)));

        let handle = ConnectionHandle {
            config: config.clone(),
            shutdown_tx,
            live_status: live_status.clone(),
            session: session_holder.clone(),
            forward_shutdowns: forward_shutdowns.clone(),
        };
        self.connections.insert(id.clone(), handle);

        // Spawn the connection task
        let audit = self.audit.clone();
        let known_hosts = self.known_hosts.clone();
        let status_tx = self.status_tx.clone();
        let finished_ids = self.finished_ids.clone();

        let live_status_for_task = live_status.clone();
        let shutdown_rx_clone = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut attempt = 0u32;
            let mut first_run = true;

            loop {
                // Check if shutdown was requested before attempting (re)connect
                if *shutdown_rx_clone.borrow() {
                    break;
                }

                if !first_run {
                    attempt += 1;
                    if attempt > max_reconnect_attempts {
                        error!("Connection {} exceeded max reconnect attempts ({})", config.name, max_reconnect_attempts);
                        if let Ok(mut s) = live_status_for_task.lock() {
                            *s = (ConnectionStatus::Error, Some("Max reconnect attempts exceeded".into()), None);
                        }
                        let _ = status_tx
                            .send((config.id.clone(), ConnectionStatus::Error, Some("Max reconnect attempts exceeded".into())))
                            .await;
                        break;
                    }

                    // Exponential backoff: 2s, 4s, 8s, ... capped at 30s
                    let delay_secs = std::cmp::min(2u64.pow(attempt), 30);
                    warn!("Reconnecting {} (attempt {}/{}), waiting {}s...", config.name, attempt, max_reconnect_attempts, delay_secs);

                    if let Ok(mut s) = live_status_for_task.lock() {
                        *s = (ConnectionStatus::Reconnecting, None, None);
                    }
                    let _ = status_tx
                        .send((config.id.clone(), ConnectionStatus::Reconnecting, None))
                        .await;

                    // Wait for backoff delay or shutdown
                    let mut shutdown_wait = shutdown_rx_clone.clone();
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(delay_secs)) => {}
                        _ = shutdown_wait.changed() => {
                            if *shutdown_wait.borrow() {
                                break;
                            }
                        }
                    }
                }
                first_run = false;

                // Create a fresh auth_status channel for reconnects (original one may be closed)
                let (reconnect_auth_tx, _reconnect_auth_rx) = mpsc::channel::<AuthStatus>(16);
                let auth_tx = if attempt == 0 { auth_status_tx.clone() } else { reconnect_auth_tx };

                let result = establish_and_hold(
                    config.clone(),
                    password.clone(),
                    shutdown_rx.clone(),
                    auth_tx,
                    status_tx.clone(),
                    session_holder.clone(),
                    known_hosts.clone(),
                    forward_shutdowns.clone(),
                    live_status_for_task.clone(),
                )
                .await;

                match &result {
                    Ok(()) => {
                        // Graceful shutdown (user requested) — do not reconnect
                        info!("Connection {} stopped gracefully", config.name);
                        // Only send Disconnected if not already sent by stop()
                        let already_disconnected = live_status_for_task
                            .lock()
                            .map(|s| s.0 == ConnectionStatus::Disconnected)
                            .unwrap_or(false);
                        if !already_disconnected {
                            if let Ok(mut s) = live_status_for_task.lock() {
                                *s = (ConnectionStatus::Disconnected, None, None);
                            }
                            let _ = status_tx
                                .send((config.id.clone(), ConnectionStatus::Disconnected, None))
                                .await;
                        }
                        let _ = audit
                            .append(&AuditEntry {
                                connection_id: config.id.clone(),
                                connection_name: config.name.clone(),
                                event: AuditEvent::Disconnected,
                                message: "Connection stopped".into(),
                                ts: chrono::Utc::now().to_rfc3339(),
                            })
                            .await;
                        break;
                    }
                    Err(e) => {
                        error!("Connection {} failed: {}", config.name, e);
                        let _ = audit
                            .append(&AuditEntry {
                                connection_id: config.id.clone(),
                                connection_name: config.name.clone(),
                                event: AuditEvent::Error,
                                message: e.to_string(),
                                ts: chrono::Utc::now().to_rfc3339(),
                            })
                            .await;

                        // If shutdown was requested during connection, don't retry
                        if *shutdown_rx_clone.borrow() {
                            let already_disconnected = live_status_for_task
                                .lock()
                                .map(|s| s.0 == ConnectionStatus::Disconnected)
                                .unwrap_or(false);
                            if !already_disconnected {
                                if let Ok(mut s) = live_status_for_task.lock() {
                                    *s = (ConnectionStatus::Disconnected, None, None);
                                }
                                let _ = status_tx
                                    .send((config.id.clone(), ConnectionStatus::Disconnected, None))
                                    .await;
                            }
                            break;
                        }

                        // If max_reconnect_attempts == 0, don't retry at all
                        if max_reconnect_attempts == 0 {
                            if let Ok(mut s) = live_status_for_task.lock() {
                                *s = (ConnectionStatus::Error, Some(e.to_string()), None);
                            }
                            let _ = status_tx
                                .send((config.id.clone(), ConnectionStatus::Error, Some(e.to_string())))
                                .await;
                            break;
                        }

                        // Otherwise loop to retry
                        continue;
                    }
                }
            }

            // Mark this connection as finished so the manager can clean up the handle
            finished_ids.lock().await.push(config.id.clone());
        });

        Ok(())
    }

    /// Stop a running connection (including all its forwards).
    pub async fn stop(&mut self, connection_id: &str) -> Result<()> {
        // Close all terminals for this connection
        self.terminals
            .retain(|_, h| h.connection_id != connection_id);

        if let Some(handle) = self.connections.remove(connection_id) {
            info!("Stopping connection {}", handle.config.name);
            // Stop all individual forwards
            let mut fwd_shutdowns = handle.forward_shutdowns.lock().await;
            for (rule_id, tx) in fwd_shutdowns.drain() {
                info!("Stopping forward {} for connection {}", rule_id, handle.config.name);
                let _ = tx.send(true);
            }
            drop(fwd_shutdowns);
            // Mark as disconnected immediately so the frontend gets instant feedback
            if let Ok(mut s) = handle.live_status.lock() {
                *s = (ConnectionStatus::Disconnected, None, None);
            }
            self.update_status(connection_id, ConnectionStatus::Disconnected, None)
                .await;
            // Signal the background task to stop
            let _ = handle.shutdown_tx.send(true);
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
    pub fn get_statuses(&self) -> HashMap<String, (ConnectionStatus, Option<String>, Option<u64>, Vec<String>)> {
        self.connections
            .iter()
            .map(|(id, h)| {
                let (status, error, connected_at) = h
                    .live_status
                    .lock()
                    .map(|g| (g.0, g.1.clone(), g.2))
                    .unwrap_or((ConnectionStatus::Disconnected, None, None));
                let uptime = connected_at.map(|t| t.elapsed().as_secs());
                let running_fwd_ids: Vec<String> = h
                    .forward_shutdowns
                    .try_lock()
                    .map(|g| g.keys().cloned().collect())
                    .unwrap_or_default();
                (id.clone(), (status, error, uptime, running_fwd_ids))
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

    /// Close a terminal. Returns the connection_id it belonged to.
    pub fn close_terminal(&mut self, terminal_id: &str) -> Result<String> {
        let handle = self.terminals
            .remove(terminal_id)
            .ok_or_else(|| anyhow!("Terminal {} not found", terminal_id))?;
        info!("Closed terminal {}", terminal_id);
        Ok(handle.connection_id)
    }

    /// Check if a connection should be auto-disconnected:
    /// no remaining terminals AND no running port forwards.
    pub fn should_auto_disconnect(&self, connection_id: &str) -> bool {
        let has_terminals = self.terminals.values().any(|h| h.connection_id == connection_id);
        if has_terminals {
            return false;
        }
        // Check if there are actual running forwards (not just config — real active listeners)
        if let Some(handle) = self.connections.get(connection_id) {
            match handle.forward_shutdowns.try_lock() {
                Ok(fwd) => {
                    if !fwd.is_empty() {
                        return false;
                    }
                }
                Err(_) => {
                    // Lock contended — conservatively assume forwards exist, don't auto-disconnect
                    return false;
                }
            }
        }
        true
    }

    /// Synchronize running forwards with the current config.
    /// Stops removed/disabled rules, starts new/enabled rules.
    pub async fn sync_forwards(&self, connection_id: &str, new_forwards: &[ForwardRule]) -> Result<()> {
        let handle = self.connections.get(connection_id)
            .ok_or_else(|| anyhow!("Connection {} is not running", connection_id))?;

        let session_guard = handle.session.lock().await;
        let session = session_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Connection {} has no active session", connection_id))?
            .clone();
        drop(session_guard);

        let mut fwd_shutdowns = handle.forward_shutdowns.lock().await;

        // Build set of rule IDs that should be running
        let desired: HashMap<&str, &ForwardRule> = new_forwards
            .iter()
            .filter(|f| f.enabled)
            .map(|f| (f.id.as_str(), f))
            .collect();

        // Stop forwards that are no longer desired
        let running_ids: Vec<String> = fwd_shutdowns.keys().cloned().collect();
        for id in &running_ids {
            if !desired.contains_key(id.as_str()) {
                if let Some(tx) = fwd_shutdowns.remove(id) {
                    info!("Hot-stop forward rule {}", id);
                    let _ = tx.send(true);
                }
            }
        }

        // Start forwards that are desired but not yet running
        for (rule_id, rule) in &desired {
            if !fwd_shutdowns.contains_key(*rule_id) {
                match spawn_single_forward(session.clone(), rule).await {
                    Ok(tx) => {
                        info!("Hot-start forward: localhost:{} -> {}:{} [{}]",
                            rule.local_port, rule.target_host, rule.target_port, rule.name);
                        fwd_shutdowns.insert(rule_id.to_string(), tx);
                    }
                    Err(e) => {
                        warn!("Failed to start forward {} (port {}): {}", rule.name, rule.local_port, e);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Establish SSH connection, authenticate, start initial forwards, then hold until shutdown.
async fn establish_and_hold(
    config: Connection,
    password: String,
    shutdown_rx: watch::Receiver<bool>,
    auth_status_tx: mpsc::Sender<AuthStatus>,
    status_tx: mpsc::Sender<(String, ConnectionStatus, Option<String>)>,
    session_holder: Arc<Mutex<Option<Arc<Handle<SshClient>>>>>,
    known_hosts: Arc<KnownHostsStore>,
    forward_shutdowns: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
    live_status: LiveStatus,
) -> Result<()> {
    let mut ssh_config = client::Config::default();
    ssh_config.keepalive_interval = Some(std::time::Duration::from_secs(KEEPALIVE_INTERVAL_SECS));
    ssh_config.keepalive_max = KEEPALIVE_MAX;
    let ssh_config = Arc::new(ssh_config);

    let host_port = format!("{}:{}", config.host, config.port);
    let handler = SshClient::new(host_port.clone(), known_hosts);

    // Connect to SSH host with timeout
    info!("Connecting to SSH host: {}", host_port);

    let mut session = tokio::time::timeout(
        std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS),
        client::connect(ssh_config, &host_port, handler),
    )
    .await
    .map_err(|_| anyhow!("Connection to {} timed out after {}s", host_port, CONNECT_TIMEOUT_SECS))?
    .with_context(|| format!("Failed to connect to {}", host_port))?;

    // Run authentication per RFC 4252
    let _ = status_tx
        .send((config.id.clone(), ConnectionStatus::Connecting, None))
        .await;

    let mut auth_handler = AuthHandler::new(password, config.private_key_path.clone(), auth_status_tx);
    auth_handler
        .run_auth(&mut session, &config.username)
        .await
        .context("Authentication failed")?;

    // Auth succeeded → mark as connected
    if let Ok(mut s) = live_status.lock() {
        *s = (ConnectionStatus::Connected, None, Some(std::time::Instant::now()));
    }
    let _ = status_tx
        .send((config.id.clone(), ConnectionStatus::Connected, None))
        .await;

    info!("Connection {} authenticated", config.name);

    // Wrap session in Arc and store for terminal/forward access
    let session = Arc::new(session);
    *session_holder.lock().await = Some(session.clone());

    // Start initial forwards (port conflict = skip that rule, not fail the connection)
    {
        let mut fwd_shutdowns = forward_shutdowns.lock().await;
        for rule in config.forwards.iter().filter(|f| f.enabled) {
            match spawn_single_forward(session.clone(), rule).await {
                Ok(tx) => {
                    fwd_shutdowns.insert(rule.id.clone(), tx);
                }
                Err(e) => {
                    warn!("Failed to start forward {} (port {}): {}, skipping",
                        rule.name, rule.local_port, e);
                }
            }
        }
    }

    // Hold connection alive until shutdown signal
    let mut shutdown_rx = shutdown_rx;
    let _ = shutdown_rx.changed().await;

    // Shut down all forwards
    {
        let mut fwd_shutdowns = forward_shutdowns.lock().await;
        for (_, tx) in fwd_shutdowns.drain() {
            let _ = tx.send(true);
        }
    }

    // Clean up session reference
    *session_holder.lock().await = None;

    Ok(())
}
