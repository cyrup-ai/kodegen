//! Static metadata for all tools across 14 categories.
//!
//! This module aggregates tool metadata from organized category submodules
//! to avoid instantiating tool objects at runtime.

use super::types::ToolMetadata;
use once_cell::sync::Lazy;

mod ai_reasoning;
mod data_persistence;
mod execution;
mod infrastructure;
mod version_control;
mod web_external;

use ai_reasoning::ai_reasoning_tools;
use data_persistence::data_persistence_tools;
use execution::execution_tools;
use infrastructure::infrastructure_tools;
use version_control::version_control_tools;
use web_external::web_external_tools;

/// All tools with static metadata, cached and sorted alphabetically.
static CACHED_TOOL_METADATA: Lazy<Vec<ToolMetadata>> = Lazy::new(|| {
    let mut tools = Vec::new();
    
    // Add infrastructure tools (config, introspection, process)
    tools.extend(infrastructure_tools());
    
    // Add version control tools (git, github)
    tools.extend(version_control_tools());
    
    // Add data and persistence tools (database, filesystem)
    tools.extend(data_persistence_tools());
    
    // Add execution tools (terminal)
    tools.extend(execution_tools());
    
    // Add web and external tools (browser, citescrape)
    tools.extend(web_external_tools());
    
    // Add AI and reasoning tools (claude_agent, candle_agent, reasoner, sequential_thinking, prompt)
    tools.extend(ai_reasoning_tools());
    
    // Sort alphabetically by tool name for consistent ordering
    tools.sort_by(|a, b| a.name.cmp(b.name));
    
    tools
});

/// Returns a static reference to all tool metadata (cached, sorted).
pub fn all_tool_metadata() -> &'static [ToolMetadata] {
    &CACHED_TOOL_METADATA
}
