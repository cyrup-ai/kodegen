//! CandleAgentRoleBuilderImpl - builder before model

use super::*;

/// First builder - no provider yet
pub struct CandleAgentRoleBuilderImpl {
    pub(super) name: String,
    pub(super) text_to_text_model: Option<TextToTextModel>,
    pub(super) text_embedding_model: Option<TextEmbeddingModel>,
    pub(super) temperature: f64,
    pub(super) max_tokens: Option<u64>,
    pub(super) memory_read_timeout: u64,
    pub(super) system_prompt: String,
    pub(super) tools: ZeroOneOrMany<ToolInfo>,
    pub(super) context_file: Option<CandleContext<CandleFile>>,
    pub(super) context_files: Option<CandleContext<CandleFiles>>,
    pub(super) context_directory: Option<CandleContext<CandleDirectory>>,
    pub(super) context_github: Option<CandleContext<CandleGithub>>,
    pub(super) additional_params: std::collections::HashMap<String, String>,
    pub(super) metadata: std::collections::HashMap<String, String>,
    pub(super) on_chunk_handler: Option<OnChunkHandler>,
    pub(super) on_tool_result_handler: Option<OnToolResultHandler>,
    pub(super) on_conversation_turn_handler: Option<OnConversationTurnHandler>,
    pub(super) conversation_history: ZeroOneOrMany<(CandleMessageRole, String)>,
    pub(super) stop_sequences: Vec<String>,
}

impl std::fmt::Debug for CandleAgentRoleBuilderImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleAgentRoleBuilderImpl")
            .field("name", &self.name)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("memory_read_timeout", &self.memory_read_timeout)
            .field(
                "system_prompt",
                &format!(
                    "{}...",
                    &self.system_prompt[..self.system_prompt.len().min(50)]
                ),
            )
            .field("tools", &self.tools)
            .finish()
    }
}

impl CandleAgentRoleBuilderImpl {
    /// Create a new agent role builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text_to_text_model: None,
            text_embedding_model: None,
            temperature: 0.0,
            max_tokens: None,
            memory_read_timeout: 5000,
            system_prompt: r#"# Well-Informed Software Architect

You think out loud as you work through problems, sharing your process in addition to the solutions.
You track every task you do or needs doing in `TODO.md` , updating it religiously before and after a meaningful change to code.
You maintain `ARCHITECTURE.md`  and carefully curate the vision for the modules we create.
You prototype exploratory code ideas, quickly putting together a prototype, so we talk about the "heart of the matter" and get on the same page.
If you don't know the answer, you ALWAYS RESEARCH on the web and talk it through with me. You know that planned work takes less time in the end that hastily forged code. You never pretend to have answers unless you are highly confident.
You produce clean, maintainable, *production quality* code all the time.
You are a master at debugging and fixing bugs.
You are a master at refactoring code, remembering to check for code that ALREADY EXISTS before writing new code that might duplicate existing functionality."#.to_string(),
            tools: ZeroOneOrMany::None,
            context_file: None,
            context_files: None,
            context_directory: None,
            context_github: None,
            additional_params: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
            on_chunk_handler: None,
            on_tool_result_handler: None,
            on_conversation_turn_handler: None,
            conversation_history: ZeroOneOrMany::None,
            stop_sequences: Vec::new(),
        }
    }
}
