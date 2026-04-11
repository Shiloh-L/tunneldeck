use anyhow::{Context, Result};
use russh::client::Handle;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{debug, error, info};

use super::client::SshClient;
use crate::connection::types::ForwardRule;

/// Spawn a single port-forward listener with its own shutdown channel.
/// Returns the shutdown sender. The listener runs in a background task.
pub async fn spawn_single_forward(
    session: Arc<Handle<SshClient>>,
    rule: &ForwardRule,
) -> Result<watch::Sender<bool>> {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let bind_addr: SocketAddr = ([127, 0, 0, 1], rule.local_port).into();
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind local port {} ({})", rule.local_port, rule.name))?;

    info!(
        "Forward: localhost:{} -> {}:{} [{}]",
        rule.local_port, rule.target_host, rule.target_port, rule.name
    );

    let target_host = rule.target_host.clone();
    let target_port = rule.target_port;
    let local_port = rule.local_port;

    tokio::spawn(async move {
        accept_loop(listener, session, target_host, target_port, local_port, shutdown_rx).await;
    });

    Ok(shutdown_tx)
}

/// Accept loop for a single forward rule.
async fn accept_loop(
    listener: TcpListener,
    session: Arc<Handle<SshClient>>,
    target_host: String,
    target_port: u16,
    local_port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((tcp_stream, peer_addr)) => {
                        debug!("New connection from {} on local port {}", peer_addr, local_port);
                        let session = session.clone();
                        let target_host = target_host.clone();
                        let shutdown = shutdown_rx.clone();

                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(
                                &session,
                                tcp_stream,
                                &target_host,
                                target_port,
                                peer_addr,
                                shutdown,
                            )
                            .await
                            {
                                error!("Connection from {} failed: {}", peer_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection on port {}: {}", local_port, e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutting down forward on port {}", local_port);
                    break;
                }
            }
        }
    }
}

/// Handle a single TCP connection: open direct-tcpip channel and bidirectionally
/// copy data between the local TCP stream and the SSH channel.
async fn handle_connection(
    session: &Handle<SshClient>,
    tcp_stream: tokio::net::TcpStream,
    target_host: &str,
    target_port: u16,
    peer_addr: SocketAddr,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    // Open a direct-tcpip channel to the target through the SSH connection
    let channel = session
        .channel_open_direct_tcpip(
            target_host,
            target_port as u32,
            &peer_addr.ip().to_string(),
            peer_addr.port() as u32,
        )
        .await
        .context("Failed to open direct-tcpip channel")?;

    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();
    let channel_stream = channel.into_stream();

    // Bidirectional copy between TCP and SSH channel
    let (mut chan_read, mut chan_write) = tokio::io::split(channel_stream);

    tokio::select! {
        result = tokio::io::copy(&mut tcp_read, &mut chan_write) => {
            debug!("TCP->SSH copy finished for {}: {:?}", peer_addr, result);
        }
        result = tokio::io::copy(&mut chan_read, &mut tcp_write) => {
            debug!("SSH->TCP copy finished for {}: {:?}", peer_addr, result);
        }
        _ = shutdown_rx.changed() => {
            debug!("Connection to {} shut down", peer_addr);
        }
    }

    Ok(())
}
