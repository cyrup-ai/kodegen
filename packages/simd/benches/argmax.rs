//! Argmax benchmarks with various data distributions

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use kodegen_simd::ops::argmax::ARGMAX_DISPATCH;
use kodegen_simd::runtime::{CpuFeatures, get_cpu_features};
use rand::Rng;
use std::hint::black_box;

/// Generate random test data
fn generate_random_data(size: usize) -> Vec<f32> {
    let mut rng = rand::rng();
    (0..size).map(|_| rng.random_range(-10.0..10.0)).collect()
}

/// Generate sorted ascending data
fn generate_sorted_ascending(size: usize) -> Vec<f32> {
    (0..size).map(|i| i as f32).collect()
}

/// Generate sorted descending data
fn generate_sorted_descending(size: usize) -> Vec<f32> {
    (0..size).rev().map(|i| i as f32).collect()
}

/// Generate data with all equal values
fn generate_all_equal(size: usize) -> Vec<f32> {
    vec![5.0; size]
}

/// Generate data with max at beginning
fn generate_max_at_beginning(size: usize) -> Vec<f32> {
    let mut data = vec![1.0; size];
    if !data.is_empty() {
        data[0] = 100.0;
    }
    data
}

/// Generate data with max at end
fn generate_max_at_end(size: usize) -> Vec<f32> {
    let mut data = vec![1.0; size];
    if !data.is_empty() {
        data[size - 1] = 100.0;
    }
    data
}

/// Generate data with max in middle
fn generate_max_in_middle(size: usize) -> Vec<f32> {
    let mut data = vec![1.0; size];
    if !data.is_empty() {
        data[size / 2] = 100.0;
    }
    data
}

fn bench_argmax_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("argmax_by_size");
    let sizes = [16, 64, 256, 1024, 4096, 16384, 65536];
    let cpu_features = get_cpu_features();
    eprintln!("Detected CPU features: {:?}", cpu_features);

    for &size in &sizes {
        let logits = generate_random_data(size);

        // Benchmark scalar
        group.bench_with_input(BenchmarkId::new("scalar", size), &size, |b, _| {
            b.iter(|| {
                let result =
                    (*ARGMAX_DISPATCH).call_with_feature(black_box(&logits), CpuFeatures::Scalar);
                black_box(result)
            })
        });

        // Benchmark SSE4.1
        if let Ok(_) = (*ARGMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Sse41) {
            group.bench_with_input(BenchmarkId::new("sse41", size), &size, |b, _| {
                b.iter(|| {
                    let result = (*ARGMAX_DISPATCH)
                        .call_with_feature(black_box(&logits), CpuFeatures::Sse41);
                    black_box(result)
                })
            });
        }

        // Benchmark AVX2
        if let Ok(_) = (*ARGMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx2) {
            group.bench_with_input(BenchmarkId::new("avx2", size), &size, |b, _| {
                b.iter(|| {
                    let result =
                        (*ARGMAX_DISPATCH).call_with_feature(black_box(&logits), CpuFeatures::Avx2);
                    black_box(result)
                })
            });
        }

        // Benchmark AVX-512
        if let Ok(_) = (*ARGMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx512) {
            group.bench_with_input(BenchmarkId::new("avx512", size), &size, |b, _| {
                b.iter(|| {
                    let result = (*ARGMAX_DISPATCH)
                        .call_with_feature(black_box(&logits), CpuFeatures::Avx512);
                    black_box(result)
                })
            });
        }

        // Benchmark NEON
        if let Ok(_) = (*ARGMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Neon) {
            group.bench_with_input(BenchmarkId::new("neon", size), &size, |b, _| {
                b.iter(|| {
                    let result =
                        (*ARGMAX_DISPATCH).call_with_feature(black_box(&logits), CpuFeatures::Neon);
                    black_box(result)
                })
            });
        }

        // Benchmark runtime dispatch
        group.bench_with_input(BenchmarkId::new("dispatch", size), &size, |b, _| {
            b.iter(|| {
                let result = ARGMAX_DISPATCH.call(black_box(&logits));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_argmax_by_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("argmax_by_distribution");
    let size = 4096;

    let distributions = [
        ("random", generate_random_data(size)),
        ("sorted_asc", generate_sorted_ascending(size)),
        ("sorted_desc", generate_sorted_descending(size)),
        ("all_equal", generate_all_equal(size)),
        ("max_at_begin", generate_max_at_beginning(size)),
        ("max_at_end", generate_max_at_end(size)),
        ("max_in_middle", generate_max_in_middle(size)),
    ];

    for (dist_name, logits) in distributions.iter() {
        // Benchmark scalar
        group.bench_with_input(BenchmarkId::new("scalar", dist_name), &logits, |b, data| {
            b.iter(|| {
                let result =
                    (*ARGMAX_DISPATCH).call_with_feature(black_box(data), CpuFeatures::Scalar);
                black_box(result)
            })
        });

        // Benchmark AVX2 (if available)
        if let Ok(_) = (*ARGMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx2) {
            group.bench_with_input(BenchmarkId::new("avx2", dist_name), &logits, |b, data| {
                b.iter(|| {
                    let result =
                        (*ARGMAX_DISPATCH).call_with_feature(black_box(data), CpuFeatures::Avx2);
                    black_box(result)
                })
            });
        }

        // Benchmark AVX-512 (if available)
        if let Ok(_) = (*ARGMAX_DISPATCH).call_with_feature(&[1.0], CpuFeatures::Avx512) {
            group.bench_with_input(BenchmarkId::new("avx512", dist_name), &logits, |b, data| {
                b.iter(|| {
                    let result =
                        (*ARGMAX_DISPATCH).call_with_feature(black_box(data), CpuFeatures::Avx512);
                    black_box(result)
                })
            });
        }

        // Benchmark runtime dispatch
        group.bench_with_input(
            BenchmarkId::new("dispatch", dist_name),
            &logits,
            |b, data| {
                b.iter(|| {
                    let result = ARGMAX_DISPATCH.call(black_box(data));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_argmax_by_size, bench_argmax_by_distribution);
criterion_main!(benches);
