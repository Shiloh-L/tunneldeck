use anyhow::{anyhow, Context, Result};
use russh::client::{Handle, Prompt};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::client::SshClient;

/// Handles multi-round keyboard-interactive authentication.
/// Flow: password prompt → Duo Push prompt → success/failure.
pub struct AuthHandler {
    password: String,
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
        status_tx: mpsc::Sender<AuthStatus>,
    ) -> Self {
        Self {
            password,
            status_tx,
        }
    }

    /// Drive the keyboard-interactive auth flow.
    /// Returns Ok(()) on success, Err on failure.
    pub async fn run_auth(
        &mut self,
        session: &mut Handle<SshClient>,
        username: &str,
    ) -> Result<()> {
        // Initiate keyboard-interactive auth
        let auth_result = session
            .authenticate_keyboard_interactive_start(username, None)
            .await
            .context("Failed to start keyboard-interactive auth")?;

        self.handle_auth_result(session, username, auth_result)
            .await
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
