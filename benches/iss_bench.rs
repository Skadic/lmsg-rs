extern crate lmsg_rs;

use criterion::{criterion_group, criterion_main, Criterion};
use lmsg_rs::iss;

fn bench_iss(c: &mut Criterion) {
    let mut input = std::fs::read("res/dna.10MB.txt").unwrap();
    input.push(0);

    c.bench_function("bench iss on dna 10MB", |b| b.iter(|| iss::iss(&input)));
}

criterion_group!(benches, bench_iss);
criterion_main!(benches);
