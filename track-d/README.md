# Team Lex: Track D

Official best: **76.514177 points / K=33.906546**, run `run-f0ffbfc40e9a`, scored commit `4588426`. T=0.083196 s, M=56,451,072 bytes, S=684, D=1. All five official gates passed. Its source/build match `4b37425`; the later small gain is a same-source measurement.

The complete 148-team board generated 2026-11-02 12:19:35 UTC ranks Lex **third** in Track D. The leader has 76.681057 points / K=34.168125, leaving 0.166880 points. This improves the earlier fifth place and exceeds the supplied project's reported 75.839.

C++20, zero runtime dependencies. Four-slot raw rows feed fused contraction/locality ordering, equal final-ID work slices, forward-edge remapping/reverse counting and mapped-page recycling. Unique atomic claims, guarded direct SWAR scanning, exact overflow paths, checked mappings and process-wide huge-page disabling remain.

Queries use saturated 16-bit labels below 65535, a 256-settle forward probe, bidirectional search and a dynamically packed 64-bucket queue with an exact distant heap. Saturated/uncertain results rerun with full 64-bit labels. General edge decoding complements the common 11-bit specialization.

The exact source and build of the verified winner are restored. Joined-worker and training-only-cleanup experiments passed gates but scored lower. Native PGO, ASan/UBSan and TSan verify the retained implementation. PERF.md preserves full results and rejected trials.

Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS`; output: `distances.txt`.
