# Team Lex: Track D

Current candidate compacts construction out-degrees into bytes with an exact overflow list, stores search-label page offsets in 32 bits, and uses a smaller near-distance bucket ring. The CSR engine, persistent phase pool, AVX2 parser, exact distant heap and conservative u64 retry remain. Exhausted narrow searches return unreachable directly when no overflow occurred.

Previous official best: **74.595852 points / K=31.039666**, T=0.100483 s, M=59,170,816 bytes, S=617 (`406a96b`, `run-24e9596420e9`, packed-map challenger). The persistent-pool baseline reached 74.589385 / K=31.030423 (`d67777d`). The new compact candidate is awaiting evaluation; local results do not establish its official score.

In 41 interleaved rounds on each of two full datasets, the candidate's median paired wall-time ratio was 0.9820 / 0.9803 versus the baseline. Sampled local anonymous peak memory was 53,336 KiB versus 56,100 KiB. These are local proxies, not official resource measurements. See PERF.md for validation and earlier experiments.

The pinned native twelve-worker seed-17 PGO build and organizer generator bytes are unchanged. Zero external runtime dependencies. Environment: SD_DATA, SD_OUT and SD_THREADS; default output /out/distances.txt.
