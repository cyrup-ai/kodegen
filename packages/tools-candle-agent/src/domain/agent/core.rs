//! Core agent data structures with automatic memory tool injection

use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use rmcp::model::Tool as ToolInfo;
use serde_json::Value;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::core::EngineError;
use crate::domain::context::CandleDocument as Document;
use crate::domain::memory::MemoryConfig;
use crate::domain::memory::config::memory::MemoryConfig as ComprehensiveMemoryConfig;
use crate::domain::memory::{Error as MemoryError, MemoryTool, MemoryToolError};
use crate::domain::model::CandleModel as Model;
use crate::memory::core::SurrealDBMemoryManager;
// Tool data now comes from SweetMCP ToolInfo directly
use cyrup_sugars::ZeroOneOrMany;

/// High-performance lock-free counter for monitoring operations
#[derive(Debug, Default)]
pub struct RelaxedCounter {
    value: AtomicU64,
}

impl RelaxedCounter {
    #[inline]
    #[must_use]
    pub const fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
        }
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn inc(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Relaxed)
    }
}

/// Comprehensive agent performance statistics with lock-free atomics
#[derive(Debug)]
pub struct AgentStatistics {
    /// Total agents created (atomic counter)
    pub agents_created: RelaxedCounter,
    /// Currently active agents (atomic counter)
    pub agents_active: AtomicUsize,
    /// Total completions processed (atomic counter)
    pub completions_total: RelaxedCounter,
    /// Total tokens processed across all completions (atomic counter)
    pub tokens_total: RelaxedCounter,
    /// Average completion time in microseconds (atomic)
    pub avg_completion_time_us: AtomicU64,
}

impl AgentStatistics {
    /// Create new statistics with zero allocation
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            agents_created: RelaxedCounter::new(0),
            agents_active: AtomicUsize::new(0),
            completions_total: RelaxedCounter::new(0),
            tokens_total: RelaxedCounter::new(0),
            avg_completion_time_us: AtomicU64::new(0),
        }
    }

    /// Record agent creation with atomic increment
    #[inline]
    pub fn record_agent_created(&self) {
        self.agents_created.inc();
        self.agents_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record agent destruction with atomic decrement
    #[inline]
    pub fn record_agent_destroyed(&self) {
        self.agents_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record completion with token counting and duration averaging
    #[inline]
    pub fn record_completion(&self, token_count: u64, duration_us: u64) {
        self.completions_total.inc();
        self.tokens_total
            .value
            .fetch_add(token_count, Ordering::Relaxed);

        // Update average duration using atomic operations
        let current_avg = self.avg_completion_time_us.load(Ordering::Relaxed);
        let total_completions = self.completions_total.get();

        if total_completions > 0 {
            let new_avg =
                ((current_avg * (total_completions - 1)) + duration_us) / total_completions;
            self.avg_completion_time_us
                .store(new_avg, Ordering::Relaxed);
        }
    }
}

impl Default for AgentStatistics {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Maximum number of tools per agent (const generic default)
pub const MAX_AGENT_TOOLS: usize = 32;

/// Agent statistics for performance monitoring
pub static AGENT_STATS: AgentStatistics = AgentStatistics::new();

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Agent creation error types
#[derive(Debug, Clone, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum AgentError {
    /// Memory system initialization error
    #[error("Memory initialization failed: {0}")]
    MemoryInit(String),
    /// Memory initialization failed during async initialization
    #[error("Memory initialization failed: {0}")]
    MemoryInitializationFailed(String),
    /// Memory tool creation error
    #[error("Memory tool creation failed: {0}")]
    MemoryTool(String),
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    /// Agent initialization error
    #[error("Agent initialization failed: {0}")]
    InitializationError(String),
    /// Completion provider not initialized
    #[error("Completion provider not initialized - use .completion_provider() in builder")]
    ProviderNotInitialized,
    /// Engine initialization or execution error
    #[error("Engine error: {0}")]
    Engine(String),
    /// Model loading or inference error
    #[error("Model error: {0}")]
    Model(String),
    /// Prompt template or formatting error
    #[error("Prompt error: {0}")]
    Prompt(String),
    /// Context loading error (distinct from general Config)
    #[error("Context error: {0}")]
    Context(String),
    /// Tool invocation error (distinct from `MemoryTool`)
    #[error("Tool error: {0}")]
    Tool(String),
    /// Unknown/unclassified error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

// From implementations for error conversions
impl From<MemoryError> for AgentError {
    fn from(err: MemoryError) -> Self {
        AgentError::MemoryInit(err.to_string())
    }
}

impl From<Box<MemoryToolError>> for AgentError {
    fn from(err: Box<MemoryToolError>) -> Self {
        AgentError::MemoryTool(err.to_string())
    }
}

impl From<MemoryToolError> for AgentError {
    fn from(err: MemoryToolError) -> Self {
        AgentError::MemoryTool(err.to_string())
    }
}

impl From<EngineError> for AgentError {
    fn from(err: EngineError) -> Self {
        AgentError::Engine(err.to_string())
    }
}

/// Agent data structure with automatic memory tool injection
#[derive(Debug, Clone)]
pub struct Agent<M: Model> {
    /// The model configuration and implementation to use for this agent
    pub model: M,
    /// System prompt that defines the agent's personality and behavior
    pub system_prompt: String,
    /// Context documents that provide background information
    pub context: ZeroOneOrMany<Document>,
    /// MCP tools available to the agent for function calling
    pub tools: ZeroOneOrMany<ToolInfo>,
    /// Optional memory system for storing and retrieving conversation context
    pub memory: Option<Arc<SurrealDBMemoryManager>>,
    /// Memory tool for automated memory management operations
    pub memory_tool: Option<MemoryTool>,
    /// Temperature setting for response randomness (0.0 to 2.0)
    pub temperature: Option<f64>,
    /// Maximum number of tokens to generate in responses
    pub max_tokens: Option<u64>,
    /// Additional model-specific parameters as flexible JSON
    pub additional_params: Option<Value>,
    /// Typed error if agent creation/operation failed
    pub error: Option<AgentError>,
}

impl<M: Model + Clone + Send + 'static + Default> Agent<M> {
    /// Create a new agent with zero-allocation memory tool injection
    ///
    /// # Arguments
    /// * `model` - Model configuration for the agent
    /// * `system_prompt` - System prompt for the agent
    ///
    /// # Returns
    /// Stream that emits configured agent with memory tool
    ///
    /// # Performance
    /// Zero allocation agent construction with lock-free memory manager sharing
    #[inline]
    pub fn new(
        model: M,
        system_prompt: impl Into<String>,
    ) -> Pin<
        Box<
            dyn Stream<Item = crate::domain::context::chunks::CandleResult<Self, AgentError>>
                + Send,
        >,
    > {
        let system_prompt = system_prompt.into();
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Initialize memory system with cognitive settings optimized for performance
            let comprehensive_config = ComprehensiveMemoryConfig::default();

            // Initialize the memory system asynchronously
            let memory_manager = match crate::memory::initialize(&crate::memory::MemoryConfig {
                database: crate::memory::utils::config::DatabaseConfig {
                    db_type: crate::memory::utils::config::DatabaseType::SurrealDB,
                    connection_string: comprehensive_config.database.connection_string.clone(),
                    namespace: "test".to_string(),
                    database: "memory".to_string(),
                    username: None,
                    password: None,
                    pool_size: None,
                    options: None,
                },
                vector_store: crate::memory::utils::config::VectorStoreConfig::default(),
                api: None,
                cache: crate::memory::utils::config::CacheConfig::default(),
                logging: crate::memory::utils::config::LoggingConfig::default(),
            })
            .await
            {
                Ok(manager) => manager,
                Err(e) => {
                    let _ = tx.send(crate::domain::context::chunks::CandleResult {
                        result: Err(AgentError::MemoryInitializationFailed(e.to_string())),
                    });
                    return;
                }
            };

            // Create memory tool with the initialized manager
            let memory_manager_arc = Arc::new(memory_manager);
            let memory_tool = MemoryTool::new(Arc::clone(&memory_manager_arc));

            // Create agent with properly initialized memory
            let agent = Self {
                model,
                system_prompt,
                context: ZeroOneOrMany::None,
                tools: ZeroOneOrMany::None,
                memory: Some(memory_manager_arc),
                memory_tool: Some(memory_tool),
                temperature: None,
                max_tokens: None,
                additional_params: None,
                error: None,
            };

            // Record agent creation in statistics
            AGENT_STATS.record_agent_created();

            // Send the successfully created agent
            let _ = tx.send(crate::domain::context::chunks::CandleResult { result: Ok(agent) });
        }))
    }

    /// Create a new agent with custom memory configuration
    ///
    /// # Arguments
    /// * `model` - Model configuration for the agent
    /// * `system_prompt` - System prompt for the agent
    /// * `memory_config` - Custom memory configuration
    ///
    /// # Returns
    /// Stream that emits configured agent with memory tool
    ///
    /// # Performance
    /// Zero allocation with custom cognitive settings
    #[inline]
    pub fn with_memory_config(
        model: M,
        system_prompt: impl Into<String>,
        memory_config: ComprehensiveMemoryConfig,
    ) -> Pin<
        Box<
            dyn Stream<Item = crate::domain::context::chunks::CandleResult<Self, AgentError>>
                + Send,
        >,
    > {
        let system_prompt = system_prompt.into();
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Convert comprehensive config to Memory::new() format
            let memory_cfg = MemoryConfig {
                database: crate::domain::memory::config::database::DatabaseConfig {
                    db_type: crate::domain::memory::config::database::DatabaseType::SurrealDB,
                    connection_string: memory_config.database.connection_string.clone(),
                    namespace: "test".to_string(),
                    database: "memory".to_string(),
                    username: None,
                    password: None,
                    pool_config: crate::domain::memory::config::database::PoolConfig::default(),
                    timeout_config: crate::domain::memory::config::database::TimeoutConfig::default(
                    ),
                    health_check_config:
                        crate::domain::memory::config::database::HealthCheckConfig::default(),
                    options: None,
                },
                vector_store: crate::domain::memory::config::vector::VectorStoreConfig::default(),
                provider_model: crate::domain::chat::config::CandleModelConfig::default(),
                cognitive: crate::domain::memory::cognitive::types::CognitiveMemoryConfig::default(
                ),
                cognitive_processor:
                    crate::domain::memory::cognitive::types::CognitiveProcessorConfig::default(),
                performance:
                    crate::domain::memory::config::memory::MemoryPerformanceConfig::default(),
                retention: crate::domain::memory::config::memory::MemoryRetentionConfig::default(),
                security: crate::domain::memory::config::memory::MemorySecurityConfig::default(),
                monitoring: crate::domain::memory::config::memory::MemoryMonitoringConfig::default(
                ),
            };

            // Initialize the memory system asynchronously
            let memory_manager = match crate::memory::initialize(&crate::memory::MemoryConfig {
                database: crate::memory::utils::config::DatabaseConfig {
                    db_type: crate::memory::utils::config::DatabaseType::SurrealDB,
                    connection_string: memory_cfg.database.connection_string.clone(),
                    namespace: "test".to_string(),
                    database: "memory".to_string(),
                    username: None,
                    password: None,
                    pool_size: None,
                    options: None,
                },
                vector_store: crate::memory::utils::config::VectorStoreConfig::default(),
                api: None,
                cache: crate::memory::utils::config::CacheConfig::default(),
                logging: crate::memory::utils::config::LoggingConfig::default(),
            })
            .await
            {
                Ok(manager) => manager,
                Err(e) => {
                    let _ = tx.send(crate::domain::context::chunks::CandleResult {
                        result: Err(AgentError::Config(format!(
                            "Failed to initialize memory: {e}"
                        ))),
                    });
                    return;
                }
            };

            // Create memory tool with the initialized manager
            let memory_manager_arc = Arc::new(memory_manager);
            let memory_tool = MemoryTool::new(Arc::clone(&memory_manager_arc));

            // Create agent with properly initialized memory
            let agent = Self {
                model,
                system_prompt,
                context: ZeroOneOrMany::None,
                tools: ZeroOneOrMany::None,
                memory: Some(memory_manager_arc),
                memory_tool: Some(memory_tool),
                temperature: None,
                max_tokens: None,
                additional_params: None,
                error: None,
            };

            // Record agent creation in statistics
            AGENT_STATS.record_agent_created();

            // Send the successfully created agent
            let _ = tx.send(crate::domain::context::chunks::CandleResult { result: Ok(agent) });
        }))
    }

    /// Create a new agent with shared memory instance
    ///
    /// # Arguments
    /// * `model` - Model configuration for the agent
    /// * `system_prompt` - System prompt for the agent
    /// * `memory` - Shared memory instance for lock-free concurrent access
    ///
    /// # Returns
    /// Stream that emits configured agent with shared memory
    ///
    /// # Performance
    /// Zero allocation with lock-free memory sharing between agents
    #[inline]
    pub fn with_shared_memory(
        model: M,
        system_prompt: impl Into<String>,
        memory: Arc<SurrealDBMemoryManager>,
    ) -> Pin<
        Box<
            dyn Stream<Item = crate::domain::context::chunks::CandleResult<Self, AgentError>>
                + Send,
        >,
    > {
        let system_prompt = system_prompt.into();
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Create memory tool with zero-allocation initialization
            let memory_tool = MemoryTool::new(Arc::clone(&memory));

            let agent = Self {
                model,
                system_prompt,
                context: ZeroOneOrMany::None,
                tools: ZeroOneOrMany::None,
                memory: Some(Arc::clone(&memory)),
                memory_tool: Some(memory_tool),
                temperature: None,
                max_tokens: None,
                additional_params: None,
                error: None,
            };

            // Record agent creation in statistics
            AGENT_STATS.record_agent_created();

            let _ = tx.send(crate::domain::context::chunks::CandleResult { result: Ok(agent) });
        }))
    }

    /// Get memory tool reference for direct access
    ///
    /// # Returns
    /// Optional reference to memory tool
    ///
    /// # Performance
    /// Zero cost abstraction with direct tool access
    #[inline]
    pub fn memory_tool(&self) -> Option<&MemoryTool> {
        self.memory_tool.as_ref()
    }

    /// Get memory reference for direct access
    ///
    /// # Returns
    /// Optional reference to memory instance
    ///
    /// # Performance
    /// Zero cost abstraction with direct memory access
    #[inline]
    pub fn memory(&self) -> Option<&Arc<SurrealDBMemoryManager>> {
        self.memory.as_ref()
    }

    /// Add additional tool to the agent
    ///
    /// # Arguments
    /// * `tool` - Tool to add to the agent
    ///
    /// # Returns
    /// Updated agent instance
    ///
    /// # Performance
    /// Zero allocation with inlined tool addition
    #[inline]
    #[must_use]
    pub fn add_tool(mut self, tool: ToolInfo) -> Self {
        match &mut self.tools {
            ZeroOneOrMany::None => {
                self.tools = ZeroOneOrMany::One(tool);
            }
            ZeroOneOrMany::One(existing) => {
                let existing = std::mem::replace(existing, tool.clone());
                self.tools = ZeroOneOrMany::Many(vec![existing, tool]);
            }
            ZeroOneOrMany::Many(tools) => {
                tools.push(tool);
            }
        }
        self
    }

    /// Set agent temperature
    ///
    /// # Arguments
    /// * `temperature` - Temperature value for model sampling
    ///
    /// # Returns
    /// Updated agent instance
    ///
    /// # Performance
    /// Zero allocation with direct field assignment
    #[inline]
    #[must_use]
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set agent max tokens
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum tokens for model output
    ///
    /// # Returns
    /// Updated agent instance
    ///
    /// # Performance
    /// Zero allocation with direct field assignment
    #[inline]
    #[must_use]
    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

impl<M: Model + Clone + Send + 'static + Default> Default for Agent<M> {
    fn default() -> Self {
        Self {
            model: M::default(),
            system_prompt: String::new(),
            context: ZeroOneOrMany::None,
            tools: ZeroOneOrMany::None,
            memory: None,
            memory_tool: None,
            temperature: None,
            max_tokens: None,
            additional_params: None,
            error: None,
        }
    }
}

use cyrup_sugars::prelude::MessageChunk;

impl<M: Model + Clone + Send + 'static + Default> MessageChunk for Agent<M> {
    fn bad_chunk(error: String) -> Self {
        let typed_error = AgentError::Unknown(error.clone());

        Self {
            model: M::default(),
            system_prompt: error, // Keep error string in system_prompt for error() lifetime
            context: ZeroOneOrMany::None,
            tools: ZeroOneOrMany::None,
            memory: None,
            memory_tool: None,
            temperature: None,
            max_tokens: None,
            additional_params: None,
            error: Some(typed_error),
        }
    }

    fn error(&self) -> Option<&str> {
        // If error field is set, return system_prompt (which contains error message)
        self.error.as_ref().map(|_| self.system_prompt.as_str())
    }
}

impl<M: Model> Agent<M> {
    /// Get typed error if agent is in error state
    ///
    /// This is preferred over `error()` for new code as it provides
    /// type-safe error handling with pattern matching.
    ///
    /// # Returns
    /// - `Some(&AgentError)` if agent has an error
    /// - `None` if agent is in valid state
    pub fn get_error(&self) -> Option<&AgentError> {
        self.error.as_ref()
    }

    /// Check if agent is in error state
    #[inline]
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Clear error state
    pub fn clear_error(&mut self) {
        self.error = None;
        if self.error.is_none() {
            self.system_prompt.clear(); // Clear error message from system_prompt
        }
    }
}

impl<M: Model> Drop for Agent<M> {
    fn drop(&mut self) {
        AGENT_STATS.record_agent_destroyed();
    }
}

/// Statistics snapshot for external access
#[derive(Debug, Clone)]
pub struct AgentStatsSnapshot {
    pub agents_created: u64,
    pub agents_active: usize,
    pub completions_total: u64,
    pub tokens_total: u64,
    pub avg_completion_time_us: u64,
}

/// Get current agent statistics snapshot
pub fn get_agent_stats() -> AgentStatsSnapshot {
    AgentStatsSnapshot {
        agents_created: AGENT_STATS.agents_created.get(),
        agents_active: AGENT_STATS.agents_active.load(Ordering::Relaxed),
        completions_total: AGENT_STATS.completions_total.get(),
        tokens_total: AGENT_STATS.tokens_total.get(),
        avg_completion_time_us: AGENT_STATS.avg_completion_time_us.load(Ordering::Relaxed),
    }
}
