# The directory walk stays single-threaded; parallelizing it is deferred

`run_search_with_progress` walks the search root with a single-threaded `WalkDir`, collected fully into a `Vec` before the parallel scoring starts. A performance audit (finding B4) flagged this: on a large tree the serial walk could dominate, and the `into_par_iter()` scoring cannot begin until the walk finishes. The proposed fix was a parallel walker (ripgrep's `ignore` crate) plus a faster glob engine (`globset`). Both are dependency additions, and `ignore` brings gitignore semantics that change behavior unless explicitly disabled.

Before taking that on, we measured the walk's share of total search time. `benches/benchmarks.rs` has a `walk` benchmark that times the `WalkDir` collect in isolation. Read against the `run_search` variants from the same CI run, its median gives the walk's fraction of the whole, and taking both numbers from one run cancels between-run machine variance.

On the Django source tree (release tag 5.2.15), the walk was 2.06% of total search time in the most walk-heavy config (`top-1`, where top-N pruning skips the most candidate diffs) and 0.88% with no count limit. The diff scoring is where the time goes; against it the walk is a rounding error.

So a parallel walker cannot win much here, even in its best case, and B4 is deferred. The `walk` benchmark stays in the suite so the share can be re-checked when the workload changes. A tree of many tiny files would push the walk to a larger fraction, since each entry costs a `stat` while carrying almost no diff work. If that share climbs past roughly 10% of a realistic search, the parallel walker is worth revisiting. Until then it is dependency and behavior risk for a sub-percent gain.
