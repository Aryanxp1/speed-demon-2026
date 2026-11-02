# Track C: Join — Team Lex

Verified standing best: **82.446607 points, K=44.558663**, run `run-d997ff843f5c`; T=0.022971 s, M=9,056,256 bytes, S=326. The earlier uniform-overflow experiment scored 82.254885 and is reverted. The early table-setup comparison awaits official evaluation. PERF.md records provenance and all results.

The Rust join retains the supplied 5pm layout/decode ideas in our capacity-preserving base: two hot huge pages, cold name/group overflow, AVX2 CSV masks, batched numeric decoding, atomic name IDs and distinct-name bits, and per-worker statistics. Names retain probabilistic fingerprints; rare collisions remain possible. All 256 groups and 131,072 ID slots remain available.

The candidate prepares the two hot table pages in main before disabling future transparent huge pages and creating workers. Parsing, hash values, region lookup, statistics, output, EOF handling and the existing barrier are unchanged. No local performance improvement is claimed; official evaluation decides retention.

Warnings-denied offline Cargo and all 12 references pass at 1/4/12 CPU affinities; ASAN passes all 12 at 1/12, including combined 110,000-name / 255-country overflow. Source: 321 Rust plus seven Dockerfile lines. Pinned build settings and zero dependencies remain unchanged.

Run `solution regions.csv records.csv output.txt`. Defaults: `/data/regions.csv`, `/data/records.csv`, `/out/aggregates.txt`.
