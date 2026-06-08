use busca::get_similarity_ratio;
use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

/// Benchmarks the pure `TextDiff::ratio()` hot path on a fixed, committed pair
/// of sample files. Deterministic and free of any external fixture.
fn bench_get_similarity_ratio(c: &mut Criterion) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let reference = fs::read_to_string(manifest.join("sample_dir_hello_world/nested_dir/ref_B.py"))
        .expect("read reference fixture");
    let candidate = fs::read_to_string(manifest.join("sample_dir_hello_world/file_1.py"))
        .expect("read candidate fixture");

    c.bench_function("get_similarity_ratio", |b| {
        b.iter(|| get_similarity_ratio(black_box(&reference), black_box(&candidate)))
    });
}

criterion_group!(benches, bench_get_similarity_ratio);
criterion_main!(benches);
