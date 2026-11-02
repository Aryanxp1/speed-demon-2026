# Team Lex: Track D

Official best: **76.486585 points / K=33.863489**, run `run-b8a48823a5d3`, commit `4b37425`. T=0.083369 s, M=56,516,608 bytes, S=684, D=1. All five official gates passed.

The complete 147-team board generated at 2026-11-02 10:25:36 UTC ranks Lex **third** in Track D. The leader has 76.681057 points / K=34.168125. This improves the earlier fifth place and exceeds the supplied project's reported 75.839.

C++20, zero runtime dependencies. Four-slot raw rows feed fused contraction/locality ordering, equal final-ID work slices, forward-edge remapping/reverse counting and mapped-page recycling. Team Lex retains unique atomic vertex claims, guarded direct SWAR scanning, checked mappings and process-wide huge-page disabling. Twelve persistent workers reuse one barrier for start/completion; callback lifetime and small-query guards remain.

Queries use 16-bit saturated labels for paths below 65535, an exact 256-settle forward probe, bidirectional search and a dynamically packed 64-bucket queue with exact distant heap. Saturated or uncertain results rerun with full 64-bit labels. General edge decoding complements the common 11-bit specialization. Label pages use pointer directories.

The pinned offline GCC15 build uses native twelve-worker seed-17 PGO. Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS`; output: `distances.txt`. The current winner combines packed discovery/degree records, specialized two-edge bypass walks, branchless walk compaction, the typed persistent dispatcher and 1.5x queue growth.

Native PGO, ASan/UBSan and TSan pass public inputs, independent oracle cases, saturation/u32/u64 boundaries and concurrency stress. PERF.md records provenance, complete results, rejected experiments and the remaining gap to the leader. Earlier implementations and all submitted trials remain in Git history.

The combined in-place bitmap / per-region offset trial passed gates but scored74.852264, so the exact4b37425 source is restored. Equal-slice mapping remains.

Pending trial preserves balanced mapping, reuses endpoint bits for bypass flags, marks fresh mmap storage as non-aliasing, and uses immutable restricted query-array pointers with GCC interprocedural pointer analysis. Exact fallbacks, 12 workers and zero runtime dependencies remain. Complete native and sanitizer suites and static PGO packaging pass; official score is pending.
