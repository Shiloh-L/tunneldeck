use async_trait::async_trait;
use russh::client;
use ssh_key::PublicKey;
use std::sync::Arc;
use tracing::{debug, error};

use crate::store::known_hosts::{HostKeyCheckResult, KnownHostsStore};

/// The russh client handler.
/// Verifies server host keys using TOFU (Trust On First Use) via KnownHostsStore.
pub struct SshClient {
    host_port: String,
    known_hosts: Arc<KnownHostsStore>,
}

impl SshClient {
    pub fn new(host_port: String, known_hosts: Arc<KnownHostsStore>) -> Self {
        Self {
            host_port,
            known_hosts,
        }
    }
}

#[async_trait]
impl client::Handler for SshClient {
    type Error = anyhow::Error;

    /// Verify server host key using TOFU mechanism.
    /// - New host: accept and store key.
    /// - Known host, key matches: accept.
    /// - Known host, key changed: REJECT (possible MITM).
    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        match self
            .known_hosts
            .check_host_key(&self.host_port, server_public_key)
            .await
        {
            Ok(HostKeyCheckResult::TrustedNew) => {
                debug!("New host key accepted for {}", self.host_port);
                Ok(true)
            }
            Ok(HostKeyCheckResult::TrustedKnown) => {
                debug!("Known host key verified for {}", self.host_port);
                Ok(true)
            }
            Ok(HostKeyCheckResult::Mismatch { .. }) => {
                error!(
                    "HOST KEY VERIFICATION FAILED for {}! Server key has changed. \
                     This could indicate a man-in-the-middle attack.",
                    self.host_port
                );
                Ok(false)
            }
            Err(e) => {
                error!("Host key check error for {}: {}", self.host_port, e);
                Ok(false)
            }
        }
    }
}
