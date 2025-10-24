use kodegen_candle_agent::prelude::*;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== CandleFluentAi Vision API Test ===\n");
    
    // Test with local image
    println!("Describing local image...");
    let test_image_path = "tests/fixtures/browser_screenshot.png";
    
    // Check if test image exists
    if !std::path::Path::new(test_image_path).exists() {
        println!("Test image not found at: {}", test_image_path);
        println!("Skipping vision test. Please provide a test image to run this example.");
        return Ok(());
    }
    
    let mut stream = CandleFluentAi::vision()
        .describe_image(test_image_path, "What UI elements are visible?");
    
    print!("Response: ");
    while let Some(chunk) = stream.next().await {
        match chunk {
            CandleStringChunk::Text(text) => print!("{}", text),
            CandleStringChunk::Complete { elapsed_secs, .. } => {
                println!("\n\nCompleted in {:.2}s", elapsed_secs.unwrap_or(0.0));
                break;
            }
            CandleStringChunk::Error(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}
