// packages/daemon/src/service/kodegen_sse.rs
use crate::config::CategoryServerConfig;
use anyhow::Result;
use std::path::PathBuf;
use tokio::process::Child;

pub struct KodegenSseService {
    servers: Vec<CategoryServer>,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
}

struct CategoryServer {
    name: String,
    binary: String,
    port: u16,
    enabled: bool,
    process: Option<Child>,
}

impl KodegenSseService {
    #[must_use]
    pub fn new(configs: Vec<CategoryServerConfig>) -> Self {
        // Discover TLS certs once for all servers
        let (tls_cert, tls_key) = crate::config::discover_certificate_paths();
        
        let servers = configs
            .into_iter()
            .map(|cfg| CategoryServer {
                name: cfg.name,
                binary: cfg.binary,
                port: cfg.port,
                enabled: cfg.enabled,
                process: None,
            })
            .collect();

        Self {
            servers,
            tls_cert,
            tls_key,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        for server in &mut self.servers {
            if !server.enabled {
                log::debug!("Skipping disabled server: {}", server.name);
                continue;
            }

            let addr = format!("127.0.0.1:{}", server.port);
            log::info!("Starting {} server on {addr}", server.name);

            // Resolve binary path using which crate
            let binary_path = which::which(&server.binary).unwrap_or_else(|_| {
                log::warn!("{} binary not found in PATH, using relative path", server.binary);
                PathBuf::from(&server.binary)
            });

            log::debug!("{} binary path: {binary_path:?}", server.name);

            // Build command to spawn category server with SSE mode
            let mut cmd = tokio::process::Command::new(&binary_path);
            cmd.arg("--sse")
                .arg(&addr)
                .stdout(std::process::Stdio::inherit()) // Forward stdout to daemon's stdout
                .stderr(std::process::Stdio::inherit()); // Forward stderr to daemon's stderr

            // Add TLS configuration if certificates are available
            if let (Some(cert_path), Some(key_path)) = (&self.tls_cert, &self.tls_key) {
                log::info!(
                    "Configuring {} with HTTPS (cert={}, key={})",
                    server.name,
                    cert_path.display(),
                    key_path.display()
                );
                cmd.arg("--tls-cert").arg(cert_path);
                cmd.arg("--tls-key").arg(key_path);
            } else {
                log::info!("No TLS certificates configured, {} starting in HTTP mode", server.name);
            }

            // Spawn subprocess with error context
            let child = cmd.spawn().map_err(|e| {
                anyhow::anyhow!(
                    "Failed to spawn {} server (binary: {binary_path:?}, addr: {addr}): {e}",
                    server.name
                )
            })?;

            let pid = child.id();
            server.process = Some(child);

            log::info!("{} server started as subprocess (PID: {pid:?})", server.name);
        }
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        for server in &mut self.servers {
            if let Some(mut child) = server.process.take() {
                let pid = child.id();
                log::info!("Stopping {} server (PID: {pid:?})", server.name);

                #[cfg(unix)]
                {
                    use nix::sys::signal::{self, Signal};
                    use nix::unistd::Pid;

                    if let Some(pid_u32) = pid {
                        let nix_pid = Pid::from_raw(pid_u32 as i32);

                        // Phase 1: Send SIGTERM for graceful shutdown
                        match signal::kill(nix_pid, Signal::SIGTERM) {
                            Ok(()) => log::info!("Sent SIGTERM to {} process", server.name),
                            Err(e) => log::warn!("Failed to send SIGTERM to {}: {e}", server.name),
                        }

                        // Phase 2: Wait up to 30 seconds for graceful exit
                        match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait())
                            .await
                        {
                            Ok(Ok(status)) => {
                                log::info!("{} server exited gracefully: {status}", server.name);
                                continue;
                            }
                            Ok(Err(e)) => {
                                log::error!("Error waiting for {} process: {e}", server.name);
                                return Err(e.into());
                            }
                            Err(_) => {
                                log::warn!(
                                    "{} graceful shutdown timeout (30s), escalating to SIGKILL",
                                    server.name
                                );
                            }
                        }

                        // Phase 3: Graceful shutdown failed, send SIGKILL
                        child.start_kill()?;
                        log::warn!("Sent SIGKILL to {} process", server.name);

                        // Phase 4: Wait up to 5 seconds for SIGKILL (should be instant)
                        match tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
                            .await
                        {
                            Ok(Ok(status)) => {
                                log::info!("{} process terminated by SIGKILL: {status}", server.name);
                                continue;
                            }
                            Ok(Err(e)) => {
                                log::error!("Error after SIGKILL for {}: {e}", server.name);
                                return Err(e.into());
                            }
                            Err(_) => {
                                // SIGKILL failed after 5 seconds - this is a kernel-level problem
                                return Err(anyhow::anyhow!(
                                    "{} process did not respond to SIGKILL after 5 seconds. \
                                     This indicates a kernel-level issue (process in uninterruptible sleep, \
                                     zombie process, or kernel bug). PID: {pid_u32:?}",
                                    server.name
                                ));
                            }
                        }
                    }
                }

                #[cfg(windows)]
                {
                    // Windows doesn't have SIGTERM - start_kill() sends SIGKILL equivalent
                    log::info!("Terminating {} process (Windows - forceful kill)", server.name);
                    child.start_kill()?;

                    // Wait up to 5 seconds for process to exit
                    match tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await {
                        Ok(Ok(status)) => {
                            log::info!("{} server terminated: {}", server.name, status);
                            continue;
                        }
                        Ok(Err(e)) => {
                            log::error!("Error terminating {} process: {}", server.name, e);
                            return Err(e.into());
                        }
                        Err(_) => {
                            // Process didn't die after 5 seconds of forceful termination
                            return Err(anyhow::anyhow!(
                                "{} process did not terminate after 5 seconds on Windows. \
                                 PID: {:?}. This may indicate a hung process or system issue.",
                                server.name,
                                child.id()
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
