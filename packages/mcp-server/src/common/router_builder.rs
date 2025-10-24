// packages/server/src/common/router_builder.rs
use anyhow::Result;
use rmcp::handler::server::router::{tool::ToolRouter, prompt::PromptRouter};
use std::collections::HashSet;
#[cfg(any(feature = "citescrape", feature = "browser", feature = "database"))]
use std::sync::Arc;
use kodegen_utils::usage_tracker::UsageTracker;

/// Managers that require explicit shutdown on server exit
pub struct Managers {
    #[cfg(feature = "citescrape")]
    pub browser_manager: Option<Arc<kodegen_tools_citescrape::BrowserManager>>,
    
    #[cfg(feature = "browser")]
    pub browser_tools_manager: Option<Arc<kodegen_tools_browser::BrowserManager>>,
    
    #[cfg(feature = "database")]
    pub tunnel_guard: std::sync::Arc<tokio::sync::Mutex<Option<kodegen_tools_database::SSHTunnel>>>,
}

impl Managers {
    /// Shutdown all managers gracefully
    pub async fn shutdown(&self) -> Result<()> {
        #[cfg(feature = "citescrape")]
        if let Some(ref manager) = self.browser_manager
            && let Err(e) = manager.shutdown().await
        {
            log::warn!("Failed to shutdown browser manager: {e}");
        }
        
        #[cfg(feature = "browser")]
        if let Some(ref manager) = self.browser_tools_manager
            && let Err(e) = manager.shutdown().await
        {
            log::warn!("Failed to shutdown browser tools manager: {e}");
        }
        
        #[cfg(feature = "database")]
        {
            let mut guard = self.tunnel_guard.lock().await;
            if let Some(tunnel) = guard.take() {
                tunnel.close().await;
            }
        }
        
        Ok(())
    }
}

/// Container for both routers and managers
pub struct RouterSet<S> 
where 
    S: Send + Sync + 'static
{
    pub tool_router: ToolRouter<S>,
    pub prompt_router: PromptRouter<S>,
    pub managers: Managers,
}

/// Build tool and prompt routers with all available tools
/// 
/// This function is generic over S to work with both `StdioServer` and `SseServer`
pub async fn build_routers<S>(
    config_manager: &kodegen_tools_config::ConfigManager,
    usage_tracker: &UsageTracker,
    enabled_categories: &Option<HashSet<String>>,
    database_dsn: Option<&str>,
    #[cfg(feature = "database")]
    ssh_config: Option<(kodegen_tools_database::SSHConfig, kodegen_tools_database::TunnelConfig)>,
    #[cfg(not(feature = "database"))]
    ssh_config: Option<()>,
) -> Result<RouterSet<S>>
where
    S: Send + Sync + 'static
{
    // Log what's enabled
    match enabled_categories {
        None => {
            log::info!("Runtime filter: ALL compiled tool categories enabled");
        }
        Some(set) => {
            log::info!("Runtime filter: ONLY these categories enabled: {set:?}");
            // No validation needed - already validated at startup
        }
    }
    
    // Initialize routers
    let tool_router = ToolRouter::new();
    let prompt_router = PromptRouter::new();
    
    // Register all tools using the tool_registry helper (zero-clone move semantics)
    let (tool_router, prompt_router, managers) = crate::common::tool_registry::register_all_tools(
        tool_router,
        prompt_router,
        config_manager,
        usage_tracker,
        enabled_categories,
        database_dsn,
        ssh_config,
    ).await?;
    
    Ok(RouterSet {
        tool_router,
        prompt_router,
        managers,
    })
}
