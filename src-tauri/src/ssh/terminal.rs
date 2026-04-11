use anyhow::{Context, Result};
use base64::Engine;
use russh::client::Handle;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use super::client::SshClient;

pub enum TerminalInput {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

/// Spawn a terminal session on the SSH connection.
/// Returns an UnboundedSender for sending input to the terminal.
pub fn spawn_terminal(
    terminal_id: String,
    connection_id: String,
    session: Arc<Handle<SshClient>>,
    cols: u32,
    rows: u32,
    app: AppHandle,
) -> mpsc::UnboundedSender<TerminalInput> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        if let Err(e) = run_terminal(
            terminal_id.clone(),
            session,
            cols,
            rows,
            rx,
            app.clone(),
        )
        .await
        {
            error!("Terminal {} error: {}", terminal_id, e);
        }

        let _ = app.emit(
            "terminal-exit",
            serde_json::json!({
                "terminalId": terminal_id,
                "connectionId": connection_id,
            }),
        );
        info!("Terminal {} closed", terminal_id);
    });

    tx
}

async fn run_terminal(
    terminal_id: String,
    session: Arc<Handle<SshClient>>,
    cols: u32,
    rows: u32,
    mut input_rx: mpsc::UnboundedReceiver<TerminalInput>,
    app: AppHandle,
) -> Result<()> {
    let mut channel = session
        .channel_open_session()
        .await
        .context("Failed to open session channel")?;

    channel
        .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
        .await
        .context("Failed to request PTY")?;

    channel
        .request_shell(false)
        .await
        .context("Failed to request shell")?;

    debug!("Terminal {} ready ({}x{})", terminal_id, cols, rows);

    loop {
        tokio::select! {
            msg = channel.wait() => {
                match msg {
                    Some(russh::ChannelMsg::Data { data }) => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                        let _ = app.emit("terminal-data", serde_json::json!({
                            "terminalId": terminal_id,
                            "data": b64,
                        }));
                    }
                    Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                        let _ = app.emit("terminal-data", serde_json::json!({
                            "terminalId": terminal_id,
                            "data": b64,
                        }));
                    }
                    Some(russh::ChannelMsg::Eof) | Some(russh::ChannelMsg::Close) | None => {
                        debug!("Terminal {} channel closed", terminal_id);
                        break;
                    }
                    _ => {}
                }
            }
            input = input_rx.recv() => {
                match input {
                    Some(TerminalInput::Data(data)) => {
                        channel.data(&data[..]).await
                            .context("Failed to send data to terminal")?;
                    }
                    Some(TerminalInput::Resize { cols, rows }) => {
                        channel.window_change(cols, rows, 0, 0).await
                            .context("Failed to resize terminal")?;
                    }
                    None => {
                        debug!("Terminal {} input channel closed", terminal_id);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
