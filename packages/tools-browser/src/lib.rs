//! Browser automation tools for AI agents
//!
//! Provides browser control, page navigation, and content extraction via chromiumoxide.

mod browser;
mod manager;
mod tools;

pub use browser::{
    BrowserWrapper, 
    launch_browser, 
    BrowserError, 
    BrowserResult,
    find_browser_executable,
    download_managed_browser,
};
pub use manager::BrowserManager;
pub use tools::{
    BrowserNavigateTool, 
    BrowserScreenshotTool,
    BrowserClickTool,
    BrowserTypeTextTool,
    BrowserExtractTextTool,
    BrowserScrollTool,
    BrowserWaitTool,
};
