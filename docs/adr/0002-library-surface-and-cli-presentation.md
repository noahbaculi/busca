# Library surface is `Args::new` + `run_search`; presentation lives in the CLI

The public Rust surface of `busca` is intentionally narrow in v3.0.0:

- `Args` (constructed only via `Args::new`)
- `run_search` and `run_search_with_progress`
- `FileComparison`
- `get_similarity_ratio`
- `format_file_comparisons`
- `busca::Error`

Everything else, including `parse_glob_vec`, `compare_file`, and `read_file`, is `pub(crate)` or private.

The motivation is forward-compat headroom. The library should be able to change how it walks, parses globs, and compares files without breaking downstream Rust embedders. Hiding `parse_glob_vec` and `compare_file` is the cheapest enforcement: callers go through `Args::new` and `run_search`, neither of which exposes `walkdir::DirEntry` or `glob::Pattern` in its public signature.

`format_file_comparisons` stays public despite being a presentation helper. It is pure, has no external dependencies in its signature, and gives library callers a quick "print the ranked list" path that they would otherwise have to reimplement. The doctest no longer reads sample files from disk, so the function is self-contained.

`run_search_with_progress` was added late in the design pass. It takes a `Fn(u64, u64) + Send + Sync` callback. The CLI binary uses it to drive its `indicatif` bar without the library depending on `indicatif`. External callers who do not need progress reporting can use `run_search`, which is a one-line wrapper.

`Args` is marked `#[non_exhaustive]` so fields can be added in future minor releases. `Error` is also `#[non_exhaustive]` so new variants can be added (for example a `ReferenceUnreadable` variant if a future `load_reference_from_path` helper lands).
