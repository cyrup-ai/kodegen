//! Model pricing tier classification and analysis
//!
//! This module provides types and functionality for classifying and analyzing
//! AI model pricing tiers based on cost per token.

use serde::{Deserialize, Serialize};

/// Pricing tier classification for cost analysis
///
/// Models are classified into tiers based on their cost per 1M tokens
/// for both input and output. This helps in quickly identifying the
/// cost-effectiveness of different models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PricingTier {
    /// Ultra-low cost models (< $0.50 input, < $1.50 output per 1M tokens)
    ///
    /// These are the most cost-effective models, typically smaller models
    /// with good performance for simple tasks.
    UltraLow,

    /// Low cost models (< $1.00 input, < $3.00 output per 1M tokens)
    ///
    /// Good balance of cost and capability, suitable for most production workloads.
    Low,

    /// Medium cost models (< $5.00 input, < $15.00 output per 1M tokens)
    ///
    /// More capable models with higher quality outputs, at a moderate cost.
    Medium,

    /// High cost models (< $20.00 input, < $60.00 output per 1M tokens)
    ///
    /// High-performance models for complex tasks, with correspondingly higher costs.
    High,

    /// Premium cost models (>= $20.00 input or >= $60.00 output per 1M tokens)
    ///
    /// Top-tier models with the highest capabilities and costs.
    Premium}

impl Default for PricingTier {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for PricingTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UltraLow => write!(f, "Ultra Low"),
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Premium => write!(f, "Premium")}
    }
}

impl PricingTier {
    /// Classify model into pricing tier based on costs
    ///
    /// # Arguments
    /// * `input_cost` - Cost per 1M input tokens in USD
    /// * `output_cost` - Cost per 1M output tokens in USD
    ///
    /// # Returns
    /// * The appropriate `PricingTier` for the given costs
    pub fn classify(input_cost: f64, output_cost: f64) -> Self {
        if input_cost < 0.5 && output_cost < 1.5 {
            Self::UltraLow
        } else if input_cost < 1.0 && output_cost < 3.0 {
            Self::Low
        } else if input_cost < 5.0 && output_cost < 15.0 {
            Self::Medium
        } else if input_cost < 20.0 && output_cost < 60.0 {
            Self::High
        } else {
            Self::Premium
        }
    }

    /// Get the typical cost range for this tier
    ///
    /// Returns a tuple of (min_input_cost, max_input_cost, min_output_cost, max_output_cost)
    pub fn cost_range(&self) -> (f64, f64, f64, f64) {
        match self {
            Self::UltraLow => (0.0, 0.5, 0.0, 1.5),
            Self::Low => (0.5, 1.0, 1.5, 3.0),
            Self::Medium => (1.0, 5.0, 3.0, 15.0),
            Self::High => (5.0, 20.0, 15.0, 60.0),
            Self::Premium => (20.0, f64::MAX, 60.0, f64::MAX)}
    }

    /// Check if this tier is considered cost-effective for the given requirements
    ///
    /// # Arguments
    /// * `requires_high_quality` - Whether the task requires high-quality outputs
    /// * `budget_constrained` - Whether the usage is budget-constrained
    ///
    /// # Returns
    /// * `true` if this tier is recommended for the given requirements
    pub fn is_recommended(&self, requires_high_quality: bool, budget_constrained: bool) -> bool {
        match (requires_high_quality, budget_constrained) {
            (true, _) => matches!(self, Self::High | Self::Premium),
            (false, true) => matches!(self, Self::UltraLow | Self::Low),
            (false, false) => matches!(self, Self::Low | Self::Medium)}
    }
}
