# Team Lex: Track D

Official best: **76.311963 points / K=33.592263**, run `run-22eacd56e441`, commit `443a0ac`. T=0.084272 s, M=56,107,008 bytes, S=703, D=1. All five official gates passed.

The complete 147-team board generated at 2026-11-02 10:25:36 UTC ranks Lex **third** in Track D. The leader has 76.681057 points / K=34.168125. This improves the earlier fifth place and exceeds the supplied project's reported 75.839.

C++20, zero runtime dependencies. Four-slot raw rows feed fused contraction/locality ordering, equal final-ID work slices, forward-edge remapping/reverse counting and mapped-page recycling. Team Lex retains unique atomic vertex claims, guarded direct SWAR scanning, checked mappings and process-wide huge-page disabling. Twelve persistent workers reuse one barrier for start/completion; callback lifetime and small-query guards remain.

Queries use 16-bit saturated labels for paths below 65535, an exact 256-settle forward probe, bidirectional search and a dynamically packed 64-bucket queue with exact distant heap. Saturated or uncertain results rerun with full 64-bit labels. General edge decoding complements the common 11-bit specialization. Label pages use pointer directories.

The pinned offline GCC15 build uses native twelve-worker seed-17 PGO. Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS`; output: `distances.txt`. The current winner combines packed discovery/degree records, specialized two-edge bypass walks, branchless walk compaction, the typed persistent dispatcher and 1.5x queue growth.

Native PGO, ASan/UBSan and TSan pass public inputs, independent oracle cases, saturation/u32/u64 boundaries and concurrency stress. PERF.md records provenance, complete results, rejected experiments and the remaining gap to the leader. Earlier implementations and all submitted trials remain in Git history.

The pending direct SWAR scanner removes the AVX2 separator state machine while preserving exact parsing. It passes the full native/ASan/UBSan/TSan suite and improves both full-input local medians. The official best above remains authoritative until this trial is scored.
