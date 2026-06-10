# busca's scripting contract: exit codes, JSON, and a shared threshold

busca was built for a human at a terminal: it ranks files and drops into an
interactive picker. Three changes in 3.0.0 make it a usable building block for
scripts and other programs.

## Exit codes follow grep

`0` means at least one comparison survived `min_similarity_ratio` and `count`.
`1` means nothing matched. `2` means an error (bad glob, missing search path,
unreadable reference). Previously the empty case exited `0` with a stdout
message and every error exited `1`, so a caller could not tell "found nothing"
from "failed" without parsing stdout. The empty-result message moved to stderr
so stdout carries only results, which keeps `--format json` and the human grid
clean for piping.

## JSON formatting stays in the CLI

`--format json` emits an array of `{ path, similarity_ratio }`, with `content`
added under `--with-content`. The serializer lives in `src/main.rs`, not in the
library. This keeps the library surface as ADR-0002 defines it, and it keeps
`serde` out of the code reachable from the PyO3 module, so it is
dead-code-eliminated from the wheel. `path` is rendered with `Display` to match
the human grid and stay infallible on the UTF-8 paths busca surfaces;
`similarity_ratio` is the raw `f32`, which serde_json prints in its shortest
round-trip form.

## `min_similarity_ratio` is a search parameter

It moved from a CLI-only post-filter into `Args`/`run_search` and the
`busca_py.search` kwarg, so the CLI, the Rust library, and Python all express
the same filter. The bounded top-N search uses it as a heap floor, so a file
whose upper bound is below the floor skips its full diff. This is the fast-follow
recorded in the top-N pruning design. Moving the filter into the search changes
one invariant: `run_search` no longer returns a comparison for every candidate
once the floor is set. CONTEXT.md's "Comparison" definition is updated to match.
