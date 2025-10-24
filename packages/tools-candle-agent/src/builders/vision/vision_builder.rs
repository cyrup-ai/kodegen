use super::*;

/// Vision builder implementation
pub struct VisionBuilderImpl {
    vision_model: VisionModel,
}

impl VisionBuilderImpl {
    /// Create a new vision builder with default LLaVA model
    pub(crate) fn new() -> Self {
        // Get LLaVA model from registry
        // Access the registry directly to get the concrete VisionModel type
        use crate::capability::registry::storage::VISION_UNIFIED;
        
        let vision_model = if let Some(model) = VISION_UNIFIED.read().get("llava-hf/llava-1.5-7b-hf").cloned() {
            model
        } else {
            // This should never happen as LLaVA is registered at startup
            log::error!("LLaVA vision model not registered - this indicates a critical initialization error");
            panic!("LLaVA vision model should be registered at startup");
        };
        
        Self { vision_model }
    }
}

impl CandleVisionBuilder for VisionBuilderImpl {
    fn describe_image(
        &self,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        // Delegate directly to VisionCapable trait
        // Pool routing happens automatically in VisionModel implementation
        self.vision_model.describe_image(image_path, query)
    }

    fn describe_url(
        &self,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        // Delegate directly to VisionCapable trait
        self.vision_model.describe_url(url, query)
    }
}
