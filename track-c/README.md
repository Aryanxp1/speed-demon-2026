# Track C: Join — Team Lex

Latest leaderboard score: **82.648 points, K=44.974289**, scored commit `2ae1a6c`; T=0.023 s, M=9 MB, S=323. These are the displayed values in the latest 2 November 2026 screenshot. The SIMD region-clamp version has been evaluated; PERF.md records provenance and historical results.

The Rust engine adapts the supplied 5pm small-table and decode ideas into our base. The uniform compact heads and cold tails retain all 256 groups and 131,072 ID slots. The new candidate clamps region IDs in the batched SIMD decoder, retaining the original record loop and BOM handling. Hash values, EOF padding, statistics, ordered output and original parallel table setup remain unchanged. Names retain probabilistic fingerprints; rare collisions remain possible.

Warnings-denied offline Cargo and all 12 references pass at 1/4/12 CPU affinities; ASAN passes all 12 at 1/12, including combined 110,000-name / 255-country overflow. Source: 316 Rust plus seven Dockerfile lines. Pinned build settings and zero dependencies remain unchanged.

Run `solution regions.csv records.csv output.txt`. Defaults: `/data/regions.csv`, `/data/records.csv`, `/out/aggregates.txt`.
