//! Temperature scaling benchmarks

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use kodegen_simd::ops::temperature::TEMPERATURE_DISPATCH;
use kodegen_simd::runtime::{CpuFeatures, get_cpu_features};
use rand::Rng;
use std::hint::black_box;

/// Generate test data for benchmarking
fn generate_test_data(size: usize) -> Vec<f32> {
    let mut rng = rand::rng();
    (0..size).map(|_| rng.random_range(-10.0..10.0)).collect()
}

fn bench_temperature_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("temperature_scaling");
    let sizes = [16, 64, 256, 1024, 4096, 16384, 65536];
    let temperature = 0.7f32;

    // Detect available CPU features for informational output
    let cpu_features = get_cpu_features();
    eprintln!("Detected CPU features: {:?}", cpu_features);

    for &size in &sizes {
        let logits = generate_test_data(size);

        // Benchmark scalar implementation (always available)
        group.bench_with_input(BenchmarkId::new("scalar", size), &size, |b, _| {
            b.iter(|| {
                let mut data = logits.clone();
                let result = (*TEMPERATURE_DISPATCH).call_with_feature(
                    black_box(&mut data),
                    black_box(temperature),
                    CpuFeatures::Scalar,
                );
                black_box(result)
            })
        });

        // Benchmark SSE4.1 implementation (if available)
        if let Ok(()) =
            (*TEMPERATURE_DISPATCH).call_with_feature(&mut vec![1.0], 1.0, CpuFeatures::Sse41)
        {
            group.bench_with_input(BenchmarkId::new("sse41", size), &size, |b, _| {
                b.iter(|| {
                    let mut data = logits.clone();
                    let result = (*TEMPERATURE_DISPATCH).call_with_feature(
                        black_box(&mut data),
                        black_box(temperature),
                        CpuFeatures::Sse41,
                    );
                    black_box(result)
                })
            });
        }

        // Benchmark AVX2 implementation (if available)
        if let Ok(()) =
            (*TEMPERATURE_DISPATCH).call_with_feature(&mut vec![1.0], 1.0, CpuFeatures::Avx2)
        {
            group.bench_with_input(BenchmarkId::new("avx2", size), &size, |b, _| {
                b.iter(|| {
                    let mut data = logits.clone();
                    let result = (*TEMPERATURE_DISPATCH).call_with_feature(
                        black_box(&mut data),
                        black_box(temperature),
                        CpuFeatures::Avx2,
                    );
                    black_box(result)
                })
            });
        }

        // Benchmark AVX-512 implementation (if available)
        if let Ok(()) =
            (*TEMPERATURE_DISPATCH).call_with_feature(&mut vec![1.0], 1.0, CpuFeatures::Avx512)
        {
            group.bench_with_input(BenchmarkId::new("avx512", size), &size, |b, _| {
                b.iter(|| {
                    let mut data = logits.clone();
                    let result = (*TEMPERATURE_DISPATCH).call_with_feature(
                        black_box(&mut data),
                        black_box(temperature),
                        CpuFeatures::Avx512,
                    );
                    black_box(result)
                })
            });
        }

        // Benchmark NEON implementation (if available on ARM)
        if let Ok(()) =
            (*TEMPERATURE_DISPATCH).call_with_feature(&mut vec![1.0], 1.0, CpuFeatures::Neon)
        {
            group.bench_with_input(BenchmarkId::new("neon", size), &size, |b, _| {
                b.iter(|| {
                    let mut data = logits.clone();
                    let result = (*TEMPERATURE_DISPATCH).call_with_feature(
                        black_box(&mut data),
                        black_box(temperature),
                        CpuFeatures::Neon,
                    );
                    black_box(result)
                })
            });
        }

        // Benchmark runtime dispatch (default API)
        group.bench_with_input(BenchmarkId::new("dispatch", size), &size, |b, _| {
            b.iter(|| {
                let mut data = logits.clone();
                let result =
                    TEMPERATURE_DISPATCH.call(black_box(&mut data), black_box(temperature));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_temperature_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("temperature_edge_cases");

    // Test with different temperature values
    let temperatures = [0.1f32, 0.5f32, 1.0f32, 2.0f32, 10.0f32];
    let size = 1024;
    let logits = generate_test_data(size);
    for &temp in &temperatures {
        group.bench_with_input(
            BenchmarkId::new("temperature_value", format!("{:.1}", temp)),
            &temp,
            |b, &t| {
                b.iter(|| {
                    let mut data = logits.clone();
                    let result = TEMPERATURE_DISPATCH.call(black_box(&mut data), black_box(t));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_temperature_scaling,
    bench_temperature_edge_cases
);
criterion_main!(benches);
