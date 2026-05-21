# Changelog

All notable changes to this project are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - unreleased

### Added

- `--min-similarity-ratio <FLOAT>` CLI flag. Filters out comparisons below the
  given threshold after sorting but before `--count` truncation.
- `busca::Error` enum (`#[non_exhaustive]`) returned by `Args::new` and
  propagated by `run_search`. Replaces panics on invalid globs and missing
  search paths.
- `busca::run_search_with_progress(&Args, F)` where `F: Fn(u64, u64) + Send +
  Sync`. Library entry point used by the CLI to drive its progress bar
  without the library depending on `indicatif`.
- README "Versioning" section declaring the Rust MSRV, Python MSRV, and the
  public Rust surface covered by semver.

### Changed (BREAKING)

Ubiquitous language cleanup so the code reads in the vocabulary established by
`CONTEXT.md`. See `docs/adr/0001-similarity-ratio-uses-textdiff-ratio.md` for
the rationale behind the similarity metric.

| Old name | New name |
|---|---|
| `busca_py.FileMatch` | `busca_py.FileComparison` |
| `busca::FileMatch` | `busca::FileComparison` |
| `FileMatch.percent_match` | `FileComparison.similarity_ratio` |
| `FileMatch.lines` | `FileComparison.content` |
| `busca::get_percent_matching_lines` | `busca::get_similarity_ratio` |
| `busca::format_file_matches` | `busca::format_file_comparisons` |
| `busca_py.search_for_lines(...)` | `busca_py.search(...)` |
| `--max-lines` (CLI) | `--max-file-lines` |
| `Args.max_lines` (Rust), `max_lines` (Python kwarg) | `max_file_lines` |
| `Args.include_patterns` / `exclude_patterns` | `Args.include_glob` / `exclude_glob` |
| `include_globs` / `exclude_globs` (Python kwargs, plural) | `include_glob` / `exclude_glob` (singular) |
| `parse_glob_pattern` was `pub` and panicked on bad globs | `pub(crate)` and returns `Result<Pattern, busca::Error>`; library callers go through `Args::new` |
| `compare_files` was `pub` and took `ParallelIterator<Item = Result<walkdir::DirEntry, walkdir::Error>>` | removed; library callers use `run_search` or `run_search_with_progress` |
| `Args { ... }` struct-literal construction outside the crate | `Args::new(...)` only (`Args` is now `#[non_exhaustive]`) |
| `requires-python = ">=3.7"` | `requires-python = ">=3.11"` |
| `Cargo.toml` had no `rust-version` | `rust-version = "1.80"` |

### Removed

- `atty` dependency. Replaced by `std::io::IsTerminal` (in std since Rust 1.70).
  Removes exposure to RUSTSEC-2021-0145.

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
