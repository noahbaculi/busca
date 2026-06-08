use busca::{get_similarity_ratio, run_search, Args};
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

/// Benchmarks a full search over the `django/` package source, pinned to
/// Django release tag 5.2.15 (commit 21e98408). A cohesive single codebase
/// gives a realistic similarity distribution, and a real multi-hundred-line
/// module as the reference makes each candidate diff carry real cost. Those are
/// the conditions audit finding B2 (top-N pruning) needs to be measured
/// against. Skips itself (no panic) when the fixture is absent so the
/// micro-benchmark still runs locally without the clone.
fn bench_run_search(c: &mut Criterion) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let search_path = manifest.join("sample-comprehensive/django");
    if !search_path.is_dir() {
        eprintln!(
            "skipping run_search benchmark: fixture '{}' not found.\n\
             Clone it with:\n  \
             git clone --depth 1 --branch 5.2.15 https://github.com/django/django.git sample-comprehensive",
            search_path.display()
        );
        return;
    }

    // A real Django module, present in the tree at the pinned tag, so it scores
    // against its own cohesive corpus rather than a synthetic snippet.
    let reference =
        fs::read_to_string(search_path.join("forms/models.py")).expect("read reference fixture");
    // `count` drives audit finding B2: a small top-N is where a running
    // threshold could prune full diffs, while `None` returns every candidate
    // and must stay a no-op. Benching all three lets an audit branch prove the
    // win on small counts without regressing the unbounded library default.
    let count_variants = [
        ("top-1", Some(1)),
        ("top-10", Some(10)),
        ("top-20", Some(20)),
        ("unbounded", None),
    ];

    let mut group = c.benchmark_group("run_search");
    group.sample_size(10);
    for (label, count) in count_variants {
        let args = Args::new(
            reference.clone(),
            search_path.clone(),
            Some(10_000),
            count,
            vec!["*.py".to_string()],
            vec![],
        )
        .expect("build args");
        group.bench_function(label, |b| {
            b.iter(|| run_search(black_box(&args)).expect("search"))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_get_similarity_ratio, bench_run_search);
criterion_main!(benches);
