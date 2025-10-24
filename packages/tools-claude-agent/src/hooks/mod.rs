//! Hook system for intercepting agent events
//!
//! This module provides the hook system that allows users to intercept
//! and respond to various events in the agent lifecycle.

use std::sync::Arc;

use crate::error::Result;
use crate::types::hooks::{HookCallback, HookContext, HookDecision, HookMatcher, HookOutput};

/// Hook manager for registering and invoking hooks
pub struct HookManager {
    /// Registered hook matchers
    matchers: Vec<HookMatcher>,
}

impl HookManager {
    /// Create a new hook manager
    #[must_use]
    pub const fn new() -> Self {
        Self {
            matchers: Vec::new(),
        }
    }

    /// Register a hook with a matcher
    ///
    /// # Arguments
    /// * `matcher` - Hook matcher configuration
    pub fn register(&mut self, matcher: HookMatcher) {
        self.matchers.push(matcher);
    }

    /// Invoke hooks for a given event
    ///
    /// # Arguments
    /// * `event_data` - Event data (JSON value)
    /// * `tool_name` - Optional tool name
    /// * `context` - Hook context
    ///
    /// # Returns
    /// Hook output with optional decision and modifications
    ///
    /// # Errors
    /// Returns error if hook callback execution fails
    pub async fn invoke(
        &self,
        event_data: serde_json::Value,
        tool_name: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput> {
        let mut output = HookOutput::default();

        // Find matching hooks
        for matcher in &self.matchers {
            if Self::matches(matcher.matcher.as_ref(), tool_name.as_ref()) {
                // Invoke each hook callback
                for hook in &matcher.hooks {
                    let result =
                        hook(event_data.clone(), tool_name.clone(), context.clone()).await?;

                    // Merge hook results
                    if result.decision.is_some() {
                        output.decision = result.decision;
                    }
                    if result.system_message.is_some() {
                        output.system_message = result.system_message;
                    }
                    if result.hook_specific_output.is_some() {
                        output.hook_specific_output = result.hook_specific_output;
                    }

                    // If decision is Block, stop processing
                    if matches!(output.decision, Some(HookDecision::Block)) {
                        return Ok(output);
                    }
                }
            }
        }

        Ok(output)
    }

    /// Check if a matcher matches a tool name
    ///
    /// # Security Note
    /// This uses simple pattern matching with pipe-separated alternatives.
    /// For production use with untrusted patterns, consider using a proper
    /// glob or regex library with safety guarantees (e.g., `globset` crate).
    ///
    /// # Examples
    /// ```
    /// use kodegen_claude_agent::hooks::HookManager;
    ///
    /// // Wildcard matches all
    /// assert!(HookManager::matches(Some(&"*".to_string()), Some(&"tool".to_string())));
    ///
    /// // Exact match
    /// assert!(HookManager::matches(Some(&"Bash".to_string()), Some(&"Bash".to_string())));
    ///
    /// // Pattern with alternatives
    /// assert!(HookManager::matches(Some(&"Read|Write".to_string()), Some(&"Read".to_string())));
    /// ```
    #[must_use]
    pub fn matches(matcher: Option<&String>, tool_name: Option<&String>) -> bool {
        match (matcher, tool_name) {
            (None, _) => true, // No matcher = match all
            (Some(pattern), Some(name)) => {
                // Simple wildcard matching
                if pattern == "*" {
                    return true;
                }
                // Exact match or simple pipe-separated pattern
                // Note: This doesn't handle edge cases like pipe characters in tool names
                pattern == name || pattern.split('|').any(|p| p == name)
            }
            (Some(_), None) => false,
        }
    }

    /// Create a hook callback from a closure
    pub fn callback<F, Fut>(f: F) -> HookCallback
    where
        F: Fn(serde_json::Value, Option<String>, HookContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<HookOutput>> + Send + 'static,
    {
        Arc::new(move |event_data, tool_name, context| Box::pin(f(event_data, tool_name, context)))
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating hook matchers
pub struct HookMatcherBuilder {
    matcher: Option<String>,
    hooks: Vec<HookCallback>,
}

impl HookMatcherBuilder {
    /// Create a new hook matcher builder
    ///
    /// # Arguments
    /// * `pattern` - Matcher pattern (None for all, or specific tool name/pattern)
    pub fn new(pattern: Option<impl Into<String>>) -> Self {
        Self {
            matcher: pattern.map(std::convert::Into::into),
            hooks: Vec::new(),
        }
    }

    /// Add a hook callback
    #[must_use]
    pub fn add_hook(mut self, hook: HookCallback) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Build the hook matcher
    #[must_use]
    pub fn build(self) -> HookMatcher {
        HookMatcher {
            matcher: self.matcher,
            hooks: self.hooks,
        }
    }
}
