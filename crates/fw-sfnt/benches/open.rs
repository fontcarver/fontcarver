//! Benchmarks for opening representative Latin and CJK fonts.

use std::path::Path;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fw_sfnt::{read_path, ReadOptions};

const SOURCE_HAN_SANS: &str = "/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/\
SimplifiedChinese/SourceHanSansSC-Regular.otf";

fn bench_open(criterion: &mut Criterion) {
    let options = ReadOptions::default();
    let dejavu = fw_fixtures::path("DejaVuSans.ttf");
    criterion.bench_function("open/dejavu_sans", |bencher| {
        bencher.iter(|| read_path(black_box(&dejavu), black_box(&options)))
    });

    let source_han_sans = Path::new(SOURCE_HAN_SANS);
    if source_han_sans.is_file() {
        criterion.bench_function("open/source_han_sans_sc", |bencher| {
            bencher.iter(|| read_path(black_box(source_han_sans), black_box(&options)))
        });
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_open
}
criterion_main!(benches);
