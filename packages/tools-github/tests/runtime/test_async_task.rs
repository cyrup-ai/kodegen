//! Tests for async task runtime primitives.

use futures::StreamExt;
use kodegen_tools_github::runtime::{AsyncStream, AsyncTask};

#[tokio::test]
async fn test_async_task_spawn() {
    let task = AsyncTask::spawn(|| 42);
    let result = task.await.unwrap();
    assert_eq!(result, 42);
}

#[tokio::test]
async fn test_async_task_spawn_async() {
    let task = AsyncTask::spawn_async(async { 42 });
    let result = task.await.unwrap();
    assert_eq!(result, 42);
}

#[tokio::test]
async fn test_async_stream_from_vec() {
    let mut stream = AsyncStream::from_vec(vec![1, 2, 3]);

    assert_eq!(stream.next().await, Some(1));
    assert_eq!(stream.next().await, Some(2));
    assert_eq!(stream.next().await, Some(3));
    assert_eq!(stream.next().await, None);
}
