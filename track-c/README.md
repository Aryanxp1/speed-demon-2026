# Track C: Join — Team Lex

Verified standing best: **82.441739 points, K=44.548675**, run `run-bba010f3ad51`; T=0.022911 s, M=9,125,888 bytes, S=326. The uniform-overflow layout is awaiting official evaluation. PERF.md records source provenance, local checks and native results.

Rust 2021, standard library only, unchanged pinned offline build. The existing parser uses AVX2 structural masks, batched numeric decoding, tree workers, a region join and per-worker statistics. Shared atomic name buckets assign IDs and bit rows count distinct names per country. Names retain the existing probabilistic fingerprints; rare collisions remain possible.

The candidate gives every country the same compact bit-row head and cold overflow tail, preserving all 256 groups and 131,072 IDs. The first 241 groups' hot heads and name buckets still fit inside two huge pages. This simplifies our capacity-preserving adaptation of the supplied 5pm layout without copying its hard limits. CSV handling, EOF padding and the record loop remain unchanged.

All 12 reference datasets pass in the actual Cargo executable at 1/4/12 CPU affinities and under AddressSanitizer at 1/12. These include full/other seeds, hostile CSV, 110,000 names spread over 255 countries, and unterminated/page-aligned records. Candidate canonical source: 312 Rust plus seven Dockerfile lines. No native improvement is claimed until measured.

Run `solution regions.csv records.csv output.txt`. Defaults: `/data/regions.csv`, `/data/records.csv`, `/out/aggregates.txt`.
