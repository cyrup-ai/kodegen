// Category SSE Server: Database Tools
//
// This binary serves database query and schema exploration tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30446.
//
// REQUIRED: DATABASE_DSN environment variable must be set.
// OPTIONAL: SSH_* environment variables for SSH tunnel support.

use anyhow::{Result, Context};
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, ShutdownHook, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// SSH TUNNEL SHUTDOWN HOOK
// ============================================================================

/// Wrapper to implement ShutdownHook for SSH tunnel
struct TunnelGuard(Arc<Mutex<Option<kodegen_tools_database::SSHTunnel>>>);

impl ShutdownHook for TunnelGuard {
    fn shutdown(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let guard = self.0.clone();
        Box::pin(async move {
            let mut tunnel_opt = guard.lock().await;
            if let Some(tunnel) = tunnel_opt.take() {
                log::info!("Closing SSH tunnel for database connection");
                tunnel.close().await;
            }
            Ok(())
        })
    }
}

// ============================================================================
// ENVIRONMENT VARIABLE PARSING
// ============================================================================

/// Parse SSH configuration from environment variables
///
/// Required variables (all must be set):
/// - SSH_HOST: SSH server hostname
/// - SSH_PORT: SSH server port
/// - SSH_USER: SSH username
/// - SSH_AUTH_TYPE: "password" or "key"
///
/// For password auth:
/// - SSH_PASSWORD: Password
///
/// For key auth:
/// - SSH_KEY_PATH: Path to private key
/// - SSH_KEY_PASSPHRASE: Optional key passphrase
///
/// Target configuration:
/// - SSH_TARGET_HOST: Database host from SSH perspective
/// - SSH_TARGET_PORT: Database port
fn parse_ssh_config_from_env() -> Result<Option<(
    kodegen_tools_database::SSHConfig,
    kodegen_tools_database::TunnelConfig,
)>> {
    // Check if SSH is configured
    let ssh_host = match std::env::var("SSH_HOST") {
        Ok(h) => h,
        Err(_) => return Ok(None), // No SSH configured
    };

    // All other SSH vars are required if SSH_HOST is set
    let ssh_port: u16 = std::env::var("SSH_PORT")
        .context("SSH_PORT required when SSH_HOST is set")?
        .parse()
        .context("SSH_PORT must be valid port number")?;

    let ssh_user = std::env::var("SSH_USER")
        .context("SSH_USER required when SSH_HOST is set")?;

    let auth_type = std::env::var("SSH_AUTH_TYPE")
        .context("SSH_AUTH_TYPE required (must be 'password' or 'key')")?;

    let auth = match auth_type.as_str() {
        "password" => {
            let password = std::env::var("SSH_PASSWORD")
                .context("SSH_PASSWORD required when SSH_AUTH_TYPE=password")?;
            kodegen_tools_database::SSHAuth::Password(password)
        }
        "key" => {
            let key_path = std::env::var("SSH_KEY_PATH")
                .context("SSH_KEY_PATH required when SSH_AUTH_TYPE=key")?;
            let passphrase = std::env::var("SSH_KEY_PASSPHRASE").ok();
            kodegen_tools_database::SSHAuth::Key {
                path: PathBuf::from(key_path),
                passphrase,
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid SSH_AUTH_TYPE '{}': must be 'password' or 'key'",
                auth_type
            ));
        }
    };

    let target_host = std::env::var("SSH_TARGET_HOST")
        .context("SSH_TARGET_HOST required for tunnel target")?;
    let target_port: u16 = std::env::var("SSH_TARGET_PORT")
        .context("SSH_TARGET_PORT required for tunnel target")?
        .parse()
        .context("SSH_TARGET_PORT must be valid port number")?;

    let ssh_config = kodegen_tools_database::SSHConfig {
        host: ssh_host,
        port: ssh_port,
        username: ssh_user,
        auth,
    };

    let tunnel_config = kodegen_tools_database::TunnelConfig {
        target_host,
        target_port,
    };

    Ok(Some((ssh_config, tunnel_config)))
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("database", |config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let mut managers = Managers::new();

        // Get DATABASE_DSN from environment (required)
        let dsn = std::env::var("DATABASE_DSN")
            .context("DATABASE_DSN environment variable is required for database server")?;

        // Parse optional SSH tunnel configuration
        let ssh_config = parse_ssh_config_from_env()?;

        // Setup database connection pool (with optional SSH tunnel)
        let db_connection = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                kodegen_tools_database::setup_database_pool(config, &dsn, ssh_config).await
            })
        })?;

        // Register SSH tunnel for graceful shutdown if present
        if db_connection.tunnel.is_some() {
            let tunnel_guard = Arc::new(Mutex::new(db_connection.tunnel));
            managers.register(TunnelGuard(tunnel_guard));
        }

        // Register all 7 database tools
        use kodegen_tools_database::tools::*;

        let pool = db_connection.pool;
        let connection_url = &db_connection.connection_url;

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            ExecuteSQLTool::new(pool.clone(), config.clone(), connection_url)?,
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            ListSchemasTool::new(pool.clone(), connection_url, config.clone())?,
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            ListTablesTool::new(pool.clone(), connection_url, config.clone())?,
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            GetTableSchemaTool::new(pool.clone(), connection_url, Arc::new(config.clone()))?,
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            GetTableIndexesTool::new(pool.clone(), connection_url, Arc::new(config.clone()))?,
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            GetStoredProceduresTool::new(pool.clone(), connection_url, Arc::new(config.clone()))?,
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            GetPoolStatsTool::new(pool.clone(), connection_url)?,
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
