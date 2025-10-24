//! Softmax benchmarks with various value ranges

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use kodegen_simd::ops::softmax::SOFTMAX_DISPATCH;
use kodegen_simd::runtime::{CpuFeatures, get_cpu_features};
use rand::Rng;
use std::hint::black_box;

/// Generate test data in specified range
fn generate_test_data(size: usize, min: f32, max: f32) -> Vec<f32> {
    let mut rng = rand::rng();
    (0..size).map(|_| rng.random_range(min..max)).collect()
}

fn bench_softmax_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("softmax_by_size");
    let sizes = [16, 64, 256, 1024, 4096, 16384, 65536];

    let cpu_features = get_cpu_features();
    eprintln!("Detected CPU features: {:?}", cpu_features);

    for &size in &sizes {
        let logits = generate_test_data(size, -10.0, 10.0);

        // Benchmark scalar
        group.bench_with_input(BenchmarkId::new("scalar", size), &size, |b, _| {
            b.iter(|| {
                let result =
                    (*SOFTMAX_DISPATCH).call_with_feature(black_box(&logits), CpuFeatures::Scalar);
                black_box(result)
            })
        }); // Benchmark SSE4.1
        if let Ok(_) = (*SOFTMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Sse41) {
            group.bench_with_input(BenchmarkId::new("sse41", size), &size, |b, _| {
                b.iter(|| {
                    let result = (*SOFTMAX_DISPATCH)
                        .call_with_feature(black_box(&logits), CpuFeatures::Sse41);
                    black_box(result)
                })
            });
        }

        // Benchmark AVX2
        if let Ok(_) = (*SOFTMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx2) {
            group.bench_with_input(BenchmarkId::new("avx2", size), &size, |b, _| {
                b.iter(|| {
                    let result = (*SOFTMAX_DISPATCH)
                        .call_with_feature(black_box(&logits), CpuFeatures::Avx2);
                    black_box(result)
                })
            });
        }

        // Benchmark AVX-512
        if let Ok(_) = (*SOFTMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx512) {
            group.bench_with_input(BenchmarkId::new("avx512", size), &size, |b, _| {
                b.iter(|| {
                    let result = (*SOFTMAX_DISPATCH)
                        .call_with_feature(black_box(&logits), CpuFeatures::Avx512);
                    black_box(result)
                })
            });
        } // Benchmark NEON
        if let Ok(_) = (*SOFTMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Neon) {
            group.bench_with_input(BenchmarkId::new("neon", size), &size, |b, _| {
                b.iter(|| {
                    let result = (*SOFTMAX_DISPATCH)
                        .call_with_feature(black_box(&logits), CpuFeatures::Neon);
                    black_box(result)
                })
            });
        }

        // Benchmark runtime dispatch
        group.bench_with_input(BenchmarkId::new("dispatch", size), &size, |b, _| {
            b.iter(|| {
                let result = SOFTMAX_DISPATCH.call(black_box(&logits));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_softmax_by_value_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("softmax_by_value_range");
    let size = 4096;

    let value_ranges = [
        ("small", -1.0, 1.0),
        ("medium", -10.0, 10.0),
        ("large", -100.0, 100.0),
    ];
    for (range_name, min, max) in value_ranges.iter() {
        let logits = generate_test_data(size, *min, *max);

        // Benchmark scalar
        group.bench_with_input(
            BenchmarkId::new("scalar", range_name),
            &logits,
            |b, data| {
                b.iter(|| {
                    let result =
                        (*SOFTMAX_DISPATCH).call_with_feature(black_box(data), CpuFeatures::Scalar);
                    // Verify sum is approximately 1.0 (lightweight correctness check)
                    if let Ok(probs) = &result {
                        let sum: f32 = probs.iter().sum();
                        debug_assert!(
                            (sum - 1.0).abs() < 0.01,
                            "Softmax probabilities should sum to ~1.0"
                        );
                    }
                    black_box(result)
                })
            },
        );

        // Benchmark AVX2
        if let Ok(_) = (*SOFTMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx2) {
            group.bench_with_input(BenchmarkId::new("avx2", range_name), &logits, |b, data| {
                b.iter(|| {
                    let result =
                        (*SOFTMAX_DISPATCH).call_with_feature(black_box(data), CpuFeatures::Avx2);
                    black_box(result)
                })
            });
        } // Benchmark AVX-512
        if let Ok(_) = (*SOFTMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx512) {
            group.bench_with_input(
                BenchmarkId::new("avx512", range_name),
                &logits,
                |b, data| {
                    b.iter(|| {
                        let result = (*SOFTMAX_DISPATCH)
                            .call_with_feature(black_box(data), CpuFeatures::Avx512);
                        black_box(result)
                    })
                },
            );
        }

        // Benchmark runtime dispatch
        group.bench_with_input(
            BenchmarkId::new("dispatch", range_name),
            &logits,
            |b, data| {
                b.iter(|| {
                    let result = SOFTMAX_DISPATCH.call(black_box(data));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_softmax_by_size, bench_softmax_by_value_range);
criterion_main!(benches);
