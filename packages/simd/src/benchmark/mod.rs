//! Benchmarking utilities for SIMD operations

use std::io::Write;
use std::time::{Duration, Instant};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Results from a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the benchmark
    pub name: String,
    /// Number of operations performed
    pub operations: usize,
    /// Total duration of the benchmark
    pub duration: Duration,
    /// Operations per second
    pub ops_per_sec: f64,
}

/// Run a benchmark with the given operation
///
/// # Arguments
///
/// * `name` - Name of the benchmark
/// * `iterations` - Number of iterations to run
/// * `operation` - Closure containing the operation to benchmark
///
/// # Returns
///
/// Returns a `BenchmarkResult` with the benchmark statistics
pub fn run_benchmark<F>(name: &str, iterations: usize, mut operation: F) -> BenchmarkResult
where
    F: FnMut(usize),
{
    let _start = Instant::now();

    // Warmup
    for i in 0..iterations.min(10) {
        operation(i);
    }

    let start = Instant::now();
    for i in 0..iterations {
        operation(i);
    }
    let duration = start.elapsed();

    let ops_per_sec = (iterations as f64) / duration.as_secs_f64();

    BenchmarkResult {
        name: name.to_string(),
        operations: iterations,
        duration,
        ops_per_sec,
    }
}

/// Benchmark logits processing functions
#[must_use]
pub fn benchmark_logits_processing() -> Vec<BenchmarkResult> {
    let mut results = Vec::new();

    // Test with different input sizes
    for &size in &[128, 512, 2048, 8192] {
        let mut logits = vec![0.0f32; size];

        // Initialize with random values
        for x in &mut logits {
            *x = rand::random::<f32>();
        }

        // Benchmark temperature scaling
        let temp_result = run_benchmark(&format!("temperature_scaling_{size}"), 1000, |_| {
            let _ = crate::logits::processing::apply_temperature_scaling_simd(&mut logits, 0.7);
        });
        results.push(temp_result);

        // Benchmark top-k filtering
        let topk_result = run_benchmark(&format!("topk_filtering_{size}"), 1000, |_| {
            let _ = crate::logits::topk::topk_filtering_simd(&mut logits, size / 2);
        });
        results.push(topk_result);

        // Benchmark normalization
        let norm_result = run_benchmark(&format!("normalization_{size}"), 1000, |_| {
            let _ = crate::logits::processing::normalize_probabilities_simd(&mut logits);
        });
        results.push(norm_result);
    }

    results
}

/// Print benchmark results in a formatted table
pub fn print_benchmark_results(results: &[BenchmarkResult]) -> std::io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n=== Benchmark Results ===")?;
    stdout.reset()?;
    writeln!(&mut stdout)?;

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
    writeln!(
        &mut stdout,
        "{:<30} | {:>12} | {:>12} | {:>12}",
        "Test", "Ops", "Time (ms)", "Ops/s"
    )?;
    stdout.reset()?;
    writeln!(&mut stdout, "{:-<80}", "")?;

    for result in results {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        writeln!(
            &mut stdout,
            "{:<30} | {:>12} | {:>12.2} | {:>12.2}",
            result.name,
            result.operations,
            result.duration.as_secs_f64() * 1000.0,
            result.ops_per_sec
        )?;
    }
    stdout.reset()?;

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n=== End of Results ===")?;
    stdout.reset()?;
    writeln!(&mut stdout)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_runner() {
        let result = run_benchmark("test", 1000, |i| {
            // Simple operation
            let _ = i * i;
        });

        assert_eq!(result.name, "test");
        assert_eq!(result.operations, 1000);
        assert!(result.duration.as_secs_f64() > 0.0);
        assert!(result.ops_per_sec > 0.0);
    }

    #[test]
    fn test_benchmark_logits_processing() {
        let results = benchmark_logits_processing();
        assert!(!results.is_empty());

        // Just verify the structure, not the actual performance
        for result in results {
            assert!(!result.name.is_empty());
            assert!(result.operations > 0);
            assert!(result.duration.as_secs_f64() >= 0.0);
            assert!(result.ops_per_sec >= 0.0);
        }
    }
}
