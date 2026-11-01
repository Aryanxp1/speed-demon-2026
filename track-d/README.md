# Team Lex: Track D

Retained source is restored byte-for-byte to the first-place CSR implementation `f3c7439`. Its latest best is **74.589385 points / K=31.030423**, T=0.099307 s, M=59,797,504 bytes, S=634 (`d67777d`, `run-dd36ed5a58fd`; source unchanged from f3c7439).

The packed-map challenger 4dabb25 passed every gate but regressed on the scoring machine: 72.965705 / K=28.794803, T=0.117368 s. It is not retained. Further experiments remain outside the submission until validated.

The retained engine combines CSR adjacency, six-byte parse bins, degree-two contraction/BFS ordering, an exact bidirectional bucket search and conservative u32-to-u64 retries. Lex's persistent worker pool, AVX2 scanner and checked mappings remain. The pinned native twelve-worker seed-17 PGO build and organizer generator bytes are unchanged. See PERF.md for verification and experiment history.

Environment: SD_DATA, SD_OUT and SD_THREADS; output defaults to /out/distances.txt. Zero external dependencies.
