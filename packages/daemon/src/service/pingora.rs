use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use log::{error, info, warn};
use tokio_util::sync::CancellationToken;

use crate::config::ServiceDefinition;
use crate::ipc::{Cmd, Evt};

/// Pingora proxy service that manages TLS certificates and spawns sweetmcp_server
pub struct PingoraService {
    name: String,
    bus: Sender<Evt>,
    def: ServiceDefinition,
}

impl PingoraService {
    pub fn new(def: ServiceDefinition, bus: Sender<Evt>) -> Self {
        Self {
            name: def.name.clone(),
            bus,
            def,
        }
    }

    pub fn run(self, cmd_rx: Receiver<Cmd>) -> Result<()> {
        info!("🚀 Starting SweetMCP Pingora proxy service");

        // Create tokio runtime for TLS operations
        let rt = tokio::runtime::Runtime::new()?;

        // Create cancellation token for graceful shutdown (reserved for future implementation)
        let _cancel_token = CancellationToken::new();
        let _shutdown_complete = Arc::new(AtomicBool::new(false));

        let mut child: Option<Child> = None;

        // Handle control commands
        loop {
            match cmd_rx.recv()? {
                Cmd::Start => {
                    if child.is_some() {
                        warn!("{} already running", self.name);
                        continue;
                    }

                    // Verify TLS certificates before starting
                    match rt.block_on(self.setup_tls()) {
                        Ok(()) => match self.spawn_pingora_binary() {
                            Ok(spawned_child) => {
                                let pid = spawned_child.id();
                                child = Some(spawned_child);

                                let _ = self.bus.send(Evt::State {
                                    service: self.name.clone(),
                                    kind: "running",
                                    ts: chrono::Utc::now(),
                                    pid: Some(pid),
                                });

                                info!("{} started (pid {})", self.name, pid);
                            }
                            Err(e) => {
                                error!("Failed to spawn pingora binary: {}", e);
                                let _ = self.bus.send(Evt::Fatal {
                                    service: self.name.clone(),
                                    msg: "Failed to spawn binary",
                                    ts: chrono::Utc::now(),
                                });
                            }
                        },
                        Err(e) => {
                            error!("Failed to setup TLS: {}", e);
                            let _ = self.bus.send(Evt::Fatal {
                                service: self.name.clone(),
                                msg: "TLS setup failed",
                                ts: chrono::Utc::now(),
                            });
                        }
                    }
                }
                Cmd::Stop => {
                    if let Some(mut ch) = child.take() {
                        let pid = ch.id();
                        ch.kill().ok();
                        let _ = self.bus.send(Evt::State {
                            service: self.name.clone(),
                            kind: "stopped",
                            ts: chrono::Utc::now(),
                            pid: Some(pid),
                        });
                        info!("{} stopped", self.name);
                    }
                }
                Cmd::Restart => {
                    // Stop then start
                    if let Some(mut ch) = child.take() {
                        ch.kill().ok();
                    }
                    // Will restart on next Start command
                }
                Cmd::Shutdown => {
                    if let Some(mut ch) = child.take() {
                        ch.kill().ok();
                    }
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn setup_tls(&self) -> Result<()> {
        let cert_dir = self.get_cert_dir();

        info!("Loading TLS certificates from: {}", cert_dir.display());

        // Verify certificate files exist (generated during installation)
        let ca_cert_path = cert_dir.join("ca.crt");
        let ca_key_path = cert_dir.join("ca.key");
        let server_cert_path = cert_dir.join("server.crt");
        let server_key_path = cert_dir.join("server.key");

        // Check if all required certificate files exist
        for (file_type, path) in [
            ("CA certificate", &ca_cert_path),
            ("CA key", &ca_key_path),
            ("Server certificate", &server_cert_path),
            ("Server key", &server_key_path),
        ] {
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "{} not found: {}. Please run the installer to generate certificates.",
                    file_type,
                    path.display()
                ));
            }
        }

        info!("TLS certificates verified successfully");
        Ok(())
    }

    fn get_cert_dir(&self) -> std::path::PathBuf {
        let xdg_config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{}/.config", home)
        });
        std::path::PathBuf::from(xdg_config)
            .join("sweetmcp")
            .join("certs")
    }

    fn spawn_pingora_binary(&self) -> Result<Child> {
        // Determine binary path
        let binary_path = self.find_pingora_binary()?;

        info!("Spawning pingora binary: {}", binary_path);

        let mut cmd = Command::new(binary_path);

        // Set working directory
        if let Some(dir) = &self.def.working_dir {
            cmd.current_dir(dir);
        }

        // No environment variables needed - pingora uses XDG_CONFIG_HOME/sweetmcp directly

        // Add service environment variables
        for (key, value) in &self.def.env_vars {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdout(Stdio::null()).stderr(Stdio::null());

        let child = cmd.spawn()?;
        Ok(child)
    }

    fn find_pingora_binary(&self) -> Result<String> {
        // Check common locations for sweetmcp_server binary
        let possible_paths = vec![
            "target/debug/sweetmcp_server",
            "target/release/sweetmcp_server",
            "/usr/local/bin/sweetmcp_server",
            "./sweetmcp_server",
        ];

        for path in possible_paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }

        Err(anyhow::anyhow!("Could not find sweetmcp_server binary"))
    }
}

/// Spawn the pingora service thread
pub fn spawn_pingora(def: ServiceDefinition, bus: Sender<Evt>) -> Result<Sender<Cmd>, crate::service::ServiceError> {
    let (cmd_tx, cmd_rx) = crossbeam_channel::bounded(16);
    let service_name = def.name.clone();

    thread::Builder::new()
        .name(format!("svc-pingora-{}", service_name))
        .spawn(move || {
            let service = PingoraService::new(def, bus);
            if let Err(e) = service.run(cmd_rx) {
                error!("Pingora service error: {}", e);
            }
        })
        .map_err(|source| crate::service::ServiceError::SpawnFailed {
            service: service_name,
            source,
        })?;

    Ok(cmd_tx)
}
