use kodegen_simd::context::ProcessingContext;

#[test]
fn test_processing_context() {
    let mut ctx = ProcessingContext::new()
        .with_temperature(0.7)
        .with_top_k(Some(40))
        .with_top_p(Some(0.9))
        .with_token_history(vec![1, 2, 3]);

    assert_eq!(ctx.temperature, 0.7);
    assert_eq!(ctx.top_k, Some(40));
    assert_eq!(ctx.top_p, Some(0.9));
    assert_eq!(ctx.token_history(), &[1, 2, 3]);

    ctx.extend_history(&[4, 5]);
    assert_eq!(ctx.token_history(), &[1, 2, 3, 4, 5]);

    assert!(!ctx.should_stop(10, 0));

    let mut ctx_with_stop = ProcessingContext {
        stop_tokens: vec![10, 20],
        max_new_tokens: Some(5),
        ..Default::default()
    };

    assert!(ctx_with_stop.should_stop(10, 0)); // Stop token
    assert!(!ctx_with_stop.should_stop(15, 0)); // Not a stop token
    assert!(ctx_with_stop.should_stop(15, 5)); // Max tokens reached

    ctx_with_stop.start_timer();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let elapsed = match ctx_with_stop.elapsed() {
        Some(duration) => duration,
        None => panic!("Timer should be started before calling elapsed in test"),
    };
    assert!(elapsed > std::time::Duration::from_millis(5));
}
