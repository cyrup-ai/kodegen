// packages/server/src/common/router_builder.rs
use anyhow::Result;
use rmcp::handler::server::router::{tool::ToolRouter, prompt::PromptRouter};
use std::collections::HashSet;
use kodegen_utils::usage_tracker::UsageTracker;

/// Container for both routers
pub struct RouterSet<S> 
where 
    S: Send + Sync + 'static
{
    pub tool_router: ToolRouter<S>,
    pub prompt_router: PromptRouter<S>,
}

/// Build tool and prompt routers with all available tools
/// 
/// This function is generic over S to work with both StdioServer and SseServer
pub async fn build_routers<S>(
    config_manager: &kodegen_config::ConfigManager,
    usage_tracker: &UsageTracker,
    enabled_categories: &Option<HashSet<String>>,
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
            log::info!("Runtime filter: ONLY these categories enabled: {:?}", set);
            // No validation needed - already validated at startup
        }
    }
    
    // Initialize routers
    let tool_router = ToolRouter::new();
    let prompt_router = PromptRouter::new();
    
    // Register all tools using the tool_registry helper (zero-clone move semantics)
    let (tool_router, prompt_router) = crate::common::tool_registry::register_all_tools(
        tool_router,
        prompt_router,
        config_manager,
        usage_tracker,
        enabled_categories,
    ).await?;
    
    Ok(RouterSet {
        tool_router,
        prompt_router,
    })
}
