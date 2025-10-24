//! Browser automation tool implementations

mod navigate;
mod screenshot;
mod click;
mod type_text;
mod extract_text;
mod scroll;
mod wait;

pub use navigate::BrowserNavigateTool;
pub use screenshot::BrowserScreenshotTool;
pub use click::BrowserClickTool;
pub use type_text::BrowserTypeTextTool;
pub use extract_text::BrowserExtractTextTool;
pub use scroll::BrowserScrollTool;
pub use wait::BrowserWaitTool;
