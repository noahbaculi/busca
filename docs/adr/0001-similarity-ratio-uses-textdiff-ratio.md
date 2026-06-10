# Similarity ratio is `TextDiff::ratio()`, not line set membership

Busca's `similarity_ratio` (formerly `percent_match`) is the value returned by `similar::TextDiff::from_lines(reference, candidate).ratio()`, a Ratcliff/Obershelp similarity over the two line sequences. The earlier doc comment on `get_percent_matching_lines` claimed it was "the percentage of lines from the reference that also exist in the candidate," which is not what the code computes and not what we want.

We chose the diff ratio because it captures ordering and structural similarity, not just line presence. Two files that share every line in completely different orders should not score as a perfect match, and two files that differ by one inserted line should not drop sharply. The `similar` crate already implements this well and is the source of the CLI's inline diff view, so reusing it keeps the scoring metric and the diff display consistent.

The trade-off: the metric is harder to explain than "fraction of overlapping lines," and users sometimes read `0.43` as "43% of lines match." That misread is now addressed by the rename to `similarity_ratio` and by an explicit definition in `CONTEXT.md`.
