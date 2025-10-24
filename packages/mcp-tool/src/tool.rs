use rmcp::handler::server::tool::schema_for_type;
use rmcp::model::{PromptArgument, PromptMessage};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::McpError;

// ============================================================================
// PERFORMANCE OPTIMIZATIONS
// ============================================================================

/// Type alias for the schema cache to reduce complexity
type SchemaCache =
    parking_lot::RwLock<HashMap<&'static str, std::sync::Arc<serde_json::Map<String, Value>>>>;

/// Schema cache to avoid repeated serialization
static SCHEMA_CACHE: std::sync::LazyLock<SchemaCache> =
    std::sync::LazyLock::new(|| parking_lot::RwLock::new(HashMap::new()));

// ============================================================================
// CORE TRAIT
// ============================================================================

/// Core trait that all tools must implement
///
/// Tools are STRUCTS that hold their own dependencies (`GitClient`, `GitHubClient`, etc.)
/// The trait is generic and knows nothing about specific services.
/// Every method (except execute/prompt) has a sensible default.
pub trait Tool: Send + Sync + Sized + 'static {
    /// Tool execution arguments (auto-generates schema via `JsonSchema`)
    type Args: DeserializeOwned + Serialize + JsonSchema + Send + 'static;

    /// Prompt arguments (what context does teaching need?)
    type PromptArgs: DeserializeOwned + JsonSchema + Send + 'static;

    // ========================================================================
    // IDENTITY (Required)
    // ========================================================================

    /// Unique tool name (e.g., "`git_clone`")
    fn name() -> &'static str;

    /// Human-readable description of what this tool does
    fn description() -> &'static str;

    // ========================================================================
    // SCHEMA (Auto-generated with caching)
    // ========================================================================

    /// Input schema - AUTO-GENERATED from Args type via `JsonSchema` derive
    /// Cached for performance - schema is computed once and reused
    #[inline]
    fn input_schema() -> std::sync::Arc<serde_json::Map<String, Value>> {
        let name = Self::name();

        // Fast path: read from cache
        if let Some(schema) = SCHEMA_CACHE.read().get(name) {
            return schema.clone();
        }

        // Slow path: generate and cache
        let schema = std::sync::Arc::new(schema_for_type::<Self::Args>());
        SCHEMA_CACHE.write().insert(name, schema.clone());
        schema
    }

    /// Output schema (optional, rarely needed)
    #[must_use]
    #[inline]
    fn output_schema() -> Option<std::sync::Arc<serde_json::Map<String, Value>>> {
        None
    }

    // ========================================================================
    // BEHAVIOR ANNOTATIONS (Tool IS its behavior)
    // ========================================================================

    /// Does this tool only read (never modify) state?
    ///
    /// true = read-only (safe, can't break things)
    /// false = writes/modifies state (requires caution)
    ///
    /// Default: true (assumes read-only by default - safe default)
    #[must_use]
    #[inline]
    fn read_only() -> bool {
        true
    }

    /// Can this tool delete or overwrite existing data?
    ///
    /// Only meaningful when `read_only` = false.
    /// true = can delete/overwrite (dangerous)
    /// false = only adds/creates (safer)
    ///
    /// Default: false (assumes non-destructive by default - safe default)
    #[must_use]
    #[inline]
    fn destructive() -> bool {
        false
    }

    /// Is calling this tool repeatedly with same args safe/idempotent?
    ///
    /// Only meaningful when `read_only` = false.
    /// true = safe to retry (same result every time)
    /// false = each call has different effect
    ///
    /// Default: true (assumes idempotent by default - safe default)
    #[must_use]
    #[inline]
    fn idempotent() -> bool {
        true
    }

    /// Does this tool interact with external systems (network, filesystem outside repo)?
    ///
    /// true = open world (network calls, external APIs, can fail due to external factors)
    /// false = closed world (only local operations, deterministic)
    ///
    /// Default: false (assumes local operations by default - safe default)
    #[must_use]
    #[inline]
    fn open_world() -> bool {
        false
    }

    // ========================================================================
    // EXECUTION (Required)
    // ========================================================================

    /// Execute the tool with given arguments
    ///
    /// This is where the actual work happens.
    /// Since tools are structs holding their dependencies, we use &self.
    fn execute(
        &self,
        args: Self::Args,
    ) -> impl std::future::Future<Output = Result<Value, McpError>> + Send;

    // ========================================================================
    // PROMPTING (Required - agents need this!)
    // ========================================================================

    /// Prompt name (defaults to "{`tool_name`}_help")
    #[must_use]
    #[inline]
    fn prompt_name() -> Cow<'static, str> {
        Cow::Owned(format!("{}_help", Self::name()))
    }

    /// Prompt description (defaults to tool description)
    #[must_use]
    #[inline]
    fn prompt_description() -> &'static str {
        Self::description()
    }

    /// What arguments does the teaching prompt accept?
    ///
    /// These let agents customize what they want to learn about.
    /// Example: "repo" (which repo?), "shallow" (learn about shallow clones?)
    fn prompt_arguments() -> Vec<PromptArgument>;

    /// Generate teaching conversation for this tool
    ///
    /// Returns a conversation showing agent how/when to use the tool.
    /// Should include examples, common patterns, gotchas, requirements.
    /// Since tools are structs, we use &self here too.
    fn prompt(
        &self,
        args: Self::PromptArgs,
    ) -> impl std::future::Future<Output = Result<Vec<PromptMessage>, McpError>> + Send;

    // ========================================================================
    // RMCP INTEGRATION (Default implementations)
    // ========================================================================

    /// Convert this tool into an RMCP `ToolRoute`
    ///
    /// This default implementation builds the route from trait methods.
    /// Tools get this for free - no need to implement `IntoToolRoute` manually.
    fn into_tool_route<S>(self) -> rmcp::handler::server::router::tool::ToolRoute<S>
    where
        S: Send + Sync + 'static,
    {
        use rmcp::handler::server::router::tool::ToolRoute;
        use rmcp::handler::server::wrapper::Parameters;
        use rmcp::model::{CallToolResult, Content, Tool as RmcpTool, ToolAnnotations};
        use std::sync::Arc;

        // Build annotations from trait methods
        let annotations = ToolAnnotations::new()
            .read_only(Self::read_only())
            .destructive(Self::destructive())
            .idempotent(Self::idempotent())
            .open_world(Self::open_world());

        // Build RMCP Tool metadata
        let metadata = RmcpTool {
            name: Self::name().into(),
            title: None,
            description: Some(Self::description().into()),
            input_schema: Self::input_schema(),
            output_schema: Self::output_schema(),
            annotations: Some(annotations),
            icons: None,
        };

        // Wrap self in Arc for handler
        let tool = Arc::new(self);

        // Handler captures the tool instance
        let handler = move |Parameters(args): Parameters<Self::Args>| {
            let tool = tool.clone();
            async move {
                let start = std::time::Instant::now();

                // Serialize args before execute consumes them
                let args_json =
                    serde_json::to_value(&args).unwrap_or_else(|_| serde_json::json!({}));

                // Execute tool
                let result = tool.execute(args).await;
                let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

                // Convert result to JSON for history (record both success and error)
                let output_json = match &result {
                    Ok(value) => value.clone(),
                    Err(e) => serde_json::json!({
                        "error": e.to_string(),
                        "is_error": true
                    }),
                };

                // Record to history (exclude get_recent_tool_calls to prevent recursion)
                if Self::name() != "get_recent_tool_calls"
                    && let Some(history) = crate::tool_history::get_global_history()
                {
                    history.add_call(
                        Self::name().to_string(),
                        args_json,
                        output_json,
                        Some(duration_ms),
                    );
                }

                // Return result as normal
                let result =
                    result.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                Ok(CallToolResult::success(vec![Content::text(
                    result.to_string(),
                )]))
            }
        };

        ToolRoute::new(metadata, handler)
    }

    /// Convert this tool into an RMCP `PromptRoute`
    ///
    /// This default implementation builds the route from trait methods.
    /// Tools get this for free - no need to implement `IntoPromptRoute` manually.
    fn into_prompt_route<S>(self) -> rmcp::handler::server::router::prompt::PromptRoute<S>
    where
        S: Send + Sync + 'static,
    {
        use rmcp::handler::server::router::prompt::PromptRoute;
        use rmcp::handler::server::wrapper::Parameters;
        use rmcp::model::{GetPromptResult, Prompt as RmcpPrompt};
        use std::sync::Arc;

        // Build RMCP Prompt metadata
        let metadata = RmcpPrompt {
            name: Self::prompt_name().into_owned(),
            title: None,
            description: Some(Self::prompt_description().to_string()),
            arguments: Some(Self::prompt_arguments()),
            icons: None,
        };

        // Wrap self in Arc for handler
        let tool = Arc::new(self);

        // Handler captures the tool instance
        let handler = move |Parameters(args): Parameters<Self::PromptArgs>| {
            let tool = tool.clone();
            async move {
                let messages = tool
                    .prompt(args)
                    .await
                    .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                Ok(GetPromptResult {
                    description: Some(Self::prompt_description().to_string()),
                    messages,
                })
            }
        };

        PromptRoute::new(metadata, handler)
    }

    /// Convert Arc-wrapped tool into an RMCP `ToolRoute` (optimized - no extra Arc allocation)
    ///
    /// This is more efficient than `into_tool_route(self)` when the tool is already wrapped in Arc.
    /// The tool is used directly without creating an additional Arc wrapper.
    fn arc_into_tool_route<S>(self: Arc<Self>) -> rmcp::handler::server::router::tool::ToolRoute<S>
    where
        S: Send + Sync + 'static,
    {
        use rmcp::handler::server::router::tool::ToolRoute;
        use rmcp::handler::server::wrapper::Parameters;
        use rmcp::model::{CallToolResult, Content, Tool as RmcpTool, ToolAnnotations};

        // Build annotations from trait methods
        let annotations = ToolAnnotations::new()
            .read_only(Self::read_only())
            .destructive(Self::destructive())
            .idempotent(Self::idempotent())
            .open_world(Self::open_world());

        // Build RMCP Tool metadata
        let metadata = RmcpTool {
            name: Self::name().into(),
            title: None,
            description: Some(Self::description().into()),
            input_schema: Self::input_schema(),
            output_schema: Self::output_schema(),
            annotations: Some(annotations),
            icons: None,
        };

        // Use self directly (already Arc<Self>)
        let tool = self;

        // Handler captures the Arc<Tool>
        let handler = move |Parameters(args): Parameters<Self::Args>| {
            let tool = tool.clone(); // Cheap Arc clone
            async move {
                let start = std::time::Instant::now();

                // Serialize args before execute consumes them
                let args_json =
                    serde_json::to_value(&args).unwrap_or_else(|_| serde_json::json!({}));

                // Execute tool
                let result = tool.execute(args).await;
                let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

                // Convert result to JSON for history
                let output_json = match &result {
                    Ok(value) => value.clone(),
                    Err(e) => serde_json::json!({
                        "error": e.to_string(),
                        "is_error": true
                    }),
                };

                // Record to history (exclude get_recent_tool_calls to prevent recursion)
                if Self::name() != "get_recent_tool_calls"
                    && let Some(history) = crate::tool_history::get_global_history()
                {
                    history.add_call(
                        Self::name().to_string(),
                        args_json,
                        output_json,
                        Some(duration_ms),
                    );
                }

                // Return result
                let result =
                    result.map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                Ok(CallToolResult::success(vec![Content::text(
                    result.to_string(),
                )]))
            }
        };

        ToolRoute::new(metadata, handler)
    }

    /// Convert Arc-wrapped tool into an RMCP `PromptRoute` (optimized - no extra Arc allocation)
    ///
    /// This is more efficient than `into_prompt_route(self)` when the tool is already wrapped in Arc.
    /// The tool is used directly without creating an additional Arc wrapper.
    fn arc_into_prompt_route<S>(
        self: Arc<Self>,
    ) -> rmcp::handler::server::router::prompt::PromptRoute<S>
    where
        S: Send + Sync + 'static,
    {
        use rmcp::handler::server::router::prompt::PromptRoute;
        use rmcp::handler::server::wrapper::Parameters;
        use rmcp::model::{GetPromptResult, Prompt as RmcpPrompt};

        // Build RMCP Prompt metadata
        let metadata = RmcpPrompt {
            name: Self::prompt_name().into_owned(),
            title: None,
            description: Some(Self::prompt_description().to_string()),
            arguments: Some(Self::prompt_arguments()),
            icons: None,
        };

        // Use self directly (already Arc<Self>)
        let tool = self;

        // Handler captures the Arc<Tool>
        let handler = move |Parameters(args): Parameters<Self::PromptArgs>| {
            let tool = tool.clone(); // Cheap Arc clone
            async move {
                let messages = tool
                    .prompt(args)
                    .await
                    .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                Ok(GetPromptResult {
                    description: Some(Self::prompt_description().to_string()),
                    messages,
                })
            }
        };

        PromptRoute::new(metadata, handler)
    }
}
