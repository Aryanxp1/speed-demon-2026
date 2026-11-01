# Team Lex: Track D

The retained implementation adapts the user-provided `C:/Users/aryan/Downloads/sd-td-main (1)/td/track-d` CSR engine. It retains Lex's persistent C++20 worker pool, AVX2 delimiter scanner, global transparent-huge-page disabling and checked mappings. No external dependencies. The pinned seed-17 PGO Docker build and organizer generator bytes are unchanged.

Edges are parsed into six-byte bins and assembled into compressed adjacency arrays. Degree-two bypassing, BFS ordering and edge gathering are fused. The reverse graph uses the same bucket builder. Queries use paged 32-bit distance labels with a conservative overflow flag and exact 64-bit retry; full sums remain 64-bit. The 256-settle forward probe and bidirectional lower-bound stopping rule remain exact for zero weights, parallel edges and unreachable vertices.

Local randomized interleaved full-data PGO comparison: previous packed-row solver 73.773 ms, adapted CSR solver 75.150 ms (29 runs each). The CSR engine is slightly slower locally but has substantially fewer formatted source lines. The official combined-K result below confirms the improvement. A compact-fragment allocator alternative reduced page-table memory but was slower and added source, so it is excluded.

Optimized, ASan/UBSan and ThreadSanitizer builds pass public tiny/sample/full at 1/4/12 workers, 70 existing differential fixtures, 88 supplied differential cases, 60 narrow/wide boundary cases, 60 lattice/contraction stress cases, the shuffled 70,004-vertex cycle, empty/single-query cases and large exact u64 distances. Tests use the actual fixed probe of 256, not ignored environment variations.

Official best: **#1 Track D by K**, **74.373046 points**, **K=30.722809**, T=0.100884 s, M=60,424,192 bytes, S=634, D=1. All gates passed: `run-48d4d8968533`, implementation `f3c7439`. A complete live-board read at 02:07:32 IST on November 2, 2026 placed Lex first; second was SweetDreamsHalloween at K=30.442281 / 74.173859 points. The preceding implementation's best was 73.794121 / K=29.914547 (`dd1051b`); its earlier measured result was 73.774168 / K=29.887072. See PERF.md for evidence and provenance.

Defaults: up to twelve workers, `/data` input and `/out/distances.txt` output. Environment: `SD_THREADS` (clamped 1-64), `SD_DATA`, `SD_OUT`.
