//! Browser automation tool implementations

#[cfg(feature = "agent")]
mod browser_agent;
#[cfg(feature = "research")]
mod browser_research;
mod click;
mod extract_text;
mod navigate;
mod screenshot;
mod scroll;
mod type_text;
mod wait;
mod wait_for;

#[cfg(feature = "agent")]
pub use browser_agent::{BrowserAgentArgs, BrowserAgentPromptArgs, BrowserAgentTool};
#[cfg(feature = "research")]
pub use browser_research::{BrowserResearchArgs, BrowserResearchPromptArgs, BrowserResearchTool};
pub use click::BrowserClickTool;
pub use extract_text::BrowserExtractTextTool;
pub use navigate::BrowserNavigateTool;
pub use screenshot::BrowserScreenshotTool;
pub use scroll::BrowserScrollTool;
pub use type_text::BrowserTypeTextTool;
pub use wait::BrowserWaitTool;
pub use wait_for::BrowserWaitForTool;
