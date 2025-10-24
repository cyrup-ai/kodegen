//! Zero-allocation agent builder with const generics

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::pin::Pin;

use arrayvec::ArrayVec;
use tokio_stream::Stream;

use super::core::{Agent, AgentError, AgentResult, MAX_AGENT_TOOLS};
use crate::memory::Memory;
use crate::memory::config::memory::MemoryConfig as ComprehensiveMemoryConfig;
use crate::memory::manager::MemoryConfig;
use crate::domain::model::traits::CandleModel;
use crate::domain::completion::types::ToolInfo;

/// Agent statistics for performance monitoring
static AGENT_STATS: AtomicUsize = AtomicUsize::new(0);

/// Zero-allocation agent builder with const generics
#[derive(Debug)]
pub struct AgentBuilder<const TOOLS_CAPACITY: usize = MAX_AGENT_TOOLS> {
    model: Option<&'static dyn CandleModel>,
    system_prompt: Option<String>,
    memory_config: Option<ComprehensiveMemoryConfig>,
    shared_memory: Option<Arc<Memory>>,
    tools: ArrayVec<ToolInfo, TOOLS_CAPACITY>,
    temperature: Option<f64>,
    max_tokens: Option<u64>}

impl<const TOOLS_CAPACITY: usize> Default for AgentBuilder<TOOLS_CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const TOOLS_CAPACITY: usize> AgentBuilder<TOOLS_CAPACITY> {
    /// Create new agent builder with zero-allocation initialization
    pub fn new() -> Self {
        Self {
            model: None,
            system_prompt: None,
            memory_config: None,
            shared_memory: None,
            tools: ArrayVec::new(),
            temperature: None,
            max_tokens: None}
    }

    /// Set model with validation
    pub fn model(mut self, model: &'static dyn CandleModel) -> Self {
        self.model = Some(model);
        self
    }

    /// Set system prompt
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set memory configuration
    pub fn memory_config(mut self, config: ComprehensiveMemoryConfig) -> Self {
        self.memory_config = Some(config);
        self
    }

    /// Set shared memory instance
    pub fn shared_memory(mut self, memory: Arc<Memory>) -> Self {
        self.shared_memory = Some(memory);
        self
    }

    /// Add tool with zero-allocation error handling
    pub fn tool(mut self, tool: ToolInfo) -> AgentResult<Self> {
        if self.tools.try_push(tool).is_err() {
            return Err(AgentError::Config(format!(
                "Tool capacity exceeded. Maximum {} tools allowed.",
                TOOLS_CAPACITY
            )));
        }
        Ok(self)
    }

    /// Set temperature with validation
    pub fn temperature(mut self, temperature: f64) -> AgentResult<Self> {
        if !(0.0..=2.0).contains(&temperature) {
            return Err(AgentError::Config(format!(
                "Temperature {} is outside valid range [0.0, 2.0]",
                temperature
            )));
        }
        self.temperature = Some(temperature);
        Ok(self)
    }

    /// Set max tokens with validation
    pub fn max_tokens(mut self, max_tokens: u64) -> AgentResult<Self> {
        if !(1..=100_000).contains(&max_tokens) {
            return Err(AgentError::Config(format!(
                "Max tokens {} is outside valid range [1, 100000]",
                max_tokens
            )));
        }
        self.max_tokens = Some(max_tokens);
        Ok(self)
    }

    /// Build agent with comprehensive error handling
    pub fn build(self) -> Pin<Box<dyn Stream<Item = AgentResult<Agent>> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|stream_sender| async move {
            let result = (|| {
                // Validate required fields
                let model = self
                    .model
                    .ok_or_else(|| AgentError::Config("Model is required".to_string()))?;
                let system_prompt = self
                    .system_prompt
                    .unwrap_or_else(|| "You are a helpful AI assistant.".to_string());

                // Increment atomic counter for lock-free statistics
                AGENT_STATS.fetch_add(1, Ordering::Relaxed);

                // Initialize memory system
                let memory = if let Some(shared_memory) = self.shared_memory {
                    shared_memory
                } else {
                    let comprehensive_config = self.memory_config.unwrap_or_default();
                    // Convert comprehensive config to Memory::new() format
                    let memory_config = MemoryConfig {
                        database_url: comprehensive_config.database.connection_string.to_string(),
                        embedding_dimension: comprehensive_config.vector_store.dimension};
                    let mut memory_stream = Memory::new(memory_config);

                    // Use stream's try_next method
                    let memory_instance = match memory_stream.try_next() {
                        Some(memory) => memory,
                        None => {
                            return Err(AgentError::InitializationError(
                                "Failed to initialize memory".to_string(),
                            ));
                        }
                    };
                    let memory_arc = Arc::new(memory_instance);
                    memory_arc
                };

                // Create memory tool
                let memory_tool = crate::memory::MemoryTool::new(memory.clone());

                // Convert tools with zero-allocation
                let tools = match self.tools.len() {
                    0 => crate::ZeroOneOrMany::None,
                    1 => match self.tools.into_iter().next() {
                        Some(tool) => crate::ZeroOneOrMany::One(tool),
                        None => {
                            return Err(AgentError::InitializationError(
                                "Expected one tool but found none".to_string(),
                            ));
                        }
                    },
                    _ => {
                        let tools_vec: Vec<_> = self.tools.into_iter().collect();
                        crate::ZeroOneOrMany::Many(tools_vec)
                    }
                };

                Ok(Agent {
                    model,
                    system_prompt,
                    context: crate::ZeroOneOrMany::None,
                    tools,
                    memory: Some((*memory).clone()),
                    memory_tool: Some(memory_tool),
                    temperature: self.temperature,
                    max_tokens: self.max_tokens,
                    additional_params: None})
            })();
            
            let _ = stream_sender.send(result);
        }))
    }
}

/// Create agent builder with default capacity
#[inline]
pub fn agent_builder() -> AgentBuilder {
    AgentBuilder::new()
}

/// Create agent builder with custom capacity
#[inline]
pub fn agent_builder_with_capacity<const CAPACITY: usize>() -> AgentBuilder<CAPACITY> {
    AgentBuilder::new()
}
