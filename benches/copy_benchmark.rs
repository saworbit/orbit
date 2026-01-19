use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use orbit::{
    config::{CompressionType, CopyConfig},
    copy_file, get_zero_copy_capabilities,
};
use std::fs::File;
use std::hint::black_box;
use std::io::Write;
use tempfile::TempDir;

/// Create a test file with specified size
fn create_test_file(dir: &TempDir, name: &str, size_mb: usize) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();

    // Write in 1MB chunks to avoid memory issues
    let chunk = vec![0xABu8; 1024 * 1024]; // 1MB of test data
    for _ in 0..size_mb {
        file.write_all(&chunk).unwrap();
    }
    file.flush().unwrap();

    path
}

/// Benchmark zero-copy vs buffered copy for different file sizes
fn bench_copy_methods(c: &mut Criterion) {
    let caps = get_zero_copy_capabilities();

    if !caps.available {
        println!("Zero-copy not available on this platform, skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("copy_methods");

    for size_mb in [1, 10, 100].iter() {
        let temp = TempDir::new().unwrap();
        let source = create_test_file(&temp, "source.bin", *size_mb);

        group.throughput(Throughput::Bytes((*size_mb as u64) * 1024 * 1024));

        // Benchmark zero-copy
        group.bench_with_input(BenchmarkId::new("zero-copy", size_mb), size_mb, |b, _| {
            b.iter(|| {
                let dest = temp.path().join("dest_zc.bin");
                let config = CopyConfig {
                    use_zero_copy: true,
                    verify_checksum: false,
                    show_progress: false,
                    ..Default::default()
                };

                let stats = copy_file(&source, &dest, &config).unwrap();
                black_box(stats);

                // Clean up for next iteration
                std::fs::remove_file(&dest).ok();
            });
        });

        // Benchmark buffered copy
        group.bench_with_input(BenchmarkId::new("buffered", size_mb), size_mb, |b, _| {
            b.iter(|| {
                let dest = temp.path().join("dest_buf.bin");
                let config = CopyConfig {
                    use_zero_copy: false,
                    verify_checksum: false,
                    show_progress: false,
                    ..Default::default()
                };

                let stats = copy_file(&source, &dest, &config).unwrap();
                black_box(stats);

                std::fs::remove_file(&dest).ok();
            });
        });
    }

    group.finish();
}

/// Benchmark impact of checksum verification
fn bench_checksum_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("checksum_impact");
    let size_mb = 100;

    let temp = TempDir::new().unwrap();
    let source = create_test_file(&temp, "source.bin", size_mb);

    group.throughput(Throughput::Bytes((size_mb as u64) * 1024 * 1024));

    // Zero-copy without checksum
    group.bench_function("zero-copy-no-checksum", |b| {
        b.iter(|| {
            let dest = temp.path().join("dest.bin");
            let config = CopyConfig {
                use_zero_copy: true,
                verify_checksum: false,
                show_progress: false,
                ..Default::default()
            };

            let stats = copy_file(&source, &dest, &config).unwrap();
            black_box(stats);
            std::fs::remove_file(&dest).ok();
        });
    });

    // Zero-copy with post-copy checksum
    group.bench_function("zero-copy-with-checksum", |b| {
        b.iter(|| {
            let dest = temp.path().join("dest.bin");
            let config = CopyConfig {
                use_zero_copy: true,
                verify_checksum: true,
                show_progress: false,
                ..Default::default()
            };

            let stats = copy_file(&source, &dest, &config).unwrap();
            black_box(stats);
            std::fs::remove_file(&dest).ok();
        });
    });

    // Buffered with streaming checksum
    group.bench_function("buffered-streaming-checksum", |b| {
        b.iter(|| {
            let dest = temp.path().join("dest.bin");
            let config = CopyConfig {
                use_zero_copy: false,
                verify_checksum: true,
                show_progress: false,
                ..Default::default()
            };

            let stats = copy_file(&source, &dest, &config).unwrap();
            black_box(stats);
            std::fs::remove_file(&dest).ok();
        });
    });

    group.finish();
}

/// Benchmark different chunk sizes for buffered copy
fn bench_chunk_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_sizes");
    let size_mb = 50;

    let temp = TempDir::new().unwrap();
    let source = create_test_file(&temp, "source.bin", size_mb);

    group.throughput(Throughput::Bytes((size_mb as u64) * 1024 * 1024));

    for chunk_kb in [64, 256, 1024, 4096, 16384].iter() {
        group.bench_with_input(
            BenchmarkId::new("chunk", chunk_kb),
            chunk_kb,
            |b, &chunk_kb| {
                b.iter(|| {
                    let dest = temp.path().join("dest.bin");
                    let config = CopyConfig {
                        use_zero_copy: false,
                        verify_checksum: false,
                        show_progress: false,
                        chunk_size: chunk_kb * 1024,
                        ..Default::default()
                    };

                    let stats = copy_file(&source, &dest, &config).unwrap();
                    black_box(stats);
                    std::fs::remove_file(&dest).ok();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark small file overhead
fn bench_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_files");

    for size_kb in [1, 4, 16, 64, 256].iter() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("small.bin");
        let mut file = File::create(&path).unwrap();
        let data = vec![0xFFu8; size_kb * 1024];
        file.write_all(&data).unwrap();
        file.flush().unwrap();

        group.throughput(Throughput::Bytes((*size_kb as u64) * 1024));

        // Zero-copy (should be skipped for < 64KB)
        group.bench_with_input(BenchmarkId::new("zero-copy", size_kb), size_kb, |b, _| {
            b.iter(|| {
                let dest = temp.path().join("dest.bin");
                let config = CopyConfig {
                    use_zero_copy: true,
                    verify_checksum: false,
                    show_progress: false,
                    ..Default::default()
                };

                let stats = copy_file(&path, &dest, &config).unwrap();
                black_box(stats);
                std::fs::remove_file(&dest).ok();
            });
        });

        // Buffered
        group.bench_with_input(BenchmarkId::new("buffered", size_kb), size_kb, |b, _| {
            b.iter(|| {
                let dest = temp.path().join("dest.bin");
                let config = CopyConfig {
                    use_zero_copy: false,
                    verify_checksum: false,
                    show_progress: false,
                    ..Default::default()
                };

                let stats = copy_file(&path, &dest, &config).unwrap();
                black_box(stats);
                std::fs::remove_file(&dest).ok();
            });
        });
    }

    group.finish();
}

/// Benchmark compression overhead vs zero-copy
fn bench_compression_vs_zero_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_vs_zero_copy");
    let size_mb = 100;

    let temp = TempDir::new().unwrap();

    // Create compressible data (highly repetitive)
    let path = temp.path().join("compressible.bin");
    let mut file = File::create(&path).unwrap();
    let chunk = vec![0xAAu8; 1024 * 1024]; // Highly compressible
    for _ in 0..size_mb {
        file.write_all(&chunk).unwrap();
    }
    file.flush().unwrap();

    group.throughput(Throughput::Bytes((size_mb as u64) * 1024 * 1024));

    // Zero-copy (no compression)
    group.bench_function("zero-copy-uncompressed", |b| {
        b.iter(|| {
            let dest = temp.path().join("dest.bin");
            let config = CopyConfig {
                use_zero_copy: true,
                compression: CompressionType::None,
                verify_checksum: false,
                show_progress: false,
                ..Default::default()
            };

            let stats = copy_file(&path, &dest, &config).unwrap();
            black_box(stats);
            std::fs::remove_file(&dest).ok();
        });
    });

    // LZ4 compression
    group.bench_function("lz4-compression", |b| {
        b.iter(|| {
            let dest = temp.path().join("dest.bin");
            let config = CopyConfig {
                compression: CompressionType::Lz4,
                verify_checksum: false,
                show_progress: false,
                ..Default::default()
            };

            let stats = copy_file(&path, &dest, &config).unwrap();
            black_box(stats);
            std::fs::remove_file(&dest).ok();
        });
    });

    // Zstd level 3 compression
    group.bench_function("zstd-3-compression", |b| {
        b.iter(|| {
            let dest = temp.path().join("dest.bin");
            let config = CopyConfig {
                compression: CompressionType::Zstd { level: 3 },
                verify_checksum: false,
                show_progress: false,
                ..Default::default()
            };

            let stats = copy_file(&path, &dest, &config).unwrap();
            black_box(stats);
            std::fs::remove_file(&dest).ok();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_copy_methods,
    bench_checksum_impact,
    bench_chunk_sizes,
    bench_small_files,
    bench_compression_vs_zero_copy,
);

criterion_main!(benches);
