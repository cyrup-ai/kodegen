use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use cyrup_sugars::OneOrMany;
use crate::domain::chat::{
    CandleChatMacro as ChatMacro, 
    CandleMacroAction as MacroAction, 
    CandleMacroExecutionConfig as MacroExecutionConfig, 
    CandleMacroMetadata as MacroMetadata, 
    CandleMacroSystemError as MacroSystemError
};

/// Builder for creating macros programmatically
pub struct MacroBuilder {
    name: Option<Arc<str>>,
    description: Option<Arc<str>>,
    actions: OneOrMany<MacroAction>,
    variables: HashMap<Arc<str>, Arc<str>>,
    triggers: OneOrMany<Arc<str>>,
    conditions: OneOrMany<Arc<str>>,
    dependencies: OneOrMany<Arc<str>>,
    execution_config: MacroExecutionConfig,
}

impl MacroBuilder {
    /// Create a new macro builder
    pub fn new() -> Self {
        Self {
            name: None,
            description: None,
            actions: OneOrMany::None,
            variables: HashMap::new(),
            triggers: OneOrMany::None,
            conditions: OneOrMany::None,
            dependencies: OneOrMany::None,
            execution_config: MacroExecutionConfig::default(),
        }
    }

    /// Set the macro name
    pub fn name(mut self, name: Arc<str>) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the macro description
    pub fn description(mut self, description: Arc<str>) -> Self {
        self.description = Some(description);
        self
    }

    /// Add an action to the macro
    pub fn add_action(mut self, action: MacroAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Add a variable to the macro
    pub fn add_variable(mut self, name: Arc<str>, value: Arc<str>) -> Self {
        self.variables.insert(name, value);
        self
    }

    /// Set execution configuration
    pub fn execution_config(mut self, config: MacroExecutionConfig) -> Self {
        self.execution_config = config;
        self
    }

    /// Build the macro
    pub fn build(self) -> Result<ChatMacro, MacroSystemError> {
        let name = self
            .name
            .ok_or_else(|| MacroSystemError::ExecutionError("Name is required".to_string()))?;
        let description = self.description.unwrap_or_else(|| Arc::from(""));

        let metadata = MacroMetadata {
            id: Uuid::new_v4(),
            name,
            description,
            created_at: Duration::from_secs(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| MacroSystemError::SystemTimeError)?
                    .as_secs(),
            ),
            updated_at: Duration::from_secs(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| MacroSystemError::SystemTimeError)?
                    .as_secs(),
            ),
            version: 1,
            tags: Arc::new([]),
            author: Arc::from("builder"),
            execution_count: 0,
            last_execution: None,
            average_duration: Duration::from_secs(0),
            success_rate: 0.0,
            category: Arc::from("programmatic"),
            is_private: false,
        };

        Ok(ChatMacro {
            metadata,
            actions: self.actions.into(),
            variables: self.variables,
            triggers: self.triggers.into(),
            conditions: self.conditions.into(),
            dependencies: self.dependencies.into(),
            execution_config: self.execution_config,
        })
    }
}

impl Default for MacroBuilder {
    fn default() -> Self {
        Self::new()
    }
}