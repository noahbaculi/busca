# Changelog

All notable changes to this project are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - 2026-06-10

### Added

- `--min-similarity-ratio <FLOAT>` CLI flag. Drops comparisons whose similarity
  ratio is below the given threshold during the search, before the `--count`
  limit.
- `busca::Error` enum (`#[non_exhaustive]`) returned by `Args::new` and
  propagated by `run_search`. Replaces panics on invalid globs and missing
  search paths.
- `busca::run_search_with_progress(&Args, F)` where `F: Fn(u64, u64) + Send +
Sync`. Library entry point used by the CLI to drive its progress bar
  without the library depending on `indicatif`.
- README "Versioning" section declaring the Rust MSRV, Python MSRV, and the
  public Rust surface covered by semver.
- `--format <human|json>` CLI flag for structured output, with `--with-content`
  to include each file's body in the JSON. JSON carries `path` and
  `similarity_ratio` (and `content` when requested).
- `--no-interactive` CLI flag to print the ranked grid instead of launching the
  interactive picker.
- `min_similarity_ratio` is now a search parameter on `Args`/`run_search` and a
  `busca_py.search` kwarg, not just a CLI post-filter. The bounded search uses it
  as an additional pruning floor.
- `busca::Error::InvalidSimilarityRatio` variant, returned by `Args::new` when
  `min_similarity_ratio` is NaN or outside `[0.0, 1.0]`.

### Performance

- Searches that request a fixed result count (the CLI `--count`, or `count` on
  the Rust and Python APIs) now skip the full line diff for files that cannot
  place in the top results. Each candidate is checked first against two cheap
  upper bounds on `similar::TextDiff::ratio()`: a length bound, then a
  line-multiset bound. Both are built from `similar`'s own line tokenizer, so
  they share the tokenization of the full diff and stay true upper bounds. A
  file is diffed in full only when its bound is at least the lowest ratio
  currently kept. Candidates collect into a per-thread bounded heap that retains
  only `count` results, so peak memory holds about `count` files rather than
  every candidate's content at once. Output is byte-identical to before, tie
  order included. On the Django 5.2.15 source tree (CI, ubuntu-latest), a top-1
  search runs in about 43% of the unbounded time, top-10 in about 82%, and
  top-20 in about 94%. The unbounded search (no `count`) keeps its previous code
  path and is unchanged.

### Changed (BREAKING)

Ubiquitous language cleanup so the code reads in the vocabulary established by
`CONTEXT.md`. See `docs/adr/0001-similarity-ratio-uses-textdiff-ratio.md` for
the rationale behind the similarity metric.

| Old name                                                                                                | New name                                                                                         |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `busca_py.FileMatch`                                                                                    | `busca_py.FileComparison`                                                                        |
| `busca::FileMatch`                                                                                      | `busca::FileComparison`                                                                          |
| `FileMatch.percent_match`                                                                               | `FileComparison.similarity_ratio`                                                                |
| `FileMatch.lines`                                                                                       | `FileComparison.content`                                                                         |
| `busca::get_percent_matching_lines`                                                                     | `busca::get_similarity_ratio`                                                                    |
| `busca::format_file_matches`                                                                            | `busca::format_file_comparisons`                                                                 |
| `busca_py.search_for_lines(...)`                                                                        | `busca_py.search(...)`                                                                           |
| `--max-lines` (CLI)                                                                                     | `--max-file-lines`                                                                               |
| `Args.max_lines` (Rust), `max_lines` (Python kwarg)                                                     | `max_file_lines`                                                                                 |
| `Args.include_patterns` / `exclude_patterns`                                                            | `Args.include_glob` / `exclude_glob`                                                             |
| `include_globs` / `exclude_globs` (Python kwargs, plural)                                               | `include_glob` / `exclude_glob` (singular)                                                       |
| `parse_glob_pattern` was `pub` and panicked on bad globs                                                | removed; glob validation moved into `Args::new`, which returns `Error::InvalidGlob` on bad globs |
| `compare_files` was `pub` and took `ParallelIterator<Item = Result<walkdir::DirEntry, walkdir::Error>>` | removed; library callers use `run_search` or `run_search_with_progress`                          |
| `Args { ... }` struct-literal construction outside the crate                                            | `Args::new(...)` only (`Args` is now `#[non_exhaustive]`)                                        |
| `requires-python = ">=3.7"`                                                                             | `requires-python = ">=3.11"`                                                                     |
| `Cargo.toml` had no `rust-version`                                                                      | `rust-version = "1.85"`                                                                          |

- `Args.include_glob` and `Args.exclude_glob` were `pub Option<Vec<glob::Pattern>>`; they are now `pub(crate)`. Library callers go through `Args::new` with string globs.
- `Args::new` gains a `min_similarity_ratio: Option<f32>` parameter, positioned
  after `count` and before the globs.
- CLI exit codes now follow grep: `0` when at least one comparison matches, `1`
  when none match (previously `0` with a stdout message), and `2` on error
  (previously `1`). The empty-result message moved from stdout to stderr.

### Removed

- `atty` dependency. Replaced by `std::io::IsTerminal` (in std since Rust 1.70).
  Removes exposure to RUSTSEC-2021-0145.

### Dependencies

- Updated all dependencies to their latest releases, including `similar` 2 -> 3
  and `pyo3` 0.27 -> 0.28. `similar` 3 keeps the same `ratio()` definition, so
  the similarity scores and the top-N pruning bounds are unchanged. The `pyo3`
  module is marked `gil_used = true` to keep requiring the GIL rather than
  advertise free-threaded support. These bumps set the 1.85 MSRV above.

### Fixed

- The doc comment on `get_similarity_ratio` (formerly `get_percent_matching_lines`)
  previously claimed the function returns "the percentage of lines from the
  reference that also exist in the candidate." The function actually returns
  `similar::TextDiff::ratio()`, a Ratcliff/Obershelp similarity. The doc comment
  now matches the implementation.
- The README's "substring 'foo'" wording and `--include-glob '**foo**'` example
  were misleading. Globs are not substring matches; the example now uses
  `'**/*foo*'`.
- `pyproject.toml` now declares `dynamic = ["version"]` so maturin >= 1.13
  resolves the version from `Cargo.toml`.
- `busca_py.pyi` declared `FileComparison.path` as `str`. The runtime value is
  `pathlib.Path`; the stub now matches. The `include_glob` and `exclude_glob`
  parameters in the stub now show `str | list[str] | None`.
- The doctest on `format_file_comparisons` no longer reads sample files from
  disk; it constructs `FileComparison` values inline.
- `read_file` no longer panics on non-`InvalidData` IO errors. The candidate
  is skipped and one line is written to stderr.
- CLI no longer shows an empty interactive picker when zero comparisons
  survive filtering. The "No files found that match the criteria." path
  is now reachable.
- `--min-similarity-ratio` rejects NaN and values outside `[0.0, 1.0]`
  at argument-parse time, with a clear clap error.
- The Python `ValueError` raised when a bad-type element is passed to
  `include_glob` / `exclude_glob` now includes the inner conversion
  detail (e.g. the offending type) instead of a generic message.
- CLI IO error messages are formatted with `Display`, not `Debug`,
  yielding `"No such file or directory (os error 2)"` instead of
  `Os { code: 2, kind: NotFound, .. }`.
