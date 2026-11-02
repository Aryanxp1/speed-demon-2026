# Track C: Join — Team Lex

Verified standing best: **82.374132 points, K=44.410192**, run `run-012b0c9fd050`; T=0.022297 s, M=10,858,496 bytes, S=289. The compact-table candidate awaits official measurement. PERF.md records provenance, experiments and official results.

Rust 2021, standard library only, with the unchanged pinned offline build. The parser uses AVX2 structural masks and batched numeric decoding. Workers parse regions, synchronize, then join records in 1 MiB claims with per-worker statistics. Shared atomic name buckets assign dense IDs; atomic bit rows count distinct names per country. Names use probabilistic fingerprints, so rare hash collisions remain possible.

The candidate keeps hot buckets and bit rows in two huge pages and retains cold space for the existing 256 groups and 131,072 name IDs. It also removes redundant digit conversion, separates the common eight-delimiter extraction, and supports a missing final newline using private mapping padding. The original join, CSV handling and worker structure remain. Useful layout ideas came from user-supplied code; the overflow layout retains our base's capacity.

All 11 reference datasets pass at 1/4/12 CPU affinities, and all 11 pass AddressSanitizer at 1/12 affinities. Tests include full and other seeds, quoted CSV and Unicode, 110,000 names, 255 country groups plus unmatched, and unterminated/page-aligned final records. Source: 319 nonblank Rust plus seven Dockerfile lines. Local memory results are diagnostic; official candidate metrics remain pending.

Run `solution regions.csv records.csv output.txt`. Defaults: `/data/regions.csv`, `/data/records.csv`, `/out/aggregates.txt`.
