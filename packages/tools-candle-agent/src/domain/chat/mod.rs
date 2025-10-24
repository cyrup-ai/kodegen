//! Chat syntax features module
//!
//! This module provides comprehensive chat syntax features with zero-allocation patterns
//! and production-ready functionality. All submodules follow blazing-fast, lock-free,
//! and elegant ergonomic design principles.
//!
//! ## Features
//! - Rich message formatting with SIMD-optimized parsing
//! - Command system with slash commands and auto-completion
//! - Template and macro system for reusable patterns
//! - Advanced configuration with nested settings
//! - Real-time features with atomic state management
//! - Enhanced history management with full-text search

//!
//! ## Architecture
//! All modules use zero-allocation patterns with String for string sharing,
//! crossbeam-skiplist for lock-free data structures, and atomic operations
//! for thread-safe state management.

pub mod commands;
pub mod config;
pub mod conversation;
pub mod export;
pub mod formatting;
pub mod orchestration;

pub mod r#loop;
pub mod macros;
pub mod message;
pub mod realtime;
pub mod search;
pub mod session;
pub mod templates;
pub mod types;

// Re-export types with corrected names to avoid ambiguous glob re-exports
pub use commands::{
    CommandExecutor as CandleCommandExecutor, CommandRegistry as CandleCommandRegistry,
    ImmutableChatCommand as CandleImmutableChatCommand,
};
pub use config::{CandleChatConfig, CandlePersonalityConfig};
pub use conversation::CandleConversationEvent as CandleConversation;
pub use export::{ExportData as CandleExportData, ExportFormat as CandleExportFormat};
pub use formatting::{
    FormatStyle as CandleFormatStyle, StreamingMessageFormatter as CandleStreamingMessageFormatter,
};

pub use r#loop::CandleChatLoop;
pub use macros::{
    ChatMacro as CandleChatMacro, MacroAction as CandleMacroAction,
    MacroExecutionConfig as CandleMacroExecutionConfig, MacroMetadata as CandleMacroMetadata,
    MacroSystem as CandleMacroSystem, MacroSystemError as CandleMacroSystemError,
};
pub use message::message_processing::{
    process_message as candle_process_message, sanitize_content as candle_sanitize_content,
    validate_message as candle_validate_message,
    validate_message_sync as candle_validate_message_sync,
};
pub use message::types::{CandleMessage, CandleMessageChunk, CandleMessageRole};
pub use realtime::RealTimeSystem as CandleRealTimeSystem;
pub use search::{
    CandleConversationTag, CandleConversationTagger, CandleEnhancedHistoryManager,
    CandleTaggingStatistics, ChatSearchIndex as CandleChatSearchIndex,
    HistoryExporter as CandleHistoryExporter, QueryProcessor as CandleQueryProcessor,
    ResultRanker as CandleResultRanker, SearchExporter as CandleSearchExporter,
    SearchQuery as CandleSearchQuery, SearchStatistics as CandleSearchStatistics,
};
pub use session::{
    ChatSessionConfig, ChatSessionContexts, ChatSessionHandlers, execute_chat_session,
};
pub use templates::{
    ChatTemplate as CandleChatTemplate, TemplateCategory as CandleTemplateCategory,
    TemplateManager as CandleTemplateManager,
};
pub use types::responses::{
    FinalResponse as CandleFinalResponse, FunctionCall as CandleFunctionCall,
    OpenAIFunctionCallResponse as CandleOpenAIFunctionCallResponse, ToolCall as CandleToolCall,
    ToolSelectionResponse as CandleToolSelectionResponse,
};

// ============================================================================
// TYPE MIGRATION MAPPING
// ============================================================================
// Documentation of type equivalencies between original chat system (./src/chat/)
// and domain chat system (./src/domain/chat/) for migration reference.
//
// ORIGINAL SYSTEM TYPES → DOMAIN SYSTEM TYPES (Candle-prefixed)
// ==============================================================
//
// Core Message Types:
// - Message → CandleMessage
// - MessageChunk → CandleMessageChunk
// - MessageRole → CandleMessageRole
//
// Command System Types:
// - CommandExecutor → CandleCommandExecutor
// - CommandRegistry → CandleCommandRegistry
// - ImmutableChatCommand → CandleImmutableChatCommand
//
// Configuration Types:
// - ChatConfig → CandleChatConfig
// - PersonalityConfig → CandlePersonalityConfig
//
// Conversation Types:
// - CandleConversation (event type)
// - CandleStreamingConversation (primary conversation implementation)
// - CandleAgentConversation (agent workflow conversations)
//
// Export/Format Types:
// - ExportData → CandleExportData
// - ExportFormat → CandleExportFormat
//
// Formatting Types:
// - FormatStyle → CandleFormatStyle
// - StreamingMessageFormatter → CandleStreamingMessageFormatter
//

// Macro System Types:
// - MacroAction → CandleMacroAction
// - MacroSystem → CandleMacroSystem
// - ChatMacro → CandleChatMacro (domain-only enhanced type)
// - MacroExecutionConfig → CandleMacroExecutionConfig (domain-only enhanced type)
// - MacroMetadata → CandleMacroMetadata (domain-only enhanced type)
// - MacroSystemError → CandleMacroSystemError (domain-only enhanced type)
//
// Search System Types:
// - ChatSearchIndex → CandleChatSearchIndex
// - SearchQuery → CandleSearchQuery
// - SearchStatistics → CandleSearchStatistics (domain-only enhanced type)
// - ConversationTagger → CandleConversationTagger (domain-only enhanced type)
// - HistoryExporter → CandleHistoryExporter (domain-only enhanced type)
// - EnhancedHistoryManager → CandleEnhancedHistoryManager (domain-only enhanced type)
// - HistoryManagerStatistics → CandleHistoryManagerStatistics (domain-only enhanced type)
//
// Template System Types:
// - ChatTemplate → CandleChatTemplate
// - TemplateManager → CandleTemplateManager
// - TemplateCategory → CandleTemplateCategory (domain-only enhanced type)
//
// Real-time System Types:
// - RealTimeSystem → CandleRealTimeSystem (domain-only enhanced type)
//
// CRITICAL MIGRATION TARGET:
// - ChatLoop → CandleChatLoop (domain system connected to agent builders)
//
// NAMING CONVENTION:
// All domain system types MUST use 'Candle' prefix for consistency and
// to avoid naming conflicts during migration phase.
//
// MIGRATION PHASES:
// Phase 1: Update imports to use domain system types
// Phase 2: Migrate superior features from original to domain system
// Phase 3: Remove original system entirely
// ============================================================================
