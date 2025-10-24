use cyrup_sugars::OneOrMany;
use crate::domain::chat::message::types::CandleMessageRole as MessageRole;
use crate::domain::chat::conversation::CandleStreamingConversation as StreamingConversation;

/// Builder for creating conversations
pub struct ConversationBuilder {
    enable_streaming: bool,
    initial_messages: OneOrMany<(String, MessageRole)>,
}

impl ConversationBuilder {
    /// Create a new conversation builder
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable event streaming
    #[inline]
    pub fn with_streaming(mut self) -> Self {
        self.enable_streaming = true;
        self
    }

    /// Add initial user message
    #[inline]
    pub fn with_user_message(mut self, message: impl Into<String>) -> Self {
        self.initial_messages = self.initial_messages
            .with_pushed((message.into(), MessageRole::User));
        self
    }

    /// Add initial assistant message
    #[inline]
    pub fn with_assistant_message(mut self, message: impl Into<String>) -> Self {
        self.initial_messages = self.initial_messages
            .with_pushed((message.into(), MessageRole::Assistant));
        self
    }

    /// Add initial system message
    #[inline]
    pub fn with_system_message(mut self, message: impl Into<String>) -> Self {
        self.initial_messages = self.initial_messages
            .with_pushed((message.into(), MessageRole::System));
        self
    }

    /// Build the conversation
    #[inline]
    pub fn build(self) -> StreamingConversation {
        let mut conversation = if self.enable_streaming {
            let (conv, _stream) = StreamingConversation::with_streaming();
            conv
        } else {
            StreamingConversation::new()
        };

        // Add initial messages
        for (content, role) in self.initial_messages {
            let _ = conversation.add_message(&content, role);
        }

        conversation
    }
}

impl Default for ConversationBuilder {
    fn default() -> Self {
        Self {
            enable_streaming: false,
            initial_messages: OneOrMany::None,
        }
    }
}