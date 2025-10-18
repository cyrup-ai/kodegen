// packages/daemon/src/service/kodegen_sse.rs
use anyhow::Result;
use crate::config::KodegenSseConfig;

pub struct KodegenSseService {
    config: KodegenSseConfig,
    child_process: Option<tokio::process::Child>,
}

impl KodegenSseService {
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
        
        log::info!("Starting kodegen SSE server on {} (subprocess mode)", addr);
        
        // Resolve kodegen binary path using which crate
        let kodegen_path = which::which("kodegen")
            .unwrap_or_else(|_| {
                log::warn!("kodegen binary not found in PATH, using relative path");
                std::path::PathBuf::from("kodegen")
            });
        
        log::debug!("Kodegen binary path: {:?}", kodegen_path);
        
        // Build command to spawn kodegen with SSE mode
        let mut cmd = tokio::process::Command::new(&kodegen_path);
        cmd.arg("--sse").arg(&addr)
           .stdout(std::process::Stdio::inherit())  // Forward stdout to daemon's stdout
           .stderr(std::process::Stdio::inherit());  // Forward stderr to daemon's stderr
        
        // Add enabled tools if configured
        if let Some(ref tools) = self.config.enabled_tools {
            for tool in tools {
                cmd.arg("--tool").arg(tool);
            }
        }
        
        // Spawn subprocess with error context
        let child = cmd.spawn()
            .map_err(|e| anyhow::anyhow!(
                "Failed to spawn kodegen SSE server (binary: {:?}, addr: {}): {}", 
                kodegen_path,
                addr,
                e
            ))?;
        
        let pid = child.id();
        self.child_process = Some(child);
        
        log::info!("Kodegen SSE server started as subprocess (PID: {:?})", pid);
        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child_process.take() {
            let pid = child.id();
            log::info!("Stopping kodegen SSE server (PID: {:?})", pid);
            
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                
                if let Some(pid_u32) = pid {
                    let nix_pid = Pid::from_raw(pid_u32 as i32);
                    
                    // Send SIGTERM for graceful shutdown
                    match signal::kill(nix_pid, Signal::SIGTERM) {
                        Ok(()) => log::info!("Sent SIGTERM to kodegen process"),
                        Err(e) => log::warn!("Failed to send SIGTERM: {}", e),
                    }
                    
                    // Wait for graceful exit with 30 second timeout
                    let timeout = std::time::Duration::from_secs(30);
                    let start = std::time::Instant::now();
                    
                    loop {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                log::info!("Kodegen SSE server exited with status: {}", status);
                                return Ok(());
                            }
                            Ok(None) => {
                                // Process still running
                                if start.elapsed() > timeout {
                                    log::warn!("Graceful shutdown timeout, sending SIGKILL");
                                    child.start_kill()?;
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                    continue;  // Loop again to confirm SIGKILL worked
                                }
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                            Err(e) => {
                                log::error!("Error waiting for kodegen process: {}", e);
                                return Err(e.into());
                            }
                        }
                    }
                }
            }
            
            #[cfg(windows)]
            {
                // Windows doesn't have SIGTERM, use forceful termination
                log::info!("Terminating kodegen process (Windows)");
                child.start_kill()?;
                
                // Poll for process exit (max 3 seconds)
                for _ in 0..30 {
                    if let Ok(Some(status)) = child.try_wait() {
                        log::info!("Kodegen SSE server exited with status: {}", status);
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                log::warn!("Kodegen process may not have exited cleanly");
            }
        }
        
        Ok(())
    }
}
