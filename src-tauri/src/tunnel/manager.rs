use anyhow::{anyhow, Context, Result};
use russh::client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{error, info};

use crate::ssh::auth::{AuthHandler, AuthStatus};
use crate::ssh::client::SshClient;
use crate::ssh::tunnel::start_local_forward;
use crate::store::audit_logger::AuditLogger;
use crate::tunnel::types::*;

/// Handle to a running tunnel, used to stop it.
struct TunnelHandle {
    config: TunnelConfig,
    shutdown_tx: watch::Sender<bool>,
    status: TunnelStatus,
    error_message: Option<String>,
    connected_at: Option<std::time::Instant>,
}

/// Manages all tunnel lifecycles.
pub struct TunnelManager {
    tunnels: HashMap<String, TunnelHandle>,
    audit: Arc<AuditLogger>,
    /// Channel to send status updates to the frontend.
    status_tx: mpsc::Sender<(String, TunnelStatus, Option<String>)>,
}

impl TunnelManager {
    pub fn new(
        audit: Arc<AuditLogger>,
        status_tx: mpsc::Sender<(String, TunnelStatus, Option<String>)>,
    ) -> Self {
        Self {
            tunnels: HashMap::new(),
            audit,
            status_tx,
        }
    }

    /// Start a tunnel. Returns immediately; the tunnel runs in the background.
    pub async fn start(
        &mut self,
        config: TunnelConfig,
        password: String,
        auth_status_tx: mpsc::Sender<AuthStatus>,
    ) -> Result<()> {
        let id = config.id.clone();

        if self.tunnels.contains_key(&id) {
            return Err(anyhow!("Tunnel {} is already running", config.name));
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Mark as connecting
        self.update_status(&id, TunnelStatus::Connecting, None).await;

        let handle = TunnelHandle {
            config: config.clone(),
            shutdown_tx,
            status: TunnelStatus::Connecting,
            error_message: None,
            connected_at: None,
        };
        self.tunnels.insert(id.clone(), handle);

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
                    info!("Tunnel {} stopped gracefully", config.name);
                    let _ = status_tx
                        .send((config.id.clone(), TunnelStatus::Disconnected, None))
                        .await;
                    let _ = audit
                        .append(&AuditEntry {
                            tunnel_id: config.id.clone(),
                            tunnel_name: config.name.clone(),
                            event: AuditEvent::Disconnected,
                            message: "Tunnel stopped".into(),
                            ts: chrono::Utc::now().to_rfc3339(),
                        })
                        .await;
                }
                Err(e) => {
                    error!("Tunnel {} failed: {}", config.name, e);
                    let _ = status_tx
                        .send((
                            config.id.clone(),
                            TunnelStatus::Error,
                            Some(e.to_string()),
                        ))
                        .await;
                    let _ = audit
                        .append(&AuditEntry {
                            tunnel_id: config.id.clone(),
                            tunnel_name: config.name.clone(),
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

    /// Stop a running tunnel.
    pub async fn stop(&mut self, tunnel_id: &str) -> Result<()> {
        if let Some(handle) = self.tunnels.remove(tunnel_id) {
            info!("Stopping tunnel {}", handle.config.name);
            let _ = handle.shutdown_tx.send(true);
            self.update_status(tunnel_id, TunnelStatus::Disconnected, None)
                .await;
            Ok(())
        } else {
            Err(anyhow!("Tunnel {} is not running", tunnel_id))
        }
    }

    /// Stop all running tunnels.
    pub async fn stop_all(&mut self) {
        let ids: Vec<String> = self.tunnels.keys().cloned().collect();
        for id in ids {
            let _ = self.stop(&id).await;
        }
    }

    /// Get the status of a specific tunnel.
    pub fn get_status(&self, tunnel_id: &str) -> TunnelStatus {
        self.tunnels
            .get(tunnel_id)
            .map(|h| h.status)
            .unwrap_or(TunnelStatus::Disconnected)
    }

    /// Get info for all tunnels.
    pub fn get_statuses(&self) -> HashMap<String, (TunnelStatus, Option<String>, Option<u64>)> {
        self.tunnels
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
        tunnel_id: &str,
        status: TunnelStatus,
        error: Option<String>,
    ) {
        let _ = self
            .status_tx
            .send((tunnel_id.to_string(), status, error))
            .await;
    }
}

/// The actual connection + forwarding logic, runs in a spawned task.
async fn connect_and_forward(
    config: TunnelConfig,
    password: String,
    shutdown_rx: watch::Receiver<bool>,
    auth_status_tx: mpsc::Sender<AuthStatus>,
    status_tx: mpsc::Sender<(String, TunnelStatus, Option<String>)>,
) -> Result<()> {
    let ssh_config = client::Config::default();
    let ssh_config = Arc::new(ssh_config);

    let handler = SshClient::new();

    // Connect to jump host
    let addr = format!("{}:{}", config.jump_host, config.jump_port);
    info!("Connecting to SSH host: {}", addr);

    let mut session = client::connect(ssh_config, &addr, handler)
        .await
        .with_context(|| format!("Failed to connect to {}", addr))?;

    // Run keyboard-interactive authentication (password + Duo Push)
    let _ = status_tx
        .send((config.id.clone(), TunnelStatus::Connecting, None))
        .await;

    let mut auth_handler = AuthHandler::new(password, auth_status_tx);
    auth_handler
        .run_auth(&mut session, &config.username)
        .await
        .context("Authentication failed")?;

    // Auth succeeded → mark as connected
    let _ = status_tx
        .send((config.id.clone(), TunnelStatus::Connected, None))
        .await;

    info!(
        "Tunnel {} connected: localhost:{} -> {}:{}",
        config.name, config.local_port, config.target_host, config.target_port
    );

    // Start local port forwarding
    start_local_forward(
        session,
        config.local_port,
        config.target_host.clone(),
        config.target_port,
        shutdown_rx,
    )
    .await?;

    Ok(())
}
