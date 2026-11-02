# Team Lex: Track D

Exact directed shortest paths, C++20, zero runtime dependencies.

Official best: **75.960000 points / K=33.052172**, run `run-3fef8f69fbcc`, T=0.085550 s, M=59,293,696 bytes, S=692. The evaluated source is the integration committed at `70952dd`; the scored commit `c4a3855` changed only Track A. All five official gates passed.

Adapted from the supplied `Downloads/td/td/track-d` solver: four-slot raw rows, fused contraction/locality ordering, equal final-ID slices, fused edge remapping/reverse counting and mapped-page recycling. Team Lex retains unique atomic vertex claims, AVX2 scanning, checked mappings and process-wide huge-page disabling. A twelve-worker pool reuses one barrier for callback start/completion; small-query guards preserve callback lifetime and worker bounds.

Queries use 16-bit saturated labels for paths below 65535, an exact 256-settle forward probe, bidirectional search and a dynamically packed 64-bucket queue with exact far heap. Saturated or uncertain results rerun using full 64-bit labels. General edge decoding remains available alongside the common 11-bit specialization. Label pages use pointer directories.

The retained pinned offline GCC15 build uses native twelve-worker seed-17 PGO. Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS`; output: `distances.txt`. The generic C++17/no-PGO alternative scored 74.849320 and is rejected.

Native PGO, ASan/UBSan and TSan pass public inputs, independent oracles, saturation/u32/u64 boundaries and concurrency stress. See PERF.md for provenance, complete metrics and rejected experiments. The previous 75.016414 baseline remains in Git history.
