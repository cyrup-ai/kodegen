//! Prompt builder implementation moved from domain
//! Builders are behavioral/construction logic, separate from core domain models

use crate::prompt::Prompt;

pub struct PromptBuilder {
    content: String}

impl Prompt {
    // Semantic entry point
    pub fn ask(content: impl Into<String>) -> PromptBuilder {
        PromptBuilder {
            content: content.into()}
    }
}

impl Into<Prompt> for PromptBuilder {
    fn into(self) -> Prompt {
        Prompt::new(self.content)
    }
}
