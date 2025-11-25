use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use orbit::core::delta::{
    algorithm::{generate_delta_rolling, SignatureIndex},
    checksum::generate_signatures,
    HashAlgorithm,
};
use rand::Rng;
use std::io::Cursor;

fn benchmark_delta_worst_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_throughput");

    let size = 10 * 1024 * 1024; // 10MB
    let mut rng = rand::thread_rng();
    let data: Vec<u8> = (0..size).map(|_| rng.gen()).collect();

    // Build a destination signature set that should not match random data.
    let block_size = 4096;
    let dest_signatures = {
        let dest_data = vec![0u8; size];
        let sigs = generate_signatures(Cursor::new(dest_data), block_size, HashAlgorithm::Blake3)
            .expect("signatures");
        SignatureIndex::new(sigs)
    };

    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("generate_delta_no_matches", |b| {
        b.iter(|| {
            let result = generate_delta_rolling(
                black_box(&data[..]),
                black_box(dest_signatures.clone()),
                black_box(HashAlgorithm::Blake3),
            );
            black_box(result).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_delta_worst_case);
criterion_main!(benches);
