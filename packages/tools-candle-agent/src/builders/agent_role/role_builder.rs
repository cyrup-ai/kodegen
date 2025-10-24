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
        // Create default tools: thinking and reasoning plugins
        let thinking_tool = ToolInfo {
            name: "thinking".to_string(),
            description: Some("Use this tool for all thinking and reasoning tasks. The tool accepts a list of user and previous assistant messages relevant to the conversation. Always call this tool before answering the user and include the latest user message in the list. The tool will generate a chain of thought reasoning which can be used to answer the user's question.".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "messages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "role": {
                                    "type": "string",
                                    "enum": ["user", "assistant"]
                                },
                                "content": {
                                    "type": "string"
                                }
                            },
                            "required": ["role", "content"]
                        }
                    }
                },
                "required": ["messages"]
            }),
        };

        let reasoner_tool = ToolInfo {
            name: "mcp-reasoner".to_string(),
            description: Some("Advanced reasoning tool with Beam Search and MCTS strategies for complex problem solving".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "thought": {"type": "string", "description": "Current reasoning step"},
                    "thoughtNumber": {"type": "integer", "description": "Current step number", "minimum": 1},
                    "totalThoughts": {"type": "integer", "description": "Total expected steps", "minimum": 1},
                    "nextThoughtNeeded": {"type": "boolean", "description": "Whether another step is needed"},
                    "parentId": {"type": ["string", "null"], "description": "Optional parent thought ID for branching"},
                    "strategyType": {"type": ["string", "null"], "enum": ["beam_search", "mcts", "mcts_002_alpha", "mcts_002alt_alpha", null], "description": "Reasoning strategy to use"},
                    "beamWidth": {"type": ["integer", "null"], "description": "Number of top paths to maintain", "minimum": 1, "maximum": 10},
                    "numSimulations": {"type": ["integer", "null"], "description": "Number of MCTS simulations", "minimum": 1, "maximum": 150}
                },
                "required": ["thought", "thoughtNumber", "totalThoughts", "nextThoughtNeeded"]
            }),
        };

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
            tools: ZeroOneOrMany::from(vec![thinking_tool, reasoner_tool]),
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
