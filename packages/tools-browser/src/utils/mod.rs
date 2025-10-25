// All browser utility modules - no feature gating
mod agent_state;
pub mod constants;
mod deep_research;
mod errors;
mod timeout;
// mod utils;

pub use agent_state::AgentState;
pub use deep_research::{DeepResearch, ResearchOptions};
pub use timeout::{
    validate_interaction_timeout, validate_navigation_timeout, validate_wait_timeout,
};
// pub use utils::{llama, encode_image, get_latest_files, capture_screenshot};

// /// Result type for utility functions
// pub type UtilsResult<T> = Result<T, UtilsError>;
