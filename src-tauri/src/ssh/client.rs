use async_trait::async_trait;
use russh::client;
use ssh_key::PublicKey;
use tracing::debug;

/// The russh client handler. Minimal: just accepts host keys.
/// Keyboard-interactive auth is driven via Handle methods in auth.rs.
pub struct SshClient;

impl SshClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl client::Handler for SshClient {
    type Error = anyhow::Error;

    /// Accept all server host keys.
    /// TODO: implement known_hosts verification for production.
    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        debug!("Accepting server host key");
        Ok(true)
    }
}
