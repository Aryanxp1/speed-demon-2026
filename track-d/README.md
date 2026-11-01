# Team Lex: Track D

The retained engine combines a packed final-vertex map, unique atomic discovery claims, byte out-degrees with exact overflow counts, 32-bit search-label page offsets and 64 buckets of width 32. Standard phase threads are joined before referenced data is released. Exact distant heaps, u64 distance retries, CSR adjacency, AVX2 parsing and allocation guards remain.

Official best: **74.612010 points / K=31.062771**, T=0.101770 s, M=56,152,064 bytes, S=631 (`289997d`, `run-cb120299feab`). All gates passed. This improves the previous best 74.595852 / K=31.039666. The target of 75 points remains unmet. The compact persistent-pool trial scored 74.424499 / K=30.795694 (`075d783`).

Across 51 randomized interleaved rounds on each of two full datasets, the combined candidate's median paired wall-time ratios versus the original CSR baseline were 0.9778 and 0.9614. Sampled local anonymous peak memory was 51,664 KiB versus 56,420 KiB. These local proxies do not establish official points. See PERF.md for measurements and validation.

The pinned native twelve-worker seed-17 PGO build and organizer generator bytes are unchanged. Zero external runtime dependencies. Environment: SD_DATA, SD_OUT and SD_THREADS; default output /out/distances.txt.
