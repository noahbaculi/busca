# Changelog

All notable changes to this project are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - unreleased

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
