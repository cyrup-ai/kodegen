//! Static metadata for all 109 tools across 14 categories.
//!
//! This module aggregates tool metadata from organized category submodules
//! to avoid instantiating tool objects at runtime.

use super::types::ToolMetadata;

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

/// All 109 tools with static metadata.
pub fn all_tool_metadata() -> Vec<ToolMetadata> {
    let mut tools = Vec::with_capacity(109);
    
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
    
    tools
}
