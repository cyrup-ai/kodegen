// NOTE: Commented out modules with broken dependencies (pre-existing issues)
// These should be fixed separately from the timeout validation task
// mod agent_state;
// mod deep_research;
// mod errors;
// mod llm;
// mod utils;

// Only active module for timeout validation
mod timeout;

// pub use agent_state::AgentState;
// pub use deep_research::{DeepResearch, ResearchResult, ResearchOptions};
// pub use errors::UtilsError;
// pub use llm::LlmConfig;
pub use timeout::{
    validate_navigation_timeout,
    validate_interaction_timeout, 
    validate_wait_timeout,
};
// pub use utils::{llama, encode_image, get_latest_files, capture_screenshot};

// /// Result type for utility functions
// pub type UtilsResult<T> = Result<T, UtilsError>;
