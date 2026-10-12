use criterion::{criterion_group, criterion_main, Criterion};
use fc_compile::{export, ExportOptions};

fn export_benchmark(criterion: &mut Criterion) {
    let font = fc_sfnt::read_path(
        &fc_fixtures::path("DejaVuSans.ttf"),
        &fc_sfnt::ReadOptions::default(),
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
