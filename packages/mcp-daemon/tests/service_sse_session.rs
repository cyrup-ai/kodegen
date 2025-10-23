use kodegen_daemon::service::sse::session::{SseSession, SessionManager, ClientInfo};
use std::time::{Duration, Instant};
use tokio::time::sleep;

fn create_test_client_info() -> ClientInfo {
    ClientInfo {
        remote_addr: "127.0.0.1:12345".to_string(),
        user_agent: Some("test-client".to_string()),
        connection_id: None,
    }
}

#[test]
fn test_session_creation() {
    let client_info = create_test_client_info();
    let session = SseSession::new(client_info.clone());

    assert!(!session.id.is_empty());
    assert_eq!(session.client_info.remote_addr, "127.0.0.1:12345");
    assert!(!session.is_expired(Duration::from_secs(1)));
}

#[test]
fn test_session_expiry() {
    let client_info = create_test_client_info();
    let mut session = SseSession::new(client_info);

    // Manually set old timestamp
    session.last_activity = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();

    assert!(session.is_expired(Duration::from_secs(5)));
    assert!(!session.is_expired(Duration::from_secs(15)));
}

#[test]
fn test_session_touch() {
    let client_info = create_test_client_info();
    let mut session = SseSession::new(client_info);

    let initial_activity = session.last_activity;

    // Small delay to ensure timestamp difference
    std::thread::sleep(Duration::from_millis(1));

    session.touch();
    assert!(session.last_activity > initial_activity);
}

#[tokio::test]
async fn test_session_manager_creation() {
    let manager = SessionManager::new(10, Duration::from_secs(60));
    let client_info = create_test_client_info();

    let session = manager.create_session(client_info).await;
    assert!(session.is_some());

    let session = session.unwrap();
    assert_eq!(manager.session_count().await, 1);

    // Test retrieval
    let retrieved = manager.get_session(&session.id).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, session.id);
}

#[tokio::test]
async fn test_session_limit() {
    let manager = SessionManager::new(2, Duration::from_secs(60));
    let client_info = create_test_client_info();

    // Create maximum sessions
    let session1 = manager.create_session(client_info.clone()).await;
    let session2 = manager.create_session(client_info.clone()).await;
    assert!(session1.is_some());
    assert!(session2.is_some());
    assert_eq!(manager.session_count().await, 2);

    // Should reject additional session
    let session3 = manager.create_session(client_info).await;
    assert!(session3.is_none());
    assert_eq!(manager.session_count().await, 2);
}

#[tokio::test]
async fn test_session_removal() {
    let manager = SessionManager::new(10, Duration::from_secs(60));
    let client_info = create_test_client_info();

    let session = manager.create_session(client_info).await.unwrap();
    assert_eq!(manager.session_count().await, 1);

    let removed = manager.remove_session(&session.id).await;
    assert!(removed);
    assert_eq!(manager.session_count().await, 0);

    // Removing again should return false
    let removed_again = manager.remove_session(&session.id).await;
    assert!(!removed_again);
}

#[tokio::test]
async fn test_session_manager_touch() {
    let manager = SessionManager::new(10, Duration::from_secs(60));
    let client_info = create_test_client_info();

    let session = manager.create_session(client_info).await.unwrap();

    let touched = manager.touch_session(&session.id).await;
    assert!(touched);

    let not_touched = manager.touch_session("nonexistent").await;
    assert!(!not_touched);
}

#[tokio::test]
async fn test_session_cleanup() {
    let manager = SessionManager::new(10, Duration::from_millis(100));
    let client_info = create_test_client_info();

    // Create a session
    let _session = manager.create_session(client_info).await.unwrap();
    assert_eq!(manager.session_count().await, 1);

    // Wait for expiry
    sleep(Duration::from_millis(150)).await;

    // Clean up expired sessions
    let cleaned = manager.cleanup_expired().await;
    assert_eq!(cleaned, 1);
    assert_eq!(manager.session_count().await, 0);
}
