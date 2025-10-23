// packages/daemon/src/service/kodegen_sse.rs
use anyhow::Result;
use crate::config::KodegenSseConfig;

pub struct KodegenSseService {
    config: KodegenSseConfig,
    child_process: Option<tokio::process::Child>,
}

impl KodegenSseService {
    #[must_use] 
    pub fn new(config: KodegenSseConfig) -> Self {
        Self {
            config,
            child_process: None,
        }
    }
    
    pub async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            log::info!("Kodegen SSE server disabled in config");
            return Ok(());
        }
        
        let addr = format!("{}:{}", 
            self.config.bind_address, 
            self.config.port
        );
        
        log::info!("Starting kodegen SSE server on {addr} (subprocess mode)");
        
        // Resolve kodegen binary path using which crate
        let kodegen_path = which::which("kodegen")
            .unwrap_or_else(|_| {
                log::warn!("kodegen binary not found in PATH, using relative path");
                std::path::PathBuf::from("kodegen")
            });
        
        log::debug!("Kodegen binary path: {kodegen_path:?}");
        
        // Build command to spawn kodegen with SSE mode
        let mut cmd = tokio::process::Command::new(&kodegen_path);
        cmd.arg("--sse").arg(&addr)
           .stdout(std::process::Stdio::inherit())  // Forward stdout to daemon's stdout
           .stderr(std::process::Stdio::inherit());  // Forward stderr to daemon's stderr
        
        // Add TLS configuration if certificates are available
        if let (Some(cert_path), Some(key_path)) = (&self.config.tls_cert, &self.config.tls_key) {
            log::info!("Configuring HTTPS with cert={}, key={}", 
                cert_path.display(), key_path.display());
            cmd.arg("--tls-cert").arg(cert_path);
            cmd.arg("--tls-key").arg(key_path);
        } else {
            log::info!("No TLS certificates configured, starting in HTTP mode");
        }
        
        // Add enabled tools if configured
        if let Some(ref tools) = self.config.enabled_tools {
            for tool in tools {
                cmd.arg("--tool").arg(tool);
            }
        }
        
        // Spawn subprocess with error context
        let child = cmd.spawn()
            .map_err(|e| anyhow::anyhow!(
                "Failed to spawn kodegen SSE server (binary: {kodegen_path:?}, addr: {addr}): {e}"
            ))?;
        
        let pid = child.id();
        self.child_process = Some(child);
        
        log::info!("Kodegen SSE server started as subprocess (PID: {pid:?})");
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child_process.take() {
            let pid = child.id();
            log::info!("Stopping kodegen SSE server (PID: {pid:?})");
            
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                
                if let Some(pid_u32) = pid {
                    let nix_pid = Pid::from_raw(pid_u32 as i32);
                    
                    // Phase 1: Send SIGTERM for graceful shutdown
                    match signal::kill(nix_pid, Signal::SIGTERM) {
                        Ok(()) => log::info!("Sent SIGTERM to kodegen process"),
                        Err(e) => log::warn!("Failed to send SIGTERM: {e}"),
                    }
                    
                    // Phase 2: Wait up to 30 seconds for graceful exit
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        child.wait()
                    ).await {
                        Ok(Ok(status)) => {
                            log::info!("Kodegen SSE server exited gracefully: {status}");
                            return Ok(());
                        }
                        Ok(Err(e)) => {
                            log::error!("Error waiting for kodegen process: {e}");
                            return Err(e.into());
                        }
                        Err(_) => {
                            log::warn!("Graceful shutdown timeout (30s), escalating to SIGKILL");
                        }
                    }
                    
                    // Phase 3: Graceful shutdown failed, send SIGKILL
                    child.start_kill()?;
                    log::warn!("Sent SIGKILL to kodegen process");
                    
                    // Phase 4: Wait up to 5 seconds for SIGKILL (should be instant)
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        child.wait()
                    ).await {
                        Ok(Ok(status)) => {
                            log::info!("Process terminated by SIGKILL: {status}");
                            return Ok(());
                        }
                        Ok(Err(e)) => {
                            log::error!("Error after SIGKILL: {e}");
                            return Err(e.into());
                        }
                        Err(_) => {
                            // SIGKILL failed after 5 seconds - this is a kernel-level problem
                            return Err(anyhow::anyhow!(
                                "Process did not respond to SIGKILL after 5 seconds. \
                                 This indicates a kernel-level issue (process in uninterruptible sleep, \
                                 zombie process, or kernel bug). PID: {pid_u32:?}"
                            ));
                        }
                    }
                }
            }
            
            #[cfg(windows)]
            {
                // Windows doesn't have SIGTERM - start_kill() sends SIGKILL equivalent
                log::info!("Terminating kodegen process (Windows - forceful kill)");
                child.start_kill()?;
                
                // Wait up to 5 seconds for process to exit
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    child.wait()
                ).await {
                    Ok(Ok(status)) => {
                        log::info!("Kodegen SSE server terminated: {}", status);
                        return Ok(());
                    }
                    Ok(Err(e)) => {
                        log::error!("Error terminating kodegen process: {}", e);
                        return Err(e.into());
                    }
                    Err(_) => {
                        // Process didn't die after 5 seconds of forceful termination
                        return Err(anyhow::anyhow!(
                            "Process did not terminate after 5 seconds on Windows. \
                             PID: {:?}. This may indicate a hung process or system issue.",
                            child.id()
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
}
