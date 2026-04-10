use anyhow::{Context, Result};
use russh::client::Handle;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{debug, error, info};

use super::client::SshClient;

/// Start a local port-forwarding tunnel.
/// Binds `local_port` on localhost and forwards all connections through the SSH
/// session to `target_host:target_port` via direct-tcpip.
///
/// Returns a shutdown sender: drop it or send () to stop the tunnel.
pub async fn start_local_forward(
    session: Handle<SshClient>,
    local_port: u16,
    target_host: String,
    target_port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let bind_addr: SocketAddr = ([127, 0, 0, 1], local_port).into();
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind local port {}", local_port))?;

    info!(
        "Local forward: localhost:{} -> {}:{} via SSH",
        local_port, target_host, target_port
    );

    // Wrap session in Arc so we can share across spawned tasks
    let session = Arc::new(session);

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
                    info!("Shutting down local forward on port {}", local_port);
                    break;
                }
            }
        }
    }

    Ok(())
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
