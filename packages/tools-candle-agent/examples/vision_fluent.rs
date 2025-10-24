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
    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        if let Some(error) = chunk.error() {
            eprintln!("\nError: {}", error);
            break;
        }
        
        if !chunk.text.is_empty() {
            print!("{}", chunk.text);
            full_text.push_str(&chunk.text);
        }
        
        if chunk.is_final {
            if let Some(stats) = &chunk.stats {
                println!("\n\nCompleted in {:.2}s", stats.elapsed_secs);
                println!("Tokens generated: {}", stats.token_count);
                println!("Tokens/sec: {:.2}", stats.tokens_per_sec);
            } else {
                println!("\n\nCompleted");
            }
            break;
        }
    }
    
    Ok(())
}
