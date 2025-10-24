//! Browser automation tools for AI agents
//!
//! Provides browser control, page navigation, and content extraction via chromiumoxide.

mod browser;
mod manager;
mod tools;

pub use browser::{
    BrowserError, BrowserResult, BrowserWrapper, download_managed_browser, find_browser_executable,
    launch_browser,
};
pub use manager::BrowserManager;
pub use tools::{
    BrowserClickTool, BrowserExtractTextTool, BrowserNavigateTool, BrowserScreenshotTool,
    BrowserScrollTool, BrowserTypeTextTool, BrowserWaitTool,
};
