//! Static metadata for all tools across 15 categories.
//!
//! This module aggregates tool metadata from organized category submodules
//! to avoid instantiating tool objects at runtime.

use super::types::ToolMetadata;
use once_cell::sync::Lazy;

// Agent packages
mod claude_agent;
mod candle_agent;

// Tool packages
mod browser;
mod citescrape;
mod config;
mod database;
mod filesystem;
mod git;
mod github;
mod introspection;
mod process;
mod prompt;
mod reasoner;
mod sequential_thinking;
mod terminal;

use claude_agent::claude_agent_tools;
use candle_agent::candle_agent_tools;
use browser::browser_tools;
use citescrape::citescrape_tools;
use config::config_tools;
use database::database_tools;
use filesystem::filesystem_tools;
use git::git_tools;
use github::github_tools;
use introspection::introspection_tools;
use process::process_tools;
use prompt::prompt_tools;
use reasoner::reasoner_tools;
use sequential_thinking::sequential_thinking_tools;
use terminal::terminal_tools;

/// All tools with static metadata, cached and sorted alphabetically.
static CACHED_TOOL_METADATA: Lazy<Vec<ToolMetadata>> = Lazy::new(|| {
    let mut tools = Vec::new();
    
    // Agent packages
    tools.extend(claude_agent_tools());
    tools.extend(candle_agent_tools());
    
    // Tool packages (alphabetical)
    tools.extend(browser_tools());
    tools.extend(citescrape_tools());
    tools.extend(config_tools());
    tools.extend(database_tools());
    tools.extend(filesystem_tools());
    tools.extend(git_tools());
    tools.extend(github_tools());
    tools.extend(introspection_tools());
    tools.extend(process_tools());
    tools.extend(prompt_tools());
    tools.extend(reasoner_tools());
    tools.extend(sequential_thinking_tools());
    tools.extend(terminal_tools());
    
    // Sort alphabetically by tool name for consistent ordering
    tools.sort_by(|a, b| a.name.cmp(b.name));
    
    tools
});

/// Returns a static reference to all tool metadata (cached, sorted).
pub fn all_tool_metadata() -> &'static [ToolMetadata] {
    &CACHED_TOOL_METADATA
}
