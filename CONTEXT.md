# Busca

Busca finds files whose contents most closely resemble a reference string. It powers a CLI and a Python library (`busca_py`) over the same Rust core.

## Language

**Reference**:
The input string (or file contents loaded from `--ref-file-path`) that every candidate is scored against. The reference is fixed for a given run.
_Avoid_: source, target, query, input

**Search root**:
The path passed to `--search-path` (or `search_path` in the Python API). Walked recursively to enumerate candidates. May be a directory tree or a single file (a one-candidate degenerate case).
_Avoid_: search directory, search dir, scan path

**Candidate file**:
A file reached by walking the search root that survives the include glob, exclude glob, and `max_file_lines` filters. Each candidate produces exactly one `FileComparison`.
_Avoid_: comp file, match file

**Search**:
The end-to-end operation: walk the search root, filter to candidates, score each candidate against the reference, and return them ranked by `similarity_ratio`. A search produces zero or more `FileComparison`s.
_Avoid_: scan, lookup

**Comparison** (`FileComparison`):
The result of scoring one candidate against the reference. Holds the candidate's path, its `similarity_ratio`, and its contents. A comparison is produced for every candidate, including ones with `similarity_ratio == 0.0`.
_Avoid_: FileMatch, match, hit, result

**Similarity ratio** (`similarity_ratio`):
A float in `[0.0, 1.0]` produced by `similar::TextDiff::ratio()`. This is a Ratcliff/Obershelp similarity over the line sequences of the reference and candidate. It is not "the fraction of reference lines that appear in the candidate." See ADR-0001.
_Avoid_: percent_match, match score, line match percentage

**Include glob** / **Exclude glob** (`include_glob`, `exclude_glob`):
Glob patterns (per the `glob` crate) applied to candidate paths. A candidate is kept only if it matches at least one include glob (when any are given) and matches no exclude glob.
_Avoid_: include substring, exclude substring, filter pattern

**`max_file_lines`**:
A per-candidate size filter. A candidate whose line count exceeds `max_file_lines` is skipped entirely, not truncated. Empty files (zero lines) are also skipped.
_Avoid_: max_lines (as a "consider only the first N lines" reading)

**`count`**:
The maximum number of `FileComparison`s returned, taken from the top of the descending `similarity_ratio` ranking. Defaults: `10` (CLI), unlimited (Python).
_Avoid_: limit, top_k, results

## Flagged ambiguities

- **"match"**: historically used both for "we produced a result for this file" and for "this file actually resembles the reference." Resolved by reserving `FileComparison` for the former and reading `similarity_ratio` for the latter. The Rust type `FileMatch` and the Python `FileMatch` class are scheduled for rename to `FileComparison`.
- **"search" with a single-file search root**: a search with one candidate is still a search, not a special "compare" mode. The CLI accepts a file at `--search-path` and treats it as a one-candidate search.

## Example dialogue

**Dev**: I ran busca with `--ref-file-path foo.py --search-path ./src` and got a `FileComparison` for `unrelated.json` with `similarity_ratio == 0.0`. Is that a bug?

**Domain**: No. Every candidate that passes the include/exclude globs and `max_file_lines` produces a comparison, even a zero one. If you want to suppress those, narrow the include glob or take a smaller `count`.

**Dev**: Got it. And `similarity_ratio` of `0.43`, that means 43% of the reference lines appear in the candidate?

**Domain**: No, that is the most common misread. It is the `TextDiff::ratio()` from the `similar` crate, a Ratcliff/Obershelp similarity over the line sequences. A line that appears in both contributes, but so does the ordering and the overall structure. See ADR-0001 for the trade-off.

**Dev**: Why is `unrelated.json` even a candidate? I only care about Python files.

**Domain**: You did not pass an include glob. Add `--include-glob '*.py'` and JSON files will be filtered out before they become candidates.
