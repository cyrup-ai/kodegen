// NOTE: Commented out modules with broken dependencies (pre-existing issues)
// These should be fixed separately from the timeout validation task
#[cfg(feature = "agent")]
mod agent_state;
#[cfg(feature = "agent")]
mod deep_research;
#[cfg(feature = "agent")]
mod errors;
// mod utils;

// Only active module for timeout validation
mod timeout;

#[cfg(feature = "agent")]
pub use agent_state::AgentState;
#[cfg(feature = "agent")]
pub use deep_research::{DeepResearch, ResearchOptions};
pub use timeout::{
    validate_interaction_timeout, validate_navigation_timeout, validate_wait_timeout,
};
// pub use utils::{llama, encode_image, get_latest_files, capture_screenshot};

// /// Result type for utility functions
// pub type UtilsResult<T> = Result<T, UtilsError>;
