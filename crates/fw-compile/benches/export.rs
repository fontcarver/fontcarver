use criterion::{criterion_group, criterion_main, Criterion};
use fw_compile::{export, ExportOptions};

fn export_benchmark(criterion: &mut Criterion) {
    let font = fw_sfnt::read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &fw_sfnt::ReadOptions::default(),
    )
    .expect("read benchmark fixture")
    .fonts
    .remove(0);
    criterion.bench_function("export_dejavu", |bencher| {
        bencher.iter(|| export(&font, &|_| 0, &ExportOptions::default()))
    });
}

criterion_group!(benches, export_benchmark);
criterion_main!(benches);
