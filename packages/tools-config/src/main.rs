//! Config Category SSE Server
//!
//! Standalone SSE server serving only config tools. Based on the reference
//! implementation in packages/tools-filesystem/src/main.rs.

// ============================================================================
// IMPORTS
// ============================================================================

use kodegen_mcp_tool::Tool;
use kodegen_tools_config::ConfigManager;
use kodegen_utils::usage_tracker::UsageTracker;
use rmcp::model::{
    ServerCapabilities, ServerInfo, ToolInfo, PromptInfo, ResourceInfo, Implementation,
};
use rmcp::router::{PromptRouter, ResourceRouter, ToolRouter};
use rmcp::server::ServerHandler;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// ============================================================================
// CLI ARGUMENTS
// ============================================================================

#[derive(clap::Parser)]
#[command(name = "kodegen-config")]
#[command(about = "Config tools SSE server", long_about = None)]
struct Args {
    /// Enable SSE server mode
    #[arg(long)]
    sse: bool,

    /// Path to TLS certificate (optional, for HTTPS)
    #[arg(long)]
    tls_cert: Option<String>,

    /// Path to TLS private key (optional, for HTTPS)
    #[arg(long)]
    tls_key: Option<String>,
}

// ============================================================================
// SERVER IMPLEMENTATION
// ============================================================================

struct ConfigServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    usage_tracker: UsageTracker,
    config_manager: ConfigManager,
}

impl ServerHandler for ConfigServer {
    async fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: "0.1.0".to_string(),
            server_info: Implementation {
                name: "kodegen-config".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: ServerCapabilities {
                tools: Some(serde_json::json!({})),
                prompts: Some(serde_json::json!({})),
                resources: Some(serde_json::json!({})),
                logging: None,
                experimental: None,
            },
            instructions: Some("KODEGEN Config Category Server".to_string()),
        }
    }

    async fn list_tools(&self) -> Vec<ToolInfo> {
        self.tool_router.list_tools().await
    }

    async fn list_prompts(&self) -> Vec<PromptInfo> {
        self.prompt_router.list_prompts().await
    }

    async fn list_resources(&self) -> Vec<ResourceInfo> {
        vec![]
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // Track tool call with usage tracker
        let result = self.tool_router.call_tool(tool_name, arguments).await;
        self.usage_tracker.track_tool_call(
            tool_name,
            result.as_ref().map(|_| "success").unwrap_or("failure"),
        );
        result
    }

    async fn get_prompt(
        &self,
        prompt_name: &str,
        arguments: serde_json::Value,
    ) -> Result<Vec<rmcp::model::PromptMessage>, String> {
        self.prompt_router.get_prompt(prompt_name, arguments).await
    }

    async fn read_resource(&self, _uri: &str) -> Result<String, String> {
        Err("No resources available".to_string())
    }
}

// ============================================================================
// MAIN FUNCTION
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse CLI arguments
    let args = <Args as clap::Parser>::parse();

    // Initialize config manager
    let config_manager = ConfigManager::new();
    config_manager.init().await?;

    // Initialize usage tracker
    let usage_tracker = UsageTracker::new();

    // Create routers
    let tool_router = ToolRouter::new();
    let prompt_router = PromptRouter::new();
    let _resource_router = ResourceRouter::new();

    // Register config tools
    let (tool_router, prompt_router) = register_config_tools(
        tool_router,
        prompt_router,
        &config_manager,
        &usage_tracker,
    )?;

    // Create server
    let server = ConfigServer {
        tool_router,
        prompt_router,
        usage_tracker,
        config_manager,
    };

    // Run SSE server if requested
    if args.sse {
        log::info!("Starting Config SSE server...");

        // Create cancellation token for graceful shutdown
        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();

        // Setup signal handler for graceful shutdown
        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    log::info!("Received shutdown signal (Ctrl+C)");
                    cancel_clone.cancel();
                }
                Err(err) => {
                    log::error!("Failed to listen for shutdown signal: {}", err);
                }
            }
        });

        // Create SSE transport
        let transport = rmcp::transport::SseServerTransport::new();

        // Determine TLS configuration
        let tls_config = match (args.tls_cert, args.tls_key) {
            (Some(cert_path), Some(key_path)) => {
                log::info!("Loading TLS certificate from: {}", cert_path);
                log::info!("Loading TLS private key from: {}", key_path);

                // Read certificate and key files
                let cert = tokio::fs::read(&cert_path).await?;
                let key = tokio::fs::read(&key_path).await?;

                // Parse certificate chain
                let certs = rustls_pemfile::certs(&mut &cert[..])
                    .collect::<Result<Vec<_>, _>>()?;

                // Parse private key
                let key = rustls_pemfile::private_key(&mut &key[..])?
                    .ok_or_else(|| anyhow::anyhow!("No private key found in key file"))?;

                // Build TLS config
                let tls_config = rustls::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certs, key)?;

                Some(tls_config)
            }
            (Some(_), None) => {
                anyhow::bail!("TLS certificate provided without private key");
            }
            (None, Some(_)) => {
                anyhow::bail!("TLS private key provided without certificate");
            }
            (None, None) => None,
        };

        // Generate instance ID (unique per server instance)
        let instance_id = format!(
            "kodegen-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis()
        );

        log::info!("Server instance ID: {}", instance_id);

        // Start SSE server
        let bind_addr = "127.0.0.1:0"; // Bind to random available port
        log::info!("Binding to {}", bind_addr);

        if let Some(tls_config) = tls_config {
            // HTTPS server
            log::info!("Starting HTTPS SSE server...");
            let tls_config = Arc::new(tls_config);
            transport
                .start_with_tls(server, bind_addr, tls_config, Some(cancel_token))
                .await?;
        } else {
            // HTTP server
            log::info!("Starting HTTP SSE server...");
            transport
                .start(server, bind_addr, Some(cancel_token))
                .await?;
        }

        log::info!("Server shutdown complete");
    } else {
        log::error!("SSE mode not enabled. Use --sse flag to start the server.");
        anyhow::bail!("SSE mode required");
    }

    Ok(())
}

// ============================================================================
// TOOL REGISTRATION
// ============================================================================

fn register_config_tools<S>(
    mut tool_router: ToolRouter<S>,
    mut prompt_router: PromptRouter<S>,
    config_manager: &kodegen_tools_config::ConfigManager,
    _usage_tracker: &UsageTracker,
) -> Result<(ToolRouter<S>, PromptRouter<S>), anyhow::Error>
where
    S: Send + Sync + 'static,
{
    use kodegen_tools_config::*;

    // Helper function to register tool in both routers
    fn register<S, T>(
        mut tool_router: ToolRouter<S>,
        mut prompt_router: PromptRouter<S>,
        tool: T,
    ) -> (ToolRouter<S>, PromptRouter<S>)
    where
        S: Send + Sync + 'static,
        T: Tool + Clone + Send + Sync + 'static,
        T::Args: Send + Sync + 'static,
        T::PromptArgs: Send + Sync + 'static,
    {
        let tool_arc = Arc::new(tool);
        tool_router = tool_router.register_tool(tool_arc.clone());
        prompt_router = prompt_router.register_prompt(tool_arc);
        (tool_router, prompt_router)
    }

    // Register GetConfigTool
    (tool_router, prompt_router) = register(
        tool_router,
        prompt_router,
        GetConfigTool::new(config_manager.clone()),
    );

    // Register SetConfigValueTool
    (tool_router, prompt_router) = register(
        tool_router,
        prompt_router,
        SetConfigValueTool::new(config_manager.clone()),
    );

    Ok((tool_router, prompt_router))
}
