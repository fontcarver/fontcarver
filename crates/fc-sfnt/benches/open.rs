//! Benchmarks for opening representative Latin and CJK fonts.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fc_sfnt::{read_path, ReadOptions};

fn bench_open(criterion: &mut Criterion) {
    let options = ReadOptions::default();
    let dejavu = fc_fixtures::path("DejaVuSans.ttf");
    criterion.bench_function("open/dejavu_sans", |bencher| {
        bencher.iter(|| read_path(black_box(&dejavu), black_box(&options)))
    });

    if let Some(source_han_sans) = fc_fixtures::external("source-han-sans-sc-otf") {
        criterion.bench_function("open/source_han_sans_sc", |bencher| {
            bencher.iter(|| read_path(black_box(&source_han_sans), black_box(&options)))
        });
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_open
}
criterion_main!(benches);
