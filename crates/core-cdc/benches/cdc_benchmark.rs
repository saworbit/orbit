use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use orbit_core_cdc::{ChunkConfig, ChunkStream};
use std::hint::black_box;
use std::io::Cursor;

fn benchmark_cdc_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("cdc_small_file");
    let data = vec![0xAB; 1024 * 1024]; // 1MB
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("cdc_1mb", |b| {
        b.iter(|| {
            let stream = ChunkStream::new(Cursor::new(black_box(&data)), ChunkConfig::default());
            let chunks: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();
            black_box(chunks);
        });
    });

    group.finish();
}

fn benchmark_cdc_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("cdc_large_file");
    let data = vec![0xCD; 10 * 1024 * 1024]; // 10MB
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("cdc_10mb", |b| {
        b.iter(|| {
            let stream = ChunkStream::new(Cursor::new(black_box(&data)), ChunkConfig::default());
            let chunks: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();
            black_box(chunks);
        });
    });

    group.finish();
}

fn benchmark_fixed_vs_cdc(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixed_vs_cdc");
    let data = vec![0xEF; 5 * 1024 * 1024]; // 5MB
    group.throughput(Throughput::Bytes(data.len() as u64));

    // Fixed-block chunking (baseline)
    group.bench_function("fixed_64kb", |b| {
        b.iter(|| {
            let block_size = 64 * 1024;
            let mut chunks = Vec::new();
            let data_ref = black_box(&data);

            for (offset, chunk_data) in data_ref.chunks(block_size).enumerate() {
                let hash = blake3::hash(chunk_data);
                chunks.push((offset * block_size, chunk_data.len(), hash));
            }
            black_box(chunks);
        });
    });

    // CDC chunking
    group.bench_function("cdc_64kb_avg", |b| {
        b.iter(|| {
            let stream = ChunkStream::new(Cursor::new(black_box(&data)), ChunkConfig::default());
            let chunks: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();
            black_box(chunks);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_cdc_small,
    benchmark_cdc_large,
    benchmark_fixed_vs_cdc
);
criterion_main!(benches);
