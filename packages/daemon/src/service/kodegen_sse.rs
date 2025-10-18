// packages/daemon/src/service/kodegen_sse.rs
use anyhow::Result;
use std::net::SocketAddr;
use tokio_util::sync::CancellationToken;
use crate::config::KodegenSseConfig;

pub struct KodegenSseService {
    config: KodegenSseConfig,
    cancellation_token: Option<CancellationToken>,
}

impl KodegenSseService {
    pub fn new(config: KodegenSseConfig) -> Self {
        Self {
            config,
            cancellation_token: None,
        }
    }
    
    pub async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            log::info!("Kodegen SSE server disabled in config");
            return Ok(());
        }
        
        let addr: SocketAddr = format!("{}:{}", 
            self.config.bind_address, 
            self.config.port
        ).parse()?;
        
        log::info!("Starting kodegen SSE server on {}", addr);
        
        // Convert enabled_tools to HashSet if provided
        let enabled_categories = self.config.enabled_tools.clone()
            .map(|tools| tools.into_iter().collect());
        
        // Create config manager and usage tracker
        let config_manager = kodegen_config::ConfigManager::new();
        config_manager.init().await?;
        let usage_tracker = kodegen_utils::usage_tracker::UsageTracker::new();
        
        // Build routers
        let routers = kodegen::common::build_routers::<kodegen::sse::SseServer>(
            &config_manager,
            &usage_tracker,
            &enabled_categories,
        ).await?;
        
        // Create SSE server
        let server = kodegen::sse::SseServer::new(
            routers.tool_router,
            routers.prompt_router,
            usage_tracker,
            config_manager,
        );
        
        // Serve returns CancellationToken from rmcp's SseServer::serve().with_service_directly()
        let ct = server.serve(addr).await?;
        
        self.cancellation_token = Some(ct);
        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<()> {
        if let Some(ct) = self.cancellation_token.take() {
            log::info!("Stopping kodegen SSE server");
            ct.cancel();
        }
        Ok(())
    }
}
