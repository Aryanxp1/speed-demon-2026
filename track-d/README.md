# Team Lex: Track D

Official best: **76.514177 points / K=33.906546**, run `run-f0ffbfc40e9a`, scored commit `4588426`. T=0.083196 s, M=56,451,072 bytes, S=684, D=1. All five official gates passed. Its source/build match `4b37425`; the later small gain is a same-source measurement.

The complete 148-team board generated 2026-11-02 12:19:35 UTC ranks Lex **third** in Track D. The leader has 76.681057 points / K=34.168125, leaving 0.166880 points. This improves the earlier fifth place and exceeds the supplied project's reported 75.839.

C++20, zero runtime dependencies. Four-slot raw rows feed fused contraction/locality ordering, equal final-ID work slices, forward-edge remapping/reverse counting and mapped-page recycling. Unique atomic claims, guarded direct SWAR scanning, exact overflow paths, checked mappings and process-wide huge-page disabling remain.

Queries use saturated 16-bit labels below 65535, a 256-settle forward probe, bidirectional search and a dynamically packed 64-bucket queue with an exact distant heap. Saturated/uncertain results rerun with full 64-bit labels. General edge decoding complements the common 11-bit specialization.

Current candidate batches temporary-bin release in groups of 128 buckets. A proven bound reserves every worker's maximum possible edge count plus one partial chunk per bucket, aligned to whole pages. This reduces sparse virtual-address coverage and release calls while preserving exact construction. The original -O3 native PGO recipe is retained. The complete native, ASan/UBSan and TSan suites pass, including maximum-size sparse/skewed/empty input cases. Official scoring is pending; the best above remains authoritative. PERF.md preserves all measurements and rejected trials.

Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS`; output: `distances.txt`.
