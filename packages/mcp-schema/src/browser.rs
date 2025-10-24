//! Browser tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// WAIT FOR
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WaitCondition {
    /// Element exists in DOM (uses page.find_element)
    Present,

    /// Element is visible (not display:none, visibility:hidden, etc)
    Visible,

    /// Element is visible AND not disabled (ready for interaction)
    Clickable,

    /// Element text contains specific string
    TextContains,

    /// Element attribute has specific value
    AttributeIs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserWaitForArgs {
    /// CSS selector for element to wait for
    pub selector: String,

    /// Condition to wait for
    pub condition: WaitCondition,

    /// Optional: timeout in milliseconds (default: 10000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Optional: expected text (required for TextContains condition)
    #[serde(default)]
    pub text: Option<String>,

    /// Optional: attribute name (required for AttributeIs condition)
    #[serde(default)]
    pub attribute_name: Option<String>,

    /// Optional: attribute value (required for AttributeIs condition)
    #[serde(default)]
    pub attribute_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserWaitForPromptArgs {}

// ============================================================================
// SCREENSHOT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScreenshotArgs {
    /// Optional: CSS selector to screenshot specific element (default: full page)
    #[serde(default)]
    pub selector: Option<String>,

    /// Optional: format (png or jpeg, default: png)
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScreenshotPromptArgs {}

// ============================================================================
// BROWSER RESEARCH
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserResearchArgs {
    /// Research query or topic to investigate
    pub query: String,

    /// Maximum number of pages to visit (default: 5)
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,

    /// Maximum link-following depth (default: 2)
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Search engine to use: "google", "bing", "duckduckgo" (default: "google")
    #[serde(default = "default_search_engine")]
    pub search_engine: String,

    /// Include hyperlinks in content extraction (default: true)
    #[serde(default = "default_true")]
    pub include_links: bool,

    /// Extract and parse HTML tables (default: true)
    #[serde(default = "default_true")]
    pub extract_tables: bool,

    /// Extract image URLs and alt text (default: false)
    #[serde(default = "default_false")]
    pub extract_images: bool,

    /// Timeout per page navigation in seconds (default: 60)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// LLM temperature for summarization (0.0=deterministic, 2.0=creative, default: 0.5)
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// Maximum tokens for LLM summary generation (default: 2048)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u64,
}

fn default_max_pages() -> usize {
    5
}
fn default_max_depth() -> usize {
    2
}
fn default_search_engine() -> String {
    "google".into()
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_timeout() -> u64 {
    60
}
fn default_temperature() -> f64 {
    0.5
}
fn default_max_tokens() -> u64 {
    2048
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserResearchPromptArgs {}

// ============================================================================
// EXTRACT TEXT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserExtractTextArgs {
    /// Optional: CSS selector for specific element (default: entire page)
    #[serde(default)]
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserExtractTextPromptArgs {}

// ============================================================================
// WAIT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserWaitArgs {
    /// Duration to wait in milliseconds
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserWaitPromptArgs {}

// ============================================================================
// NAVIGATE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserNavigateArgs {
    /// URL to navigate to (must start with http:// or https://)
    pub url: String,

    /// Optional: wait for specific CSS selector before returning
    #[serde(default)]
    pub wait_for_selector: Option<String>,

    /// Optional: timeout in milliseconds (default: 30000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserNavigatePromptArgs {}

// ============================================================================
// CLICK
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserClickArgs {
    /// CSS selector for element to click
    pub selector: String,

    /// Optional: timeout in milliseconds (default: 5000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserClickPromptArgs {}

// ============================================================================
// SCROLL
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScrollArgs {
    /// Optional: CSS selector to scroll to element (takes priority over x/y)
    #[serde(default)]
    pub selector: Option<String>,

    /// Optional: horizontal scroll amount in pixels (default: 0)
    #[serde(default)]
    pub x: Option<i32>,

    /// Optional: vertical scroll amount in pixels (default: 0)
    #[serde(default)]
    pub y: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScrollPromptArgs {}

// ============================================================================
// TYPE TEXT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserTypeTextArgs {
    /// CSS selector for input element
    pub selector: String,

    /// Text to type into the element
    pub text: String,

    /// Optional: clear existing text first (default: true)
    #[serde(default = "default_clear")]
    pub clear: bool,

    /// Optional: timeout in milliseconds (default: 5000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

fn default_clear() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserTypeTextPromptArgs {}

// ============================================================================
// BROWSER AGENT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserAgentArgs {
    /// Task description for the agent to accomplish
    pub task: String,

    /// Optional additional context or hints
    #[serde(default)]
    pub additional_info: Option<String>,

    /// Optional initial URL to navigate to before starting
    #[serde(default)]
    pub start_url: Option<String>,

    /// Maximum steps agent can take (default: 10)
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,

    /// Maximum actions per step (default: 3)
    #[serde(default = "default_max_actions")]
    pub max_actions_per_step: u32,

    /// LLM temperature for action generation (default: 0.7)
    #[serde(default = "default_agent_temperature")]
    pub temperature: f64,

    /// Max tokens per LLM call (default: 2048)
    #[serde(default = "default_agent_max_tokens")]
    pub max_tokens: u64,

    /// Vision model timeout in seconds (default: 60s)
    /// Vision analysis is typically fast, but allow time for model loading
    #[serde(default = "default_vision_timeout_secs")]
    pub vision_timeout_secs: u64,

    /// LLM generation timeout in seconds (default: 120s)
    /// Allow time for complex reasoning and high token generation
    #[serde(default = "default_llm_timeout_secs")]
    pub llm_timeout_secs: u64,
}

fn default_max_steps() -> u32 {
    10
}
fn default_max_actions() -> u32 {
    3
}
fn default_agent_temperature() -> f64 {
    0.7
}
fn default_agent_max_tokens() -> u64 {
    2048
}
fn default_vision_timeout_secs() -> u64 {
    60
}
fn default_llm_timeout_secs() -> u64 {
    120
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserAgentPromptArgs {}
