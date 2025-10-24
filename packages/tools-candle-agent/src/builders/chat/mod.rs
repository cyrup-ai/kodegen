//! Chat-related builders extracted from cyrup_domain

pub mod conversation_builder;
pub mod history_manager_builder;
pub mod macro_builder;
pub mod template_builder;

// Re-export for convenience
pub use conversation_builder::ConversationBuilder;
pub use history_manager_builder::HistoryManagerBuilder;
pub use macro_builder::MacroBuilder;
pub use template_builder::TemplateBuilder;