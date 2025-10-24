//! Unit tests for query function
//!
//! Tests the simple one-shot query functionality

use futures::StreamExt;
use kodegen_tools_claude_agent::query;

#[tokio::test]
async fn test_simple_query() {
    let _ = env_logger::builder().is_test(true).try_init();

    let stream = query("What is 2+2?", None).await.unwrap();
    let mut stream = Box::pin(stream);

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => log::info!("Message: {msg:?}"),
            Err(e) => log::error!("Error: {e}"),
        }
    }
}
