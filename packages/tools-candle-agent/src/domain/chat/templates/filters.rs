//! Template filters for data transformation
//!
//! Provides built-in filters for template processing.

use std::sync::Arc;

use crate::domain::chat::templates::core::{
    TemplateError as CandleTemplateError, TemplateResult as CandleTemplateResult,
    TemplateValue as CandleTemplateValue,
};

/// Template filter function type
pub type FilterFunction = Arc<
    dyn Fn(
            &CandleTemplateValue,
            &[CandleTemplateValue],
        ) -> CandleTemplateResult<CandleTemplateValue>
        + Send
        + Sync,
>;

/// Filter registry for managing template filters
pub struct FilterRegistry {
    filters: std::collections::HashMap<String, FilterFunction>,
}

impl FilterRegistry {
    /// Create a new filter registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            filters: std::collections::HashMap::new(),
        }
    }

    /// Create a registry with default filters
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_default_filters();
        registry
    }

    /// Register a filter
    pub fn register(&mut self, name: impl Into<String>, filter: FilterFunction) {
        self.filters.insert(name.into(), filter);
    }

    /// Get a filter by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&FilterFunction> {
        self.filters.get(name)
    }

    /// Apply a filter to a value
    ///
    /// # Errors
    ///
    /// Returns `CandleTemplateError` if:
    /// - Filter with the given name does not exist
    /// - Filter execution fails
    pub fn apply(
        &self,
        name: &str,
        value: &CandleTemplateValue,
        args: &[CandleTemplateValue],
    ) -> CandleTemplateResult<CandleTemplateValue> {
        match self.get(name) {
            Some(filter) => filter(value, args),
            None => Err(CandleTemplateError::RenderError {
                message: format!("Unknown filter: {name}"),
            }),
        }
    }

    /// Register default filters
    fn register_default_filters(&mut self) {
        // uppercase filter
        self.register(
            "uppercase",
            Arc::new(|value, _args| match value {
                CandleTemplateValue::String(s) => Ok(CandleTemplateValue::String(s.to_uppercase())),
                _ => Err(CandleTemplateError::RenderError {
                    message: "uppercase filter can only be applied to strings".to_string(),
                }),
            }),
        );

        // lowercase filter
        self.register(
            "lowercase",
            Arc::new(|value, _args| match value {
                CandleTemplateValue::String(s) => Ok(CandleTemplateValue::String(s.to_lowercase())),
                _ => Err(CandleTemplateError::RenderError {
                    message: "lowercase filter can only be applied to strings".to_string(),
                }),
            }),
        );

        // trim filter
        self.register(
            "trim",
            Arc::new(|value, _args| match value {
                CandleTemplateValue::String(s) => {
                    Ok(CandleTemplateValue::String(s.trim().to_string()))
                }
                _ => Err(CandleTemplateError::RenderError {
                    message: "trim filter can only be applied to strings".to_string(),
                }),
            }),
        );

        // length filter
        self.register(
            "length",
            Arc::new(|value, _args| match value {
                CandleTemplateValue::String(s) =>
                {
                    #[allow(clippy::cast_precision_loss)]
                    Ok(CandleTemplateValue::Number(s.len() as f64))
                }
                CandleTemplateValue::Array(arr) =>
                {
                    #[allow(clippy::cast_precision_loss)]
                    Ok(CandleTemplateValue::Number(arr.len() as f64))
                }
                _ => Err(CandleTemplateError::RenderError {
                    message: "length filter can only be applied to strings or arrays".to_string(),
                }),
            }),
        );

        // default filter
        self.register(
            "default",
            Arc::new(|value, args| {
                let is_empty = match value {
                    CandleTemplateValue::String(s) => s.is_empty(),
                    CandleTemplateValue::Null => true,
                    _ => false,
                };

                if is_empty && !args.is_empty() {
                    Ok(args[0].clone())
                } else {
                    Ok(value.clone())
                }
            }),
        );
    }
}

impl Default for FilterRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
