# Team Lex: Track D

Current candidate adapts the user-provided `C:/Users/aryan/Downloads/sd-td-main (1)/td/track-d` CSR engine. It retains Lex's persistent C++20 worker pool, AVX2 delimiter scanner, global transparent-huge-page disabling and checked mappings. No external dependencies. The pinned seed-17 PGO Docker build and organizer generator bytes are unchanged.

Edges are parsed into six-byte bins and assembled into compressed adjacency arrays. Degree-two bypassing, BFS ordering and edge gathering are fused. The reverse graph uses the same bucket builder. Queries use paged 32-bit distance labels with a conservative overflow flag and exact 64-bit retry; full sums remain 64-bit. The 256-settle forward probe and bidirectional lower-bound stopping rule remain exact for zero weights, parallel edges and unreachable vertices.

Local randomized interleaved full-data PGO comparison: previous packed-row solver 73.773 ms, adapted CSR solver 75.150 ms (29 runs each). The candidate is slightly slower locally, but has substantially fewer formatted source lines. Source size and kernel page-table costs affect K; no official gain is claimed before evaluation. A compact-fragment allocator alternative reduced page-table memory but was slower and added source, so it is excluded.

Optimized, ASan/UBSan and ThreadSanitizer builds pass public tiny/sample/full at 1/4/12 workers, 70 existing differential fixtures, 88 supplied differential cases, 60 narrow/wide boundary cases, 60 lattice/contraction stress cases, the shuffled 70,004-vertex cycle, empty/single-query cases and large exact u64 distances. Tests use the actual fixed probe of 256, not ignored environment variations.

Previous retained official best: **73.774168 points**, **K=29.887072**, T=0.097266 s, M=62,144,512 bytes, S=829 (`run-d1a841f662c3`, scored commit `216d284`, implementation `51ceb9b`). New official result pending. See PERF.md for provenance, verification and measurements.

Defaults: up to twelve workers, `/data` input and `/out/distances.txt` output. Environment: `SD_THREADS` (clamped 1-64), `SD_DATA`, `SD_OUT`.
