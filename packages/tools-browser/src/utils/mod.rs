mod agent_state;
mod deep_research;
mod errors;
mod llm;
mod timeout;
mod utils;

pub use agent_state::AgentState;
pub use deep_research::{DeepResearch, ResearchResult, ResearchOptions};
pub use errors::UtilsError;
pub use llm::LlmConfig;
pub use timeout::{
    validate_navigation_timeout,
    validate_interaction_timeout, 
    validate_wait_timeout,
    MAX_NAVIGATION_TIMEOUT_MS,
    MAX_INTERACTION_TIMEOUT_MS,
    MAX_WAIT_TIMEOUT_MS,
};
pub use utils::{llama, encode_image, get_latest_files, capture_screenshot};

/// Result type for utility functions
pub type UtilsResult<T> = Result<T, UtilsError>;
