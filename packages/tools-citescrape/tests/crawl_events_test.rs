use kodegen_tools_citescrape::crawl_events::*;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_event_bus_creation() {
    let bus = CrawlEventBus::new(100);
    assert_eq!(bus.subscriber_count(), 0);
    assert!(!bus.has_subscribers());
}

#[tokio::test]
async fn test_publish_with_no_subscribers() {
    let bus = CrawlEventBus::new(10);
    let event = CrawlEvent::crawl_started(
        "https://example.com".to_string(),
        PathBuf::from("/output"),
        2,
    );

    // Test the core functionality - publishing to empty bus should return Err(NoSubscribers)
    let result = bus.publish(event).await;
    // With the current interface, publishing to empty bus returns Err(NoSubscribers)
    assert!(
        result.is_err(),
        "Publishing to empty bus should return error"
    );
    match result {
        Err(kodegen_tools_citescrape::crawl_events::EventBusError::NoSubscribers) => {
            // Expected behavior
        }
        other => panic!("Expected EventBusError::NoSubscribers, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_subscribe_and_publish() {
    let bus = CrawlEventBus::new(10);
    let mut receiver = bus.subscribe();

    assert_eq!(bus.subscriber_count(), 1);
    assert!(bus.has_subscribers());

    let event = CrawlEvent::crawl_started(
        "https://example.com".to_string(),
        PathBuf::from("/output"),
        2,
    );

    let result = bus.publish(event.clone()).await;
    assert!(result.is_ok());
    if let Ok(count) = result {
        assert_eq!(count, 1);
    }

    // Receive the event
    let received = match timeout(Duration::from_millis(100), receiver.recv()).await {
        Ok(Ok(event)) => event,
        Ok(Err(e)) => panic!("Failed to receive event: {e}"),
        Err(_) => panic!("Timeout waiting for event"),
    };

    match (&event, &received) {
        (
            CrawlEvent::CrawlStarted {
                start_url: url1, ..
            },
            CrawlEvent::CrawlStarted {
                start_url: url2, ..
            },
        ) => {
            assert_eq!(url1, url2);
        }
        _ => panic!("Event types don't match"),
    }
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let bus = CrawlEventBus::new(10);
    let mut receiver1 = bus.subscribe();
    let mut receiver2 = bus.subscribe();

    assert_eq!(bus.subscriber_count(), 2);

    let event = CrawlEvent::page_crawled(
        "https://example.com/page".to_string(),
        PathBuf::from("/output/page.html"),
        1,
        PageCrawlMetadata {
            html_size: 1024,
            compressed_size: 512,
            links_found: 10,
            links_for_crawling: 5,
            screenshot_captured: true,
            processing_duration: Duration::from_millis(100),
        },
    );

    let result = bus.publish(event).await;
    assert!(result.is_ok());
    if let Ok(count) = result {
        assert_eq!(count, 2);
    }

    // Both receivers should get the event
    match timeout(Duration::from_millis(100), receiver1.recv()).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => panic!("Receiver 1 failed to receive event: {e}"),
        Err(_) => panic!("Receiver 1 timeout waiting for event"),
    }

    match timeout(Duration::from_millis(100), receiver2.recv()).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => panic!("Receiver 2 failed to receive event: {e}"),
        Err(_) => panic!("Receiver 2 timeout waiting for event"),
    }
}

#[tokio::test]
async fn test_async_publish() {
    let bus = CrawlEventBus::new(10);
    let event = CrawlEvent::link_rewrite_completed("https://example.com/target".to_string(), 5, 20);

    // Should not panic even with no subscribers
    let result = bus.publish(event).await;
    // Result should be Err(NoSubscribers) since we have no subscribers
    assert!(result.is_err());
}

#[test]
fn test_event_creation_helpers() {
    let start_event =
        CrawlEvent::crawl_started("https://test.com".to_string(), PathBuf::from("/test"), 3);

    match start_event {
        CrawlEvent::CrawlStarted {
            start_url,
            max_depth,
            ..
        } => {
            assert_eq!(start_url, "https://test.com");
            assert_eq!(max_depth, 3);
        }
        _ => panic!("Wrong event type"),
    }

    let metadata = PageCrawlMetadata {
        html_size: 2048,
        compressed_size: 1024,
        links_found: 15,
        links_for_crawling: 8,
        screenshot_captured: false,
        processing_duration: Duration::from_millis(200),
    };

    let page_event = CrawlEvent::page_crawled(
        "https://test.com/page".to_string(),
        PathBuf::from("/test/page.html"),
        1,
        metadata,
    );

    match page_event {
        CrawlEvent::PageCrawled {
            url,
            depth,
            metadata,
            ..
        } => {
            assert_eq!(url, "https://test.com/page");
            assert_eq!(depth, 1);
            assert_eq!(metadata.html_size, 2048);
            assert_eq!(metadata.links_found, 15);
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_filtered_receiver() {
    let bus = CrawlEventBus::new(10);

    // Filter for only PageCrawled events
    let mut filtered_receiver =
        bus.subscribe_filtered(|event| matches!(event, CrawlEvent::PageCrawled { .. }));

    // Publish a CrawlStarted event (should be filtered out)
    let start_event =
        CrawlEvent::crawl_started("https://test.com".to_string(), PathBuf::from("/test"), 2);
    let _ = bus.publish(start_event).await;

    // Publish a PageCrawled event (should pass filter)
    let metadata = PageCrawlMetadata {
        html_size: 1024,
        compressed_size: 512,
        links_found: 5,
        links_for_crawling: 3,
        screenshot_captured: true,
        processing_duration: Duration::from_millis(100),
    };
    let page_event = CrawlEvent::page_crawled(
        "https://test.com/page".to_string(),
        PathBuf::from("/test/page.html"),
        1,
        metadata,
    );
    let _ = bus.publish(page_event.clone()).await;

    // Should receive only the PageCrawled event
    let received = match timeout(Duration::from_millis(100), filtered_receiver.recv()).await {
        Ok(Ok(event)) => event,
        Ok(Err(e)) => panic!("Failed to receive filtered event: {e}"),
        Err(_) => panic!("Timeout waiting for filtered event"),
    };

    match (&page_event, &received) {
        (CrawlEvent::PageCrawled { url: url1, .. }, CrawlEvent::PageCrawled { url: url2, .. }) => {
            assert_eq!(url1, url2);
        }
        _ => panic!("Event types don't match"),
    }
}

#[tokio::test]
async fn test_filtered_receiver_would_receive() {
    let bus = CrawlEventBus::new(10);

    // Filter for only LinkRewriteCompleted events
    let filtered_receiver =
        bus.subscribe_filtered(|event| matches!(event, CrawlEvent::LinkRewriteCompleted { .. }));

    let start_event =
        CrawlEvent::crawl_started("https://test.com".to_string(), PathBuf::from("/test"), 2);
    assert!(!filtered_receiver.would_receive(&start_event));

    let link_event =
        CrawlEvent::link_rewrite_completed("https://test.com/target".to_string(), 5, 20);
    assert!(filtered_receiver.would_receive(&link_event));
}

#[tokio::test]
async fn test_batch_publish() {
    let bus = CrawlEventBus::new(50);
    let mut receiver = bus.subscribe();

    let events = vec![
        CrawlEvent::crawl_started("https://test1.com".to_string(), PathBuf::from("/test1"), 2),
        CrawlEvent::crawl_started("https://test2.com".to_string(), PathBuf::from("/test2"), 3),
        CrawlEvent::link_rewrite_completed("https://test.com/target".to_string(), 5, 20),
    ];

    let result = bus.publish_batch(events).await;
    assert!(result.is_complete());
    assert_eq!(result.published, 3); // All three events published
    assert_eq!(result.failed, 0); // No failures

    // Should receive all three events
    for i in 0..3 {
        match timeout(Duration::from_millis(100), receiver.recv()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => panic!("Failed to receive event {}: {}", i + 1, e),
            Err(_) => panic!("Timeout waiting for event {}", i + 1),
        }
    }
}

#[test]
fn test_event_bus_config() {
    let config = EventBusConfig {
        capacity: 500,
        backpressure_mode:
            kodegen_tools_citescrape::crawl_events::config::BackpressureMode::default(),
        overload_threshold: 0.8,
        enable_batching: true,
        max_batch_size: 50,
        batch_timeout_ms: 200,
        enable_metrics: false,
    };

    let bus = CrawlEventBus::with_config(config.clone());
    assert_eq!(bus.config().capacity, 500);
    assert!(bus.config().enable_batching);
    assert_eq!(bus.config().max_batch_size, 50);
    assert_eq!(bus.config().batch_timeout_ms, 200);
    assert!(!bus.config().enable_metrics);
}

#[test]
fn test_metrics_report() {
    let bus = CrawlEventBus::new(10);

    // Test with metrics enabled
    let report = bus.get_metrics_report();
    assert!(report.contains("Event Bus Metrics:"));
    assert!(report.contains("Events Published: 0"));
    assert!(report.contains("Success Rate: 100.00%"));

    // Test with metrics disabled
    let config = EventBusConfig {
        enable_metrics: false,
        ..Default::default()
    };
    let bus_no_metrics = CrawlEventBus::with_config(config);
    let report_disabled = bus_no_metrics.get_metrics_report();
    assert_eq!(report_disabled, "Metrics disabled");
}

#[tokio::test]
async fn test_block_backpressure_no_race_condition() {
    // This test verifies the fix for the TOCTOU race condition in BackpressureMode::Block
    //
    // Scenario:
    // 1. Create a bus with small capacity (10)
    // 2. Create a slow subscriber that doesn't drain quickly
    // 3. Spawn multiple concurrent publishers (20 publishers, 5 events each = 100 events total)
    // 4. Verify ALL events are received without drops
    //
    // Before fix: Intermittent failures as race window allows silent drops
    // After fix: Consistent success as semaphore prevents TOCTOU race

    use kodegen_tools_citescrape::crawl_events::config::BackpressureMode;

    let config = EventBusConfig {
        capacity: 10,
        backpressure_mode: BackpressureMode::Block,
        enable_metrics: true,
        ..Default::default()
    };

    let bus = CrawlEventBus::with_config(config);
    let mut receiver = bus.subscribe();

    // Spawn 20 concurrent publishers, each sending 5 events
    let num_publishers = 20;
    let events_per_publisher = 5;
    let total_events = num_publishers * events_per_publisher;

    let mut publisher_handles = vec![];

    for publisher_id in 0..num_publishers {
        let bus_clone = bus.clone();
        let handle = tokio::spawn(async move {
            for event_id in 0..events_per_publisher {
                let event = CrawlEvent::page_crawled(
                    format!("https://example.com/publisher{publisher_id}/page{event_id}"),
                    PathBuf::from(format!("/output/p{publisher_id}_e{event_id}.html")),
                    1,
                    PageCrawlMetadata {
                        html_size: 1024,
                        compressed_size: 512,
                        links_found: 10,
                        links_for_crawling: 5,
                        screenshot_captured: true,
                        processing_duration: Duration::from_millis(50),
                    },
                );

                // Use publish_with_backpressure in Block mode
                // This should block when channel is full, never drop events
                match bus_clone.publish_with_backpressure(event).await {
                    Ok(_) => {}
                    Err(e) => panic!("Publisher {publisher_id} event {event_id} failed: {e:?}"),
                }
            }
        });
        publisher_handles.push(handle);
    }

    // Collect all published events in a separate task
    let receiver_handle = tokio::spawn(async move {
        let mut received_events = vec![];

        // Receive all events with timeout
        for i in 0..total_events {
            match timeout(Duration::from_secs(10), receiver.recv()).await {
                Ok(Ok(event)) => {
                    received_events.push(event);
                }
                Ok(Err(e)) => {
                    panic!("Failed to receive event {i}: {e}");
                }
                Err(_) => {
                    panic!(
                        "Timeout receiving event {} (received {}/{})",
                        i,
                        received_events.len(),
                        total_events
                    );
                }
            }
        }

        received_events
    });

    // Wait for all publishers to complete
    for (idx, handle) in publisher_handles.into_iter().enumerate() {
        match timeout(Duration::from_secs(10), handle).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => panic!("Publisher {idx} panicked: {e:?}"),
            Err(_) => panic!("Publisher {idx} timed out"),
        }
    }

    // Wait for receiver to collect all events
    let received_events = match timeout(Duration::from_secs(10), receiver_handle).await {
        Ok(Ok(events)) => events,
        Ok(Err(e)) => panic!("Receiver panicked: {e:?}"),
        Err(_) => panic!("Receiver timed out"),
    };

    // Critical assertion: ALL events must be received
    assert_eq!(
        received_events.len(),
        total_events,
        "Expected {} events but received {}. Events were dropped!",
        total_events,
        received_events.len()
    );

    // Verify metrics show no drops
    let metrics = bus.metrics().snapshot();
    assert_eq!(
        metrics.events_dropped, 0,
        "Metrics show {} events dropped, but Block mode should never drop events",
        metrics.events_dropped
    );

    println!(
        "✅ Race condition test passed: {total_events} events published and received with no drops"
    );
}
