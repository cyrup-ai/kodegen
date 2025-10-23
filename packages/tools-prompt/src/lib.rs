pub mod metadata;
pub mod template;
pub mod validation;
pub mod manager;
mod defaults;

pub mod add_prompt;
pub use add_prompt::*;

pub mod edit_prompt;
pub use edit_prompt::*;

pub mod delete_prompt;
pub use delete_prompt::*;

pub mod get_prompt;
pub use get_prompt::*;

// Re-export commonly used types
pub use metadata::{PromptMetadata, PromptTemplate, ParameterDefinition, ParameterType};
pub use manager::PromptManager;
