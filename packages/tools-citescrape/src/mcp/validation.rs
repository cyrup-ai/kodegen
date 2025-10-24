//! Error context builder for structured, actionable error messages
//!
//! Provides consistent error formatting across MCP tools with:
//! - Operation that failed
//! - Details about what was checked
//! - Actionable suggestions for resolution

/// Builder for structured error messages with context and suggestions
#[derive(Debug, Clone)]
pub struct ErrorContext {
    operation: String,
    details: Vec<String>,
    suggestions: Vec<String>,
}

impl ErrorContext {
    /// Create new error context for an operation
    ///
    /// # Example
    /// ```
    /// let ctx = ErrorContext::new("Get crawl results");
    /// ```
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            details: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add detail about what was checked or why it failed
    ///
    /// # Example
    /// ```
    /// ctx.detail("crawl_id: Some(\"abc-123\")")
    ///    .detail("Session not found in active manager");
    /// ```
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.details.push(detail.into());
        self
    }

    /// Add actionable suggestion for resolution
    ///
    /// # Example
    /// ```
    /// ctx.suggest("Verify the crawl_id is correct")
    ///    .suggest("Use get_crawl_results to check status");
    /// ```
    pub fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Build formatted error message
    ///
    /// Format:
    /// ```text
    /// Operation failed: {operation}
    ///
    /// Details:
    ///   - {detail1}
    ///   - {detail2}
    ///
    /// Suggestions:
    ///   - {suggestion1}
    ///   - {suggestion2}
    /// ```
    #[must_use]
    pub fn build(self) -> String {
        let mut msg = format!("Operation failed: {}\n", self.operation);

        if !self.details.is_empty() {
            msg.push_str("\nDetails:\n");
            for detail in &self.details {
                msg.push_str(&format!("  - {detail}\n"));
            }
        }

        if !self.suggestions.is_empty() {
            msg.push_str("\nSuggestions:\n");
            for suggestion in &self.suggestions {
                msg.push_str(&format!("  - {suggestion}\n"));
            }
        }

        msg
    }
}
