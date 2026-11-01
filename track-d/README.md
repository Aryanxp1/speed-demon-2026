# Team Lex: Track D

The retained engine combines a packed final-vertex map, unique atomic discovery claims, byte out-degrees with exact overflow counts, 32-bit search-label page offsets and 64 buckets of width 32. Standard phase threads are joined before referenced data is released. Exact distant heaps, u64 distance retries, CSR adjacency, AVX2 parsing and allocation guards remain.

Official best: **75.016414 points / K=31.646688**, T=0.097952 s, M=56,291,328 bytes, S=631 (`c9fb522`, `run-11935518a85b`). All gates passed. Lex is #1 on Track D as of 2026-11-01 22:16 UTC; second place K=30.528475, 74.235256 points. The 75-point target is achieved. This rerun uses the same algorithm as `289997d` (74.612010 / K=31.062771); intervening source edits corrected comments only. Runtime variability remains material.

Across 51 randomized interleaved rounds on each of two full datasets, the combined candidate's median paired wall-time ratios versus the original CSR baseline were 0.9778 and 0.9614. Sampled local anonymous peak memory was 51,664 KiB versus 56,420 KiB. These local proxies do not establish official points. See PERF.md for measurements and validation.

The pinned native twelve-worker seed-17 PGO build and organizer generator bytes are unchanged. Zero external runtime dependencies. Environment: SD_DATA, SD_OUT and SD_THREADS; default output /out/distances.txt.
