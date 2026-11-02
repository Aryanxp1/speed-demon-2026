# Team Lex: Track D

Official best: **75.960000 points / K=33.052172**, run `run-3fef8f69fbcc`. T=0.085550 s, M=59,293,696 bytes, S=692. All five official gates passed. The retained source/Dockerfile are exactly the integration from `70952dd`; the scored commit `c4a3855` changed only Track A.

The complete board snapshot at 2026-11-02 07:49:44 UTC ranks Lex **third** in Track D. This improves the earlier fifth place and exceeds the supplied project's reported 75.839. First place is not claimed.

C++20, zero runtime dependencies. Four-slot raw rows feed fused contraction/locality ordering, equal final-ID work slices, forward-edge remapping/reverse counting and mapped-page recycling. Team Lex retains unique atomic vertex claims, AVX2 scanning, checked mappings and process-wide huge-page disabling. Twelve persistent workers reuse one barrier for start/completion; callback lifetime and small-query guards remain.

Queries use 16-bit saturated labels for paths below 65535, an exact 256-settle forward probe, bidirectional search and a dynamically packed 64-bucket queue with exact distant heap. Saturated or uncertain results rerun with full 64-bit labels. General edge decoding complements the common 11-bit specialization. Label pages use pointer directories.

The pinned offline GCC15 build uses native twelve-worker seed-17 PGO. Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS`; output: `distances.txt`. The generic build (74.849320) and packed-record trial (75.936093) are not promoted.

Native PGO, ASan/UBSan and TSan pass public inputs, independent oracle cases, saturation/u32/u64 boundaries and concurrency stress. PERF.md records provenance, complete results, rejected experiments and the remaining gap to the leader. Earlier implementations and all submitted trials remain in Git history.
