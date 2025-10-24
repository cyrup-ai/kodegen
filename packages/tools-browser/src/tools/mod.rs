//! Browser automation tool implementations

mod click;
mod extract_text;
mod navigate;
mod screenshot;
mod scroll;
mod type_text;
mod wait;

pub use click::BrowserClickTool;
pub use extract_text::BrowserExtractTextTool;
pub use navigate::BrowserNavigateTool;
pub use screenshot::BrowserScreenshotTool;
pub use scroll::BrowserScrollTool;
pub use type_text::BrowserTypeTextTool;
pub use wait::BrowserWaitTool;
