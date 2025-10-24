//! Message builder implementations
//!
//! All message construction logic and builder patterns.

use crate::domain::chat::message::{CandleMessage as Message}; // Core message types from chat
use crate::domain::{CandleAudio as Audio, CandleDocument as Document, context::CandleImageChunk as Image};
use serde::Serialize;
use serde_json::Value;

/// Message builder trait - defines the interface for building messages
pub trait MessageBuilder {
    type Content: Content;
    fn add_content(self, content: Self::Content) -> Self;
    fn build(self) -> Message;
}

/// User message builder trait - extends MessageBuilder with user-specific methods
pub trait UserMessageBuilderTrait: MessageBuilder<Content = UserContent> {
    fn text(self, text: impl Into<String>) -> Self;
    fn image(self, image: Image) -> Self;
    fn audio(self, audio: Audio) -> Self;
    fn document(self, document: Document) -> Self;
    fn say(self) -> Message;
}

/// Assistant message builder trait - extends MessageBuilder with assistant-specific methods
pub trait AssistantMessageBuilderTrait: MessageBuilder<Content = AssistantContent> {
    fn text(self, text: impl Into<String>) -> Self;
    fn tool_call(self, id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self;
    fn tool_result(self, tool_call_id: impl Into<String>, result: Value) -> Self;
    fn tool_error(self, tool_call_id: impl Into<String>, error: impl Into<String>) -> Self;
    fn respond(self) -> Message;
}

/// Message factory trait - for creating message builders
pub trait MessageFactory {
    fn user_message() -> impl UserMessageBuilderTrait;
    fn assistant_message() -> impl AssistantMessageBuilderTrait;
}

/// Concrete user message builder implementation
pub struct UserMessageBuilder {
    content: Option<UserContent>}

/// Concrete assistant message builder implementation
pub struct AssistantMessageBuilder {
    content: Option<AssistantContent>}

impl UserMessageBuilder {
    pub fn new() -> Self {
        Self { content: None }
    }
}

impl AssistantMessageBuilder {
    pub fn new() -> Self {
        Self { content: None }
    }
}

impl MessageBuilder for UserMessageBuilder {
    type Content = UserContent;

    fn add_content(mut self, content: Self::Content) -> Self {
        self.content = Some(content);
        self
    }

    fn build(self) -> Message {
        Message::user(
            self.content
                .unwrap_or_else(|| {
                    UserContent::Text(crate::domain::chat::message::Text {
                        content: "".to_string()})
                })
                .as_text(),
        )
    }
}

impl UserMessageBuilderTrait for UserMessageBuilder {
    fn text(mut self, text: impl Into<String>) -> Self {
        self.content = Some(UserContent::Text(crate::domain::chat::message::Text {
            content: text.into()}));
        self
    }

    fn image(mut self, image: Image) -> Self {
        // Convert image to user content format
        self.content = Some(UserContent::Image {
            url: image.data,
            detail: None});
        self
    }

    fn audio(mut self, audio: Audio) -> Self {
        // Convert audio to text for user content
        self.content = Some(UserContent::Text(crate::domain::chat::message::Text {
            content: format!("[Audio: {}]", audio.data)}));
        self
    }

    fn document(mut self, document: Document) -> Self {
        // Convert document to text for user content
        self.content = Some(UserContent::Text(crate::domain::chat::message::Text {
            content: document.content}));
        self
    }

    fn say(self) -> Message {
        self.build()
    }
}

impl MessageBuilder for AssistantMessageBuilder {
    type Content = AssistantContent;

    fn add_content(mut self, content: Self::Content) -> Self {
        self.content = Some(content);
        self
    }

    fn build(self) -> Message {
        Message::assistant(
            self.content
                .unwrap_or_else(|| {
                    AssistantContent::Text(crate::domain::chat::message::Text {
                        content: "".to_string()})
                })
                .as_text(),
        )
    }
}

impl AssistantMessageBuilderTrait for AssistantMessageBuilder {
    fn text(mut self, text: impl Into<String>) -> Self {
        self.content = Some(AssistantContent::Text(crate::domain::chat::message::Text {
            content: text.into()}));
        self
    }

    fn tool_call(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: Value,
    ) -> Self {
        let tool_call = ToolCall {
            id: id.into(),
            function: crate::domain::chat::message::ToolFunction {
                name: name.into(),
                description: None,
                parameters: arguments}};
        self.content = Some(AssistantContent::ToolCall(tool_call));
        self
    }

    fn tool_result(mut self, tool_call_id: impl Into<String>, result: Value) -> Self {
        self.content = Some(AssistantContent::Text(crate::domain::chat::message::Text {
            content: format!("Tool result for {}: {}", tool_call_id.into(), result)}));
        self
    }

    fn tool_error(mut self, tool_call_id: impl Into<String>, error: impl Into<String>) -> Self {
        self.content = Some(AssistantContent::Text(crate::domain::chat::message::Text {
            content: format!("Tool error for {}: {}", tool_call_id.into(), error.into())}));
        self
    }

    fn respond(self) -> Message {
        self.build()
    }
}

/// Default message factory implementation
pub struct DefaultMessageFactory;

impl MessageFactory for DefaultMessageFactory {
    fn user_message() -> impl UserMessageBuilderTrait {
        UserMessageBuilder::new()
    }

    fn assistant_message() -> impl AssistantMessageBuilderTrait {
        AssistantMessageBuilder::new()
    }
}
