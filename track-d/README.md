# Team Lex: Track D

Current candidate combines a packed final-vertex map, unique atomic discovery claims, byte out-degrees with exact overflow counts, 32-bit search-label page offsets and 64 buckets of width 32. Standard phase threads are joined before referenced data is released. Exact distant heaps, u64 distance retries, CSR adjacency, AVX2 parsing and allocation guards remain.

Official best so far: **74.595852 points / K=31.039666**, T=0.100483 s, M=59,170,816 bytes, S=617 (`406a96b`, `run-24e9596420e9`). The previous compact persistent-pool trial scored 74.424499 / K=30.795694 (`075d783`, `run-01fab7cd4dfd`). The new combined candidate awaits official scoring.

Across 51 randomized interleaved rounds on each of two full datasets, the combined candidate's median paired wall-time ratios versus the original CSR baseline were 0.9778 and 0.9614. Sampled local anonymous peak memory was 51,664 KiB versus 56,420 KiB. These local proxies do not establish official points. See PERF.md for measurements and validation.

The pinned native twelve-worker seed-17 PGO build and organizer generator bytes are unchanged. Zero external runtime dependencies. Environment: SD_DATA, SD_OUT and SD_THREADS; default output /out/distances.txt.
