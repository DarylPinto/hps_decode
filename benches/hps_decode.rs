use criterion::{criterion_group, criterion_main, Criterion};
use hps_decode::Hps;

pub fn criterion_benchmark(c: &mut Criterion) {
    let bytes = std::fs::read("./test-data/test-song.hps").unwrap();
    c.bench_function("Parse bytes into HPS struct", |b| {
        b.iter(|| TryInto::<Hps>::try_into(&bytes).unwrap())
    });

    let hps: Hps = bytes.try_into().unwrap();
    c.bench_function("Decode HPS struct into PCM samples", |b| {
        b.iter(|| hps.decode())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
