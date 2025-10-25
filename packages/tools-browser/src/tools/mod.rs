//! Browser automation tool implementations

mod browser_agent;
mod browser_research;
mod click;
mod extract_text;
mod navigate;
mod screenshot;
mod scroll;
mod type_text;
mod wait;
mod wait_for;
mod web_search;

pub use browser_agent::BrowserAgentTool;
pub use browser_research::BrowserResearchTool;
pub use click::BrowserClickTool;
pub use extract_text::BrowserExtractTextTool;
pub use navigate::BrowserNavigateTool;
pub use screenshot::BrowserScreenshotTool;
pub use scroll::BrowserScrollTool;
pub use type_text::BrowserTypeTextTool;
pub use wait::BrowserWaitTool;
pub use wait_for::BrowserWaitForTool;
pub use web_search::WebSearchTool;
