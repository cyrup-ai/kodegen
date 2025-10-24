//! Core provider trait definitions for AI services

use crate::ZeroOneOrMany;
use super::traits::CandleModel;

/// Trait for AI service providers that can enumerate their available models
pub trait Provider {
    /// The model type this provider supports
    type Model: CandleModel;

    /// Get the provider's name
    fn name(&self) -> &'static str;

    /// Get all models available from this provider
    fn models(&self) -> ZeroOneOrMany<Self::Model>;
}
