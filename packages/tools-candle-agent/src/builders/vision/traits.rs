use super::*;

/// Fluent builder trait for vision operations
pub trait CandleVisionBuilder: Send + Sync {
    /// Describe a local image file
    ///
    /// # Arguments
    /// * `image_path` - Path to the image file (PNG, JPEG, etc.)
    /// * `query` - Question or instruction about the image
    ///
    /// # Returns
    /// Stream of text chunks describing the image
    fn describe_image(
        &self,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>>;

    /// Describe an image from URL
    ///
    /// # Arguments
    /// * `url` - URL of the image
    /// * `query` - Question or instruction about the image
    ///
    /// # Returns
    /// Stream of text chunks describing the image
    fn describe_url(
        &self,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>>;
}
