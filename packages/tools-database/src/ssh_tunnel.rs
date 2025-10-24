//! SSH tunnel management for secure database connections
//!
//! This module implements SSH port forwarding to enable secure database connections
//! through bastion hosts. It creates a local TCP listener that forwards connections
//! through SSH channels to the target database server.

use crate::error::DatabaseError;
use ssh2::Session;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant, sleep, timeout};

/// RAII guard to ensure connection counter decrements on all exit paths
struct ConnectionGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

/// SSH authentication method
#[derive(Clone)]
pub enum SSHAuth {
    /// Password authentication
    Password(String),
    /// SSH key authentication with optional passphrase
    Key {
        path: PathBuf,
        passphrase: Option<String>,
    },
}

// Custom Debug that hides sensitive data
impl std::fmt::Debug for SSHAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SSHAuth::Password(_) => write!(f, "Password([REDACTED])"),
            SSHAuth::Key { path, .. } => {
                write!(f, "Key {{ path: {:?}, passphrase: [REDACTED] }}", path)
            }
        }
    }
}

/// SSH connection configuration
#[derive(Debug, Clone)]
pub struct SSHConfig {
    /// SSH server hostname
    pub host: String,
    /// SSH server port (typically 22)
    pub port: u16,
    /// SSH username
    pub username: String,
    /// Authentication method
    pub auth: SSHAuth,
}

/// Tunnel target configuration
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Target database host (from SSH server's perspective)
    pub target_host: String,
    /// Target database port
    pub target_port: u16,
}

/// SSH tunnel with local port forwarding
pub struct SSHTunnel {
    /// Shared SSH session for creating channels
    #[allow(dead_code)]
    session: Arc<Mutex<Session>>,
    /// Local port where tunnel is listening
    local_port: u16,
    /// Target database host (from SSH server's perspective)
    #[allow(dead_code)]
    target_host: String,
    /// Target database port
    #[allow(dead_code)]
    target_port: u16,
    /// Shutdown signal sender
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    /// Background listener task handle
    listener_task: Option<JoinHandle<()>>,
    /// Track active connections for graceful shutdown
    active_connections: Arc<AtomicUsize>,
}

/// Establish SSH session and authenticate
async fn establish_ssh_session(config: SSHConfig) -> Result<Session, DatabaseError> {
    // Connect to SSH server (async)
    let tcp_stream = tokio::net::TcpStream::connect((config.host.as_str(), config.port))
        .await
        .map_err(|e| {
            DatabaseError::SSHTunnelError(format!(
                "Failed to connect to SSH host {}:{}: {}",
                config.host, config.port, e
            ))
        })?;

    // Convert to std::net::TcpStream for ssh2
    let std_stream = tcp_stream.into_std().map_err(|e| {
        DatabaseError::SSHTunnelError(format!("Failed to convert TcpStream: {}", e))
    })?;

    // SSH operations in blocking context
    let session = tokio::task::spawn_blocking(move || -> Result<Session, DatabaseError> {
        let mut sess = Session::new().map_err(|e| {
            DatabaseError::SSHTunnelError(format!("Failed to create SSH session: {}", e))
        })?;

        // Attach TCP stream
        sess.set_tcp_stream(std_stream);

        // Perform SSH handshake
        sess.handshake()
            .map_err(|e| DatabaseError::SSHTunnelError(format!("SSH handshake failed: {}", e)))?;

        // Authenticate based on config
        match config.auth {
            SSHAuth::Password(ref password) => {
                sess.userauth_password(&config.username, password)
                    .map_err(|e| {
                        DatabaseError::SSHTunnelError(format!(
                            "SSH password authentication failed: {}",
                            e
                        ))
                    })?;
            }
            SSHAuth::Key {
                ref path,
                ref passphrase,
            } => {
                sess.userauth_pubkey_file(
                    &config.username,
                    None, // public key path (optional)
                    path.as_path(),
                    passphrase.as_deref(),
                )
                .map_err(|e| {
                    DatabaseError::SSHTunnelError(format!("SSH key authentication failed: {}", e))
                })?;
            }
        }

        // Verify authentication
        if !sess.authenticated() {
            return Err(DatabaseError::SSHTunnelError(
                "SSH authentication failed".to_string(),
            ));
        }

        Ok(sess)
    })
    .await
    .map_err(|e| DatabaseError::SSHTunnelError(format!("SSH session task panicked: {}", e)))??;

    Ok(session)
}

/// Handle a single tunnel connection
async fn handle_tunnel_connection(
    local_stream: tokio::net::TcpStream,
    session: Arc<Mutex<Session>>,
    target_host: String,
    target_port: u16,
    active_connections: Arc<AtomicUsize>,
) -> Result<(), DatabaseError> {
    // Increment counter at start
    active_connections.fetch_add(1, Ordering::Relaxed);

    // Ensure decrement on all exit paths
    let _guard = ConnectionGuard {
        counter: active_connections.clone(),
    };

    // Create SSH channel in blocking context
    let channel = {
        let session_clone = session.clone();
        let target_host_clone = target_host.clone();

        tokio::task::spawn_blocking(move || -> Result<ssh2::Channel, DatabaseError> {
            let session_lock = session_clone.lock().map_err(|e| {
                DatabaseError::SSHTunnelError(format!("Failed to lock session: {}", e))
            })?;

            session_lock
                .channel_direct_tcpip(&target_host_clone, target_port, None)
                .map_err(|e| {
                    DatabaseError::SSHTunnelError(format!("Failed to create SSH channel: {}", e))
                })
        })
        .await
        .map_err(|e| DatabaseError::SSHTunnelError(format!("Channel task panicked: {}", e)))??
    };

    // Copy data bidirectionally in blocking context
    tokio::task::spawn_blocking(move || {
        use std::io::{Read, Write};
        use std::thread;

        // Convert async TcpStream to std::net::TcpStream
        let std_stream = local_stream.into_std().map_err(|e| {
            DatabaseError::SSHTunnelError(format!("Failed to convert stream: {}", e))
        })?;

        std_stream.set_nonblocking(false).map_err(|e| {
            DatabaseError::SSHTunnelError(format!("Failed to set blocking mode: {}", e))
        })?;

        // Bidirectional copy using threads
        let stream_read = std_stream
            .try_clone()
            .map_err(|e| DatabaseError::SSHTunnelError(format!("Failed to clone stream: {}", e)))?;
        let stream_write = std_stream;

        // Split channel for bidirectional communication
        // Clone the channel for both directions (ssh2::Channel is Clone)
        let mut channel_read = channel.clone();
        let mut channel_write = channel;

        // Stream -> Channel
        let handle1 = thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            let mut stream_read = stream_read;
            loop {
                match stream_read.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if channel_write.write_all(&buffer[..n]).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Channel -> Stream
        let handle2 = thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            let mut stream_write = stream_write;
            loop {
                match channel_read.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if stream_write.write_all(&buffer[..n]).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Wait for both directions to complete
        let _ = handle1.join();
        let _ = handle2.join();

        Ok::<(), DatabaseError>(())
    })
    .await
    .map_err(|e| DatabaseError::SSHTunnelError(format!("Tunnel copy task panicked: {}", e)))??;

    Ok(())
}

/// Start local port forwarder
async fn start_port_forwarder(
    session: Arc<Mutex<Session>>,
    target_host: String,
    target_port: u16,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    active_connections: Arc<AtomicUsize>,
) -> Result<(u16, JoinHandle<()>), DatabaseError> {
    // Bind to localhost with auto-assigned port
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
        DatabaseError::SSHTunnelError(format!("Failed to bind local listener: {}", e))
    })?;

    let local_addr = listener.local_addr().map_err(|e| {
        DatabaseError::SSHTunnelError(format!("Failed to get local address: {}", e))
    })?;
    let local_port = local_addr.port();

    // Spawn background task to accept connections
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    break;
                }
                // Accept new connection
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let session = session.clone();
                            let target_host = target_host.clone();
                            let conn_counter = active_connections.clone();

                            // Spawn task to handle this connection
                            tokio::spawn(async move {
                                if let Err(e) = handle_tunnel_connection(
                                    stream,
                                    session,
                                    target_host,
                                    target_port,
                                    conn_counter,
                                )
                                .await
                                {
                                    eprintln!("Tunnel connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Failed to accept connection: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    Ok((local_port, handle))
}

/// Establish an SSH tunnel with port forwarding
///
/// This function creates an SSH connection to a bastion host and sets up
/// local port forwarding to a target database server. Returns a tunnel
/// instance that manages the connection lifecycle.
pub async fn establish_tunnel(
    ssh_config: SSHConfig,
    tunnel_config: TunnelConfig,
) -> Result<SSHTunnel, DatabaseError> {
    // Validate configuration
    if ssh_config.host.is_empty() {
        return Err(DatabaseError::SSHTunnelError(
            "SSH host cannot be empty".to_string(),
        ));
    }
    if ssh_config.username.is_empty() {
        return Err(DatabaseError::SSHTunnelError(
            "SSH username cannot be empty".to_string(),
        ));
    }
    if tunnel_config.target_host.is_empty() {
        return Err(DatabaseError::SSHTunnelError(
            "Target host cannot be empty".to_string(),
        ));
    }

    // Establish SSH session with timeout
    let session = timeout(Duration::from_secs(30), establish_ssh_session(ssh_config))
        .await
        .map_err(|_| {
            DatabaseError::SSHTunnelError("SSH connection timeout (30 seconds)".to_string())
        })??;

    // Wrap session for sharing
    let session = Arc::new(Mutex::new(session));

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    // Initialize connection counter
    let active_connections = Arc::new(AtomicUsize::new(0));

    // Start port forwarder
    let (local_port, listener_task) = start_port_forwarder(
        session.clone(),
        tunnel_config.target_host.clone(),
        tunnel_config.target_port,
        shutdown_rx,
        active_connections.clone(),
    )
    .await?;

    Ok(SSHTunnel {
        session,
        local_port,
        target_host: tunnel_config.target_host,
        target_port: tunnel_config.target_port,
        shutdown_tx,
        listener_task: Some(listener_task),
        active_connections,
    })
}

impl SSHTunnel {
    /// Get the local port where the tunnel is listening
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Check if tunnel is still active
    pub fn is_connected(&self) -> bool {
        self.listener_task
            .as_ref()
            .map(|task| !task.is_finished())
            .unwrap_or(false)
    }

    /// Close the tunnel gracefully and wait for cleanup
    ///
    /// This method:
    /// 1. Sends shutdown signal to listener
    /// 2. Waits for active connections to drain (with timeout)
    /// 3. Waits for listener task to finish (with timeout)
    ///
    /// Users should always call this method explicitly for guaranteed cleanup.
    /// If not called, Drop will attempt best-effort cleanup in background.
    pub async fn close(mut self) {
        // Send shutdown signal to stop accepting new connections
        let _ = self.shutdown_tx.send(());

        // Wait for active connections to drain (max 30 seconds)
        let drain_start = Instant::now();
        while self.active_connections.load(Ordering::Relaxed) > 0 {
            if drain_start.elapsed() > Duration::from_secs(30) {
                log::warn!(
                    "Timeout waiting for {} active connections to drain after 30s",
                    self.active_connections.load(Ordering::Relaxed)
                );
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }

        // Wait for listener task to finish (max 5 seconds)
        if let Some(task) = self.listener_task.take() {
            match timeout(Duration::from_secs(5), task).await {
                Ok(Ok(())) => {
                    log::debug!("SSH tunnel listener task closed cleanly");
                }
                Ok(Err(e)) => {
                    log::error!("SSH tunnel listener task panicked: {:?}", e);
                }
                Err(_) => {
                    log::error!(
                        "SSH tunnel listener task timeout after 5s - task may still be running"
                    );
                }
            }
        }

        // SSH session will be dropped automatically via Arc
    }
}

impl Drop for SSHTunnel {
    fn drop(&mut self) {
        // Best-effort cleanup: send shutdown signal
        let _ = self.shutdown_tx.send(());

        // If task still exists, spawn detached cleanup task
        if self.listener_task.is_some() {
            log::warn!(
                "SSHTunnel dropped without calling close() - spawning background cleanup task. \
                 Consider calling .close().await for guaranteed cleanup."
            );

            // Try to spawn cleanup task (may fail if runtime shutting down)
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let task = self.listener_task.take();
                handle.spawn(async move {
                    if let Some(t) = task {
                        // Give task 5 seconds to finish
                        let _ = tokio::time::timeout(Duration::from_secs(5), t).await;
                    }
                });
            }
        }
    }
}
