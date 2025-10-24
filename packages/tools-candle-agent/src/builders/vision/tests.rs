#[cfg(test)]
mod tests {
    use super::super::*;
    use tokio_stream::StreamExt;
    
    #[tokio::test]
    async fn test_vision_builder_construction() {
        // Should not panic - model should be in registry
        let _builder = VisionBuilderImpl::new();
        // If we get here, construction succeeded
    }
    
    #[tokio::test]
    async fn test_describe_image_returns_stream() {
        let builder = VisionBuilderImpl::new();
        
        // Note: This test requires a valid image file
        // Skip if test image not available
        let test_image_path = "tests/fixtures/test_image.png";
        if !std::path::Path::new(test_image_path).exists() {
            println!("Skipping test - test image not found at {}", test_image_path);
            return;
        }
        
        let mut stream = builder.describe_image(test_image_path, "What do you see?");
        
        let mut got_chunk = false;
        while let Some(chunk) = stream.next().await {
            got_chunk = true;
            match chunk {
                CandleStringChunk::Text(_) => {},
                CandleStringChunk::Complete { .. } => break,
                CandleStringChunk::Error(e) => panic!("Vision error: {}", e),
            }
        }
        
        assert!(got_chunk, "Should receive at least one chunk");
    }
}
