# Track C: Join — Team Lex

Verified standing best: **82.458848 points, K=44.583786**, run `run-1b583e4bdb40`; T=0.023070 s, M=9,080,832 bytes, S=319. The setup-policy trial scored 82.294904 and is reverted. The isolated SIMD region-clamp candidate awaits official evaluation. PERF.md records provenance and all results.

The Rust engine adapts the supplied 5pm small-table and decode ideas into our base. The uniform compact heads and cold tails retain all 256 groups and 131,072 ID slots. The new candidate clamps region IDs in the batched SIMD decoder, retaining the original record loop and BOM handling. Hash values, EOF padding, statistics, ordered output and original parallel table setup remain unchanged. Names retain probabilistic fingerprints; rare collisions remain possible.

Warnings-denied offline Cargo and all 12 references pass at 1/4/12 CPU affinities; ASAN passes all 12 at 1/12, including combined 110,000-name / 255-country overflow. Source: 316 Rust plus seven Dockerfile lines. Pinned build settings and zero dependencies remain unchanged.

Run `solution regions.csv records.csv output.txt`. Defaults: `/data/regions.csv`, `/data/records.csv`, `/out/aggregates.txt`.
