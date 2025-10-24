//! Fluent builder pattern for LLaVA queries

use std::pin::Pin;
use tokio_stream::Stream;

use super::LLaVAModel;
use super::config::VisionConfig;
use crate::domain::context::CandleStringChunk;

/// Builder for vision model queries with configurable parameters
///
/// FOLLOWS PRODUCT-WIDE IMMUTABLE BUILDER PATTERN
/// - Methods take `mut self` (consuming)
/// - Methods return `Self` (owned)
/// - Enables fluent chaining
///
/// # Example
/// ```rust
/// model.query()
///     .temperature(0.7)
///     .max_tokens(256)
///     .describe_image("image.jpg", "what is this?")
/// ```
#[derive(Clone)]
pub struct VisionQueryBuilder {
    model: LLaVAModel,
    config: VisionConfig,
}

impl VisionQueryBuilder {
    /// Create new builder with default config
    pub(crate) fn new(model: LLaVAModel) -> Self {
        Self {
            model,
            config: VisionConfig::default(),
        }
    }

    /// Set sampling temperature (0.0 = greedy, >0.0 = sampling)
    ///
    /// IMMUTABLE BUILDER PATTERN: Consumes self, returns new self
    pub fn temperature(mut self, temp: f64) -> Self {
        self.config.temperature = temp;
        self
    }

    /// Set maximum tokens to generate
    ///
    /// IMMUTABLE BUILDER PATTERN: Consumes self, returns new self
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.config.max_tokens = Some(tokens);
        self
    }

    /// Describe image with query using configured parameters
    ///
    /// Final consumption of builder - returns Stream
    pub async fn describe_image(
        self,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        self.model
            .describe_image_internal(image_path, query, self.config)
            .await
    }

    /// Describe image from URL with query using configured parameters
    ///
    /// Final consumption of builder - returns Stream
    pub async fn describe_url(
        self,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        self.model
            .describe_url_internal(url, query, self.config)
            .await
    }
}
