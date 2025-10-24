//! Prompt formatting with memory vs context sectioning
//!
//! Handles proper sectioning of prompts to distinguish memories from static context,
//! following LLM best practices for attention patterns and information clarity.

use crate::domain::chat::message::types::CandleMessage as ChatMessage;
use crate::domain::context::CandleDocument as Document;
use crate::memory::core::ops::retrieval::RetrievalResult;
use cyrup_sugars::ZeroOneOrMany;

/// Prompt formatter that creates sectioned prompts distinguishing memories from context
#[derive(Debug, Clone)]
pub struct PromptFormatter {
    /// Whether to include section headers for clarity
    pub include_headers: bool,
    /// Maximum length for memory section (to avoid context overflow)
    pub max_memory_length: Option<usize>,
    /// Maximum length for context section
    pub max_context_length: Option<usize>,
}

impl Default for PromptFormatter {
    fn default() -> Self {
        Self {
            include_headers: true,
            max_memory_length: Some(2000),
            max_context_length: Some(4000),
        }
    }
}

impl PromptFormatter {
    /// Create a new prompt formatter
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure whether to include section headers
    #[must_use]
    pub fn with_headers(mut self, include_headers: bool) -> Self {
        self.include_headers = include_headers;
        self
    }

    /// Set maximum memory section length
    #[must_use]
    pub fn with_max_memory_length(mut self, max_length: Option<usize>) -> Self {
        self.max_memory_length = max_length;
        self
    }

    /// Set maximum context section length
    #[must_use]
    pub fn with_max_context_length(mut self, max_length: Option<usize>) -> Self {
        self.max_context_length = max_length;
        self
    }

    /// Format a complete prompt with proper memory vs context sectioning
    ///
    /// # Arguments
    /// * `memories` - Retrieved memories from previous conversations
    /// * `documents` - Static context documents
    /// * `chat_history` - Conversation history
    /// * `user_message` - Current user message
    ///
    /// # Returns
    /// Formatted prompt with clear sectioning for LLM understanding
    ///
    /// # Memory vs Context Sectioning
    /// Follows best practices for LLM attention patterns:
    /// - Memories are prepended to user prompt (not system prompt)
    /// - Clear sectioning helps LLM distinguish information sources
    /// - U-shaped attention pattern places important info at beginning/end
    #[must_use]
    pub fn format_prompt(
        &self,
        system_prompt: Option<&str>, // Added system prompt parameter
        memories: &ZeroOneOrMany<RetrievalResult>,
        documents: &ZeroOneOrMany<Document>,
        chat_history: &ZeroOneOrMany<ChatMessage>,
        user_message: &str,
    ) -> String {
        let mut prompt_parts = Vec::new();

        // 1. SYSTEM PROMPT FIRST (most important for LLM attention)
        if let Some(sys_prompt) = system_prompt {
            if self.include_headers {
                prompt_parts.push(format!("--- SYSTEM INSTRUCTIONS ---\n{sys_prompt}"));
            } else {
                prompt_parts.push(sys_prompt.to_string());
            }
        }

        // 2. Memory section (prepended after system prompt)
        if let Some(memory_section) = self.format_memory_section(memories) {
            prompt_parts.push(memory_section);
        }

        // 3. Context section (static documents)
        if let Some(context_section) = self.format_context_section(documents) {
            prompt_parts.push(context_section);
        }

        // 4. Chat history section
        if let Some(history_section) = self.format_chat_history(chat_history) {
            prompt_parts.push(history_section);
        }

        // 5. Current user message
        prompt_parts.push(format!("User: {user_message}"));

        prompt_parts.join("\n\n")
    }

    /// Format memory section with retrieved memories
    #[must_use]
    pub fn format_memory_section(
        &self,
        memories: &ZeroOneOrMany<RetrievalResult>,
    ) -> Option<String> {
        let memory_items = match memories {
            ZeroOneOrMany::None => return None,
            ZeroOneOrMany::One(memory) => vec![memory],
            ZeroOneOrMany::Many(memories) => memories.iter().collect(),
        };

        if memory_items.is_empty() {
            return None;
        }

        let mut section = String::new();

        if self.include_headers {
            section.push_str("--- RELEVANT MEMORIES ---\n");
            section.push_str("Previous conversations and learned information:\n\n");
        }

        for (i, memory) in memory_items.iter().enumerate() {
            let memory_text = Self::format_single_memory(memory, i + 1);

            // Check length limit
            if let Some(max_len) = self.max_memory_length
                && section.len() + memory_text.len() > max_len
            {
                if self.include_headers {
                    section.push_str("[Additional memories truncated due to length limit]\n");
                }
                break;
            }

            section.push_str(&memory_text);
            section.push('\n');
        }

        Some(section.trim_end().to_string())
    }

    /// Format a single memory entry
    fn format_single_memory(memory: &RetrievalResult, index: usize) -> String {
        let content = memory
            .metadata
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("[No content available]");

        format!(
            "Memory {}: {} (relevance: {:.2})",
            index, content, memory.score
        )
    }

    /// Format context section with static documents
    fn format_context_section(&self, documents: &ZeroOneOrMany<Document>) -> Option<String> {
        let doc_items = match documents {
            ZeroOneOrMany::None => return None,
            ZeroOneOrMany::One(doc) => vec![doc],
            ZeroOneOrMany::Many(docs) => docs.iter().collect(),
        };

        if doc_items.is_empty() {
            return None;
        }

        let mut section = String::new();

        if self.include_headers {
            section.push_str("--- CONTEXT DOCUMENTS ---\n");
            section.push_str("Static reference information:\n\n");
        }

        for (i, doc) in doc_items.iter().enumerate() {
            let doc_text = Self::format_single_document(doc, i + 1);

            // Check length limit
            if let Some(max_len) = self.max_context_length
                && section.len() + doc_text.len() > max_len
            {
                if self.include_headers {
                    section.push_str("[Additional documents truncated due to length limit]\n");
                }
                break;
            }

            section.push_str(&doc_text);
            section.push('\n');
        }

        Some(section.trim_end().to_string())
    }

    /// Format a single document entry
    fn format_single_document(document: &Document, index: usize) -> String {
        let content = &document.data;
        let default_title = format!("Document {index}");
        let title = document
            .additional_props
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_title);

        format!("{title}: {content}")
    }

    /// Format chat history section
    fn format_chat_history(&self, chat_history: &ZeroOneOrMany<ChatMessage>) -> Option<String> {
        let history_items = match chat_history {
            ZeroOneOrMany::None => return None,
            ZeroOneOrMany::One(msg) => vec![msg],
            ZeroOneOrMany::Many(msgs) => msgs.iter().collect(),
        };

        if history_items.is_empty() {
            return None;
        }

        let mut section = String::new();

        if self.include_headers {
            section.push_str("--- CONVERSATION HISTORY ---\n");
        }

        for msg in history_items {
            section.push_str(&msg.role.to_string());
            section.push_str(": ");
            section.push_str(&msg.content);
            section.push('\n');
        }

        Some(section.trim_end().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_memory_context_sectioning() -> Result<(), Box<dyn std::error::Error>> {
        let formatter = PromptFormatter::new();

        // Create test memory
        let memory = RetrievalResult {
            id: "mem1".to_string(),
            score: 0.85,
            method: crate::memory::core::ops::retrieval::RetrievalMethod::Semantic,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert(
                    "content".to_string(),
                    serde_json::Value::String("User prefers coffee over tea".to_string()),
                );
                meta
            },
        };

        // Create test document
        let mut doc_metadata = HashMap::new();
        doc_metadata.insert(
            "title".to_string(),
            serde_json::Value::String("User Guide".to_string()),
        );
        let document = Document {
            data: "This is a user guide for the application".to_string(),
            format: None,
            media_type: None,
            additional_props: doc_metadata,
        };

        let memories = ZeroOneOrMany::One(memory);
        let documents = ZeroOneOrMany::One(document);
        let chat_history = ZeroOneOrMany::None;

        let result = formatter.format_prompt(
            None,
            &memories,
            &documents,
            &chat_history,
            "What drink should I have?",
        );

        // Verify sectioning
        assert!(result.contains("--- RELEVANT MEMORIES ---"));
        assert!(result.contains("--- CONTEXT DOCUMENTS ---"));
        assert!(result.contains("User prefers coffee"));
        assert!(result.contains("User Guide"));
        assert!(result.contains("User: What drink should I have?"));

        // Verify order (memories first, then context, then user message)
        let memory_pos = result
            .find("RELEVANT MEMORIES")
            .ok_or("Formatted prompt should contain 'RELEVANT MEMORIES' section")?;
        let context_pos = result
            .find("CONTEXT DOCUMENTS")
            .ok_or("Formatted prompt should contain 'CONTEXT DOCUMENTS' section")?;
        let user_pos = result
            .find("User: What drink")
            .ok_or("Formatted prompt should contain user message")?;

        assert!(memory_pos < context_pos);
        assert!(context_pos < user_pos);
        Ok(())
    }
}
