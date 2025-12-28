use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use orbit::core::delta::{
    algorithm::{generate_delta_rolling, SignatureIndex},
    checksum::generate_signatures,
    HashAlgorithm, RollingHashAlgo,
};
use rand::RngCore;
use std::hint::black_box;
use std::io::Cursor;

fn benchmark_delta_worst_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_throughput");

    let size = 10 * 1024 * 1024; // 10MB
    let mut rng = rand::rng();
    let mut data = vec![0u8; size];
    rng.fill_bytes(&mut data);

    // Build a destination signature set that should not match random data.
    let block_size = 4096;
    let dest_signatures = {
        let dest_data = vec![0u8; size];
        let sigs = generate_signatures(
            Cursor::new(dest_data),
            block_size,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
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
                black_box(RollingHashAlgo::Gear64),
            );
            black_box(result).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_delta_worst_case);
criterion_main!(benches);
