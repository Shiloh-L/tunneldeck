use anyhow::{anyhow, Context, Result};
use russh::client::{Handle, Prompt};
use ssh_key::private::PrivateKey;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::client::SshClient;

/// Handles SSH authentication per RFC 4252.
/// Flow: none (probe) → publickey → keyboard-interactive (supports 2FA) → password (fallback).
pub struct AuthHandler {
    password: String,
    /// Optional private key path for publickey auth
    private_key_path: Option<String>,
    /// Send auth status updates to UI
    status_tx: mpsc::Sender<AuthStatus>,
}

#[derive(Debug, Clone)]
pub enum AuthStatus {
    PromptingPassword,
    WaitingDuoPush,
    Success,
    Failed(String),
}

impl AuthHandler {
    pub fn new(
        password: String,
        private_key_path: Option<String>,
        status_tx: mpsc::Sender<AuthStatus>,
    ) -> Self {
        Self {
            password,
            private_key_path,
            status_tx,
        }
    }

    /// Drive SSH authentication per RFC 4252.
    /// 1. Send "none" to probe server-supported methods.
    /// 2. Try publickey if a private key is provided.
    /// 3. Try keyboard-interactive (supports 2FA/Duo Push).
    /// 4. Fall back to password auth.
    pub async fn run_auth(
        &mut self,
        session: &mut Handle<SshClient>,
        username: &str,
    ) -> Result<()> {
        // Step 1: "none" auth — per RFC 4252 §5.2
        debug!("Sending 'none' auth request to probe server methods");
        match session.authenticate_none(username).await {
            Ok(true) => {
                info!("Server accepted 'none' authentication");
                let _ = self.status_tx.send(AuthStatus::Success).await;
                return Ok(());
            }
            Ok(false) => {
                debug!("'none' auth rejected (expected), proceeding with real auth methods");
            }
            Err(e) => {
                debug!("'none' auth probe error: {}, proceeding anyway", e);
            }
        }

        // Step 2: Try publickey auth if private key is provided (RFC 4252 §7)
        if let Some(ref key_path) = self.private_key_path {
            debug!("Trying publickey authentication with key: {}", key_path);
            match self.try_publickey_auth(session, username, key_path).await {
                Ok(true) => {
                    info!("Publickey authentication successful");
                    let _ = self.status_tx.send(AuthStatus::Success).await;
                    return Ok(());
                }
                Ok(false) => {
                    debug!("Publickey auth rejected, trying next method");
                }
                Err(e) => {
                    debug!("Publickey auth error: {}, trying next method", e);
                }
            }
        }

        // Step 3: Try keyboard-interactive (higher priority — supports 2FA/MFA)
        debug!("Trying keyboard-interactive authentication for {}", username);
        let _ = self.status_tx.send(AuthStatus::PromptingPassword).await;

        match session
            .authenticate_keyboard_interactive_start(username, None)
            .await
        {
            Ok(auth_result) => {
                match self.handle_auth_result(session, username, auth_result).await {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        debug!("Keyboard-interactive failed: {}, trying password auth", e);
                    }
                }
            }
            Err(e) => {
                debug!("Keyboard-interactive start error: {}, trying password auth", e);
            }
        }

        // Step 4: Fall back to password auth
        debug!("Trying password authentication for {}", username);
        match session.authenticate_password(username, &self.password).await {
            Ok(true) => {
                info!("Password authentication successful");
                let _ = self.status_tx.send(AuthStatus::Success).await;
                return Ok(());
            }
            Ok(false) => {
                let msg = "All authentication methods failed".to_string();
                warn!("{}", msg);
                let _ = self.status_tx.send(AuthStatus::Failed(msg.clone())).await;
                return Err(anyhow!(msg));
            }
            Err(e) => {
                let msg = format!("Password authentication error: {}", e);
                warn!("{}", msg);
                let _ = self.status_tx.send(AuthStatus::Failed(msg.clone())).await;
                return Err(anyhow!(msg));
            }
        }
    }

    /// Attempt publickey authentication using the given private key file.
    /// The password field is used as the key passphrase if the key is encrypted.
    async fn try_publickey_auth(
        &self,
        session: &mut Handle<SshClient>,
        username: &str,
        key_path: &str,
    ) -> Result<bool> {
        let path = Path::new(key_path);
        if !path.exists() {
            return Err(anyhow!("Private key file not found: {}", key_path));
        }

        let key_data = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read private key file")?;

        // Parse the key (may be encrypted or unencrypted)
        let parsed_key = PrivateKey::from_openssh(&key_data)
            .context("Failed to parse private key file")?;

        // If encrypted, decrypt with password as passphrase
        let private_key = if parsed_key.is_encrypted() {
            if self.password.is_empty() {
                return Err(anyhow!("Private key is encrypted but no passphrase provided"));
            }
            parsed_key.decrypt(self.password.as_bytes())
                .context("Failed to decrypt private key (wrong passphrase?)")?
        } else {
            parsed_key
        };

        let result = session
            .authenticate_publickey(username, Arc::new(private_key))
            .await
            .context("Publickey authentication request failed")?;

        Ok(result)
    }

    async fn handle_auth_result(
        &mut self,
        session: &mut Handle<SshClient>,
        _username: &str,
        mut result: russh::client::KeyboardInteractiveAuthResponse,
    ) -> Result<()> {
        loop {
            match &result {
                russh::client::KeyboardInteractiveAuthResponse::Success => {
                    info!("SSH authentication successful");
                    let _ = self.status_tx.send(AuthStatus::Success).await;
                    return Ok(());
                }
                russh::client::KeyboardInteractiveAuthResponse::Failure => {
                    let msg = "Authentication rejected by server".to_string();
                    warn!("{}", msg);
                    let _ = self.status_tx.send(AuthStatus::Failed(msg.clone())).await;
                    return Err(anyhow!(msg));
                }
                russh::client::KeyboardInteractiveAuthResponse::InfoRequest {
                    name,
                    instructions,
                    prompts,
                } => {
                    debug!(
                        "Auth info request: name={:?}, instructions={:?}, prompts count={}",
                        name,
                        instructions,
                        prompts.len()
                    );

                    let responses = self.generate_responses(prompts).await?;

                    result = session
                        .authenticate_keyboard_interactive_respond(responses)
                        .await
                        .context("Failed to respond to keyboard-interactive prompt")?;
                }
            }
        }
    }

    /// Generate responses based on prompt content.
    /// - Password prompt → send stored password
    /// - Duo/2FA prompt → send "1" to trigger push, notify UI to wait
    async fn generate_responses(
        &mut self,
        prompts: &[Prompt],
    ) -> Result<Vec<String>> {
        let mut responses = Vec::with_capacity(prompts.len());

        for p in prompts {
            let prompt_lower = p.prompt.to_lowercase();

            if prompt_lower.contains("password") {
                debug!("Detected password prompt, sending stored password");
                let _ = self.status_tx.send(AuthStatus::PromptingPassword).await;
                responses.push(self.password.clone());
            } else if prompt_lower.contains("duo")
                || prompt_lower.contains("factor")
                || prompt_lower.contains("push")
                || prompt_lower.contains("passcode")
                || prompt_lower.contains("option")
                || prompt_lower.is_empty()
            {
                // Duo Push: send "1" to select push option, or empty string
                debug!("Detected Duo/2FA prompt: {:?}, sending push trigger", p.prompt);
                let _ = self.status_tx.send(AuthStatus::WaitingDuoPush).await;
                // "1" is typically the Duo Push option
                responses.push("1".to_string());
            } else {
                // Unknown prompt - try empty response
                debug!("Unknown prompt: {:?}, sending empty response", p.prompt);
                responses.push(String::new());
            }
        }

        Ok(responses)
    }
}
