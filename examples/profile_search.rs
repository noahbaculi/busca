//! Profiling harness for `busca::run_search`. Not a test or a benchmark: it runs
//! a fixed number of searches over the Django fixture so a sampling profiler can
//! attribute time across the walk, the glob filter, the diff, and the sort.
//!
//! Build and record with samply (no sudo, opens the Firefox Profiler UI):
//!
//! ```text
//! cargo build --release --example profile_search
//! samply record ./target/release/examples/profile_search
//! ```
//!
//! `cargo flamegraph` (dtrace, needs sudo on macOS) is the fallback.

use busca::{run_search, Args};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let search_path = manifest.join("sample-comprehensive/django");
    if !search_path.is_dir() {
        eprintln!(
            "profile_search: fixture '{}' not found.\n\
             Clone it with:\n  \
             git clone --depth 1 --branch 5.2.15 https://github.com/django/django.git sample-comprehensive",
            search_path.display()
        );
        return;
    }

    let reference =
        fs::read_to_string(search_path.join("forms/models.py")).expect("read reference fixture");

    // count = Some(1): the most walk-heavy config. Top-N pruning skips the most
    // diffs here, so the serial walk is the largest share of the total, which is
    // where finding B4 (parallel walk) would help most.
    let args = Args::new(
        reference,
        search_path,
        Some(10_000),
        Some(1),
        None,
        vec!["*.py".to_string()],
        vec![],
    )
    .expect("build args");

    // Enough iterations to give the sampler roughly 10 to 20 seconds of work on a
    // laptop. Adjust if the profile is too short or too long.
    let iterations = 50;
    for _ in 0..iterations {
        let results = run_search(black_box(&args)).expect("search");
        black_box(results);
    }

    eprintln!("profile_search: completed {iterations} searches");
}
