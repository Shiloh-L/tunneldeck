use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{warn, error};

use crate::tunnel::types::*;

/// Health checker: periodically checks tunnel status and triggers reconnection.
pub struct HealthChecker {
    interval: Duration,
    max_attempts: u32,
}

impl HealthChecker {
    pub fn new(interval_secs: u64, max_attempts: u32) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs),
            max_attempts,
        }
    }

    /// Monitor a tunnel's status. If it detects disconnection, signals for reconnection.
    /// This is a placeholder that will be connected to the TunnelManager's status stream.
    pub async fn run(
        &self,
        tunnel_id: String,
        mut status_rx: mpsc::Receiver<(String, TunnelStatus, Option<String>)>,
        reconnect_tx: mpsc::Sender<String>,
    ) {
        let mut attempts = 0u32;

        loop {
            tokio::select! {
                Some((id, status, _err)) = status_rx.recv() => {
                    if id != tunnel_id {
                        continue;
                    }
                    match status {
                        TunnelStatus::Connected => {
                            attempts = 0; // Reset on successful connection
                        }
                        TunnelStatus::Error | TunnelStatus::Disconnected => {
                            if attempts < self.max_attempts {
                                attempts += 1;
                                let delay = self.backoff_delay(attempts);
                                warn!(
                                    "Tunnel {} lost connection (attempt {}/{}), reconnecting in {:?}",
                                    tunnel_id, attempts, self.max_attempts, delay
                                );
                                tokio::time::sleep(delay).await;
                                let _ = reconnect_tx.send(tunnel_id.clone()).await;
                            } else {
                                error!(
                                    "Tunnel {} exceeded max reconnect attempts ({})",
                                    tunnel_id, self.max_attempts
                                );
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(self.interval) => {
                    // Periodic keepalive check would go here
                    // For now, we rely on the SSH connection's own keepalive
                }
            }
        }
    }

    /// Exponential backoff with jitter, capped at 60 seconds.
    fn backoff_delay(&self, attempt: u32) -> Duration {
        let base = Duration::from_secs(2u64.pow(attempt.min(5)));
        let max = Duration::from_secs(60);
        std::cmp::min(base, max)
    }
}
