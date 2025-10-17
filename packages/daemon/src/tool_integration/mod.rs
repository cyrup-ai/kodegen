use std::{collections::HashMap, path::PathBuf, sync::Arc};
use anyhow::{Context, Result};
use extism::*;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use log::{info, warn, error};
use crate::security::shell_executor::register_shell_host_functions;

/// Configuration for OCI registry plugin loading
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Registry URL (e.g., "https://ghcr.io")
    pub registry_url: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Password or token for authentication
    pub password: Option<String>,
    /// List of plugin image references to load
    pub plugins: Vec<String>,
    /// Local cache directory for downloaded plugins
    pub cache_dir: PathBuf,
    /// Whether to verify plugin signatures
    pub verify_signatures: bool,
}

impl RegistryConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self> {
        let plugins_str = std::env::var("SWEETMCP_PLUGINS")
            .context("SWEETMCP_PLUGINS not configured")?;
        
        let plugins: Vec<String> = plugins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        if plugins.is_empty() {
            anyhow::bail!("No plugins configured in SWEETMCP_PLUGINS");
        }
        
        let cache_dir = if let Ok(cache_home) = std::env::var("XDG_CACHE_HOME") {
            PathBuf::from(cache_home).join("sweetmcp/plugins")
        } else if let Some(home) = dirs::home_dir() {
            home.join(".cache/sweetmcp/plugins")
        } else {
            PathBuf::from("/tmp/sweetmcp/plugins")
        };
        
        Ok(Self {
            registry_url: std::env::var("SWEETMCP_REGISTRY_URL")
                .unwrap_or_else(|_| "https://ghcr.io".to_string()),
            username: std::env::var("SWEETMCP_REGISTRY_USER").ok(),
            password: std::env::var("SWEETMCP_REGISTRY_TOKEN").ok(),
            plugins,
            cache_dir,
            verify_signatures: std::env::var("SWEETMCP_VERIFY_SIGNATURES")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
        })
    }
    
    /// Build RegistryAuth from config
    pub fn build_auth(&self) -> oci_client::secrets::RegistryAuth {
        match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                oci_client::secrets::RegistryAuth::Basic(user.clone(), pass.clone())
            }
            _ => oci_client::secrets::RegistryAuth::Anonymous,
        }
    }
}

/// Plugin host for tool auto-configuration
pub struct ToolConfiguratorHost {
    /// Loaded configurator plugins
    plugins: Arc<RwLock<HashMap<String, Plugin>>>,
    /// Discovery paths
    discovery_paths: Vec<PathBuf>,
}

/// Information about a detected tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTool {
    pub name: String,
    pub version: Option<String>,
    pub installed: bool,
    pub config_path: Option<String>,
}

/// Configuration update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    pub server_name: String,
    pub server_config: ServerConfig,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Result of a configuration update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateResult {
    pub success: bool,
    pub message: String,
    pub restart_required: bool,
}

impl ToolConfiguratorHost {
    /// Create a new tool configurator host
    pub fn new() -> Self {
        let mut discovery_paths = vec![];
        
        // System-wide plugins
        discovery_paths.push(PathBuf::from("/usr/local/lib/sweetmcp/tool-configurators"));
        
        // User plugins
        if let Some(config_dir) = dirs::config_dir() {
            discovery_paths.push(config_dir.join("sweetmcp/tool-configurators"));
        }
        
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            discovery_paths,
        }
    }
    
    /// Discover and load all tool configurator plugins
    pub async fn discover_plugins(&self) -> Result<()> {
        info!("Discovering tool configurator plugins...");
        
        let mut plugins = self.plugins.write().await;
        
        // Load plugins from filesystem
        for path in &self.discovery_paths {
            if path.exists() {
                self.load_plugins_from_directory(&mut plugins, path).await?;
            }
        }
        
        // Load plugins from OCI registry if configured
        if let Ok(registry_config) = RegistryConfig::from_env() {
            info!("Loading plugins from OCI registry...");
            if let Err(e) = self.load_plugins_from_registry(&mut plugins, &registry_config).await {
                warn!("Failed to load plugins from registry: {}", e);
                // Continue with filesystem plugins - registry is optional
            }
        }
        
        info!("Loaded {} tool configurator plugins", plugins.len());
        Ok(())
    }
    
    /// Load plugins from a directory
    async fn load_plugins_from_directory(
        &self,
        plugins: &mut HashMap<String, Plugin>,
        dir: &PathBuf,
    ) -> Result<()> {
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {:?}", dir))?;
            
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                match self.load_plugin_from_file(&path).await {
                    Ok((name, plugin)) => {
                        info!("Loaded tool configurator: {}", name);
                        plugins.insert(name, plugin);
                    }
                    Err(e) => {
                        warn!("Failed to load plugin {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a single plugin from file
    async fn load_plugin_from_file(&self, path: &PathBuf) -> Result<(String, Plugin)> {
        let wasm = std::fs::read(path)
            .with_context(|| format!("Failed to read plugin file: {:?}", path))?;
            
        let manifest = Manifest::new([Wasm::data(wasm)]);
        let builder = PluginBuilder::new(&manifest).with_wasi(true);
        let builder = register_shell_host_functions(builder);
        let plugin = builder.build()
            .with_context(|| format!("Failed to create plugin from: {:?}", path))?;
            
        // Get plugin metadata
        let metadata: serde_json::Value = plugin.call("get_metadata", "")
            .with_context(|| format!("Failed to get metadata from plugin: {:?}", path))?;
            
        let name = metadata["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Plugin metadata missing 'name' field"))?
            .to_string();
            
        Ok((name, plugin))
    }
    
    /// Load plugins from OCI registry
    async fn load_plugins_from_registry(
        &self,
        plugins: &mut HashMap<String, Plugin>,
        config: &RegistryConfig,
    ) -> Result<()> {
        use oci_client::{Client, Reference, manifest};
        
        // Create OCI client with default configuration
        let client_config = oci_client::client::ClientConfig::default();
        let mut client = Client::new(client_config);
        
        let auth = config.build_auth();
        
        // Ensure cache directory exists
        if !config.cache_dir.exists() {
            std::fs::create_dir_all(&config.cache_dir)
                .context("Failed to create plugin cache directory")?;
        }
        
        for plugin_ref_str in &config.plugins {
            match self.load_plugin_from_registry(
                &mut client,
                &auth,
                plugin_ref_str,
                &config.cache_dir,
            ).await {
                Ok((name, plugin)) => {
                    info!("Loaded plugin from registry: {} ({})", name, plugin_ref_str);
                    plugins.insert(name, plugin);
                }
                Err(e) => {
                    warn!("Failed to load plugin from registry {}: {}", plugin_ref_str, e);
                    // Continue loading other plugins - don't fail entire operation
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a single plugin from OCI registry with caching
    async fn load_plugin_from_registry(
        &self,
        client: &mut oci_client::Client,
        auth: &oci_client::secrets::RegistryAuth,
        reference_str: &str,
        cache_dir: &PathBuf,
    ) -> Result<(String, Plugin)> {
        use oci_client::{Reference, manifest};
        
        // Parse OCI reference
        let reference: Reference = reference_str.parse()
            .with_context(|| format!("Invalid OCI reference: {}", reference_str))?;
        
        // Check cache first
        let cache_path = self.build_cache_path(cache_dir, &reference);
        
        let wasm_bytes = if cache_path.exists() {
            info!("Loading plugin from cache: {:?}", cache_path);
            std::fs::read(&cache_path)
                .with_context(|| format!("Failed to read cached plugin: {:?}", cache_path))?
        } else {
            // Pull from registry
            info!("Pulling plugin from registry: {}", reference_str);
            
            let image_data = client
                .pull(&reference, auth, vec![manifest::WASM_LAYER_MEDIA_TYPE])
                .await
                .with_context(|| format!("Failed to pull plugin: {}", reference_str))?;
            
            // Extract WASM bytes from first layer
            let wasm_bytes = image_data.layers
                .into_iter()
                .next()
                .map(|layer| layer.data)
                .ok_or_else(|| anyhow::anyhow!("No WASM layer found in {}", reference_str))?;
            
            // Write to cache
            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create cache directory: {:?}", parent))?;
            }
            
            std::fs::write(&cache_path, &wasm_bytes)
                .with_context(|| format!("Failed to write plugin to cache: {:?}", cache_path))?;
            
            info!("Cached plugin at: {:?}", cache_path);
            wasm_bytes
        };
        
        // Load into Extism plugin
        let manifest = extism::Manifest::new([extism::Wasm::data(wasm_bytes)]);
        let builder = PluginBuilder::new(&manifest).with_wasi(true);
        let builder = register_shell_host_functions(builder);
        let plugin = builder.build()
            .with_context(|| format!("Failed to create plugin from: {}", reference_str))?;
        
        // Get plugin metadata
        let metadata: serde_json::Value = plugin.call("get_metadata", "")
            .with_context(|| format!("Failed to get metadata from plugin: {}", reference_str))?;
        
        let name = metadata["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Plugin metadata missing 'name' field"))?
            .to_string();
        
        Ok((name, plugin))
    }
    
    /// Build cache file path for an OCI reference
    fn build_cache_path(&self, cache_dir: &PathBuf, reference: &oci_client::Reference) -> PathBuf {
        let registry = reference.resolve_registry().replace("://", "/").replace(":", "_");
        let repository = reference.repository().replace("/", "_");
        let tag = reference.tag().unwrap_or("latest");
        
        cache_dir
            .join(registry)
            .join(format!("{}-{}.wasm", repository, tag))
    }
    
    /// Detect all installed tools
    pub async fn detect_tools(&self) -> Result<Vec<DetectedTool>> {
        let mut detected_tools = Vec::new();
        let plugins = self.plugins.read().await;
        
        for (name, plugin) in plugins.iter() {
            match plugin.call::<&str, Json<DetectedTool>>("detect", "") {
                Ok(Json(tool)) => {
                    if tool.installed {
                        info!("Detected tool: {} ({})", tool.name, name);
                        detected_tools.push(tool);
                    }
                }
                Err(e) => {
                    warn!("Failed to detect tool {}: {}", name, e);
                }
            }
        }
        
        Ok(detected_tools)
    }
    
    /// Configure a specific tool
    pub async fn configure_tool(
        &self,
        tool_name: &str,
        config: ConfigUpdateRequest,
    ) -> Result<ConfigUpdateResult> {
        let plugins = self.plugins.read().await;
        
        let plugin = plugins.get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool configurator not found: {}", tool_name))?;
            
        // Read current configuration
        let current_config: serde_json::Value = plugin.call("read_config", "")
            .context("Failed to read current configuration")?;
            
        // Update configuration
        let result: Json<ConfigUpdateResult> = plugin.call("update_config", Json(&config))
            .context("Failed to update configuration")?;
            
        // Restart tool if needed
        if result.0.restart_required && result.0.success {
            match plugin.call::<&str, String>("restart_tool", "") {
                Ok(_) => {
                    info!("Successfully restarted {}", tool_name);
                }
                Err(e) => {
                    warn!("Failed to restart {}: {}", tool_name, e);
                    // Don't fail the whole operation if restart fails
                }
            }
        }
        
        Ok(result.0)
    }
    
    /// Configure all detected tools
    pub async fn configure_all_tools(&self, config: ConfigUpdateRequest) -> Result<()> {
        let tools = self.detect_tools().await?;
        
        for tool in tools {
            info!("Configuring {}...", tool.name);
            
            match self.configure_tool(&tool.name, config.clone()).await {
                Ok(result) => {
                    if result.success {
                        info!("Successfully configured {}: {}", tool.name, result.message);
                    } else {
                        warn!("Failed to configure {}: {}", tool.name, result.message);
                    }
                }
                Err(e) => {
                    error!("Error configuring {}: {}", tool.name, e);
                }
            }
        }
        
        Ok(())
    }
}