// Tests extracted from src/core/generation/stats.rs

use kodegen_candle_agent::core::generation::stats::GenerationStatistics;
use std::thread;
use std::time::Duration;

#[test]
fn test_generation_timing() {
    let mut stats = GenerationStatistics::new();

    stats.start_generation();
    thread::sleep(Duration::from_millis(10));
    stats.stop_generation();

    assert!(stats.total_duration.as_millis() >= 10);
}

#[test]
fn test_tokens_per_second() {
    let mut stats = GenerationStatistics::new();
    stats.total_tokens = 100;
    stats.total_duration = Duration::from_secs(2);

    assert_eq!(stats.tokens_per_second(), 50.0);
}

#[test]
fn test_simd_utilization() {
    let mut stats = GenerationStatistics::new();
    stats.simd_operations = 80;
    stats.scalar_operations = 20;

    assert_eq!(stats.simd_utilization(), 80.0);
}

#[test]
fn test_cache_hit_rate() {
    let mut stats = GenerationStatistics::new();
    stats.cache_hits = 90;
    stats.cache_misses = 10;

    assert_eq!(stats.cache_hit_rate(), 90.0);
}

#[test]
fn test_efficiency_summary() {
    let mut stats = GenerationStatistics::new();
    stats.total_tokens = 100;
    stats.total_duration = Duration::from_secs(1);
    stats.simd_operations = 8;
    stats.scalar_operations = 2;
    stats.cache_hits = 45;
    stats.cache_misses = 5;
    stats.peak_memory_bytes = 1_048_576; // 1MB

    let summary = stats.efficiency_summary();
    assert!(summary.contains("100.00")); // tokens/sec
    assert!(summary.contains("80.0%")); // SIMD utilization
    assert!(summary.contains("90.0%")); // Cache hit rate
    assert!(summary.contains("1.0MB")); // Memory usage
}
