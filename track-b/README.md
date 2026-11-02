# Track B: Index — Team Lex

Latest leaderboard score: **79.073 points, K=38.146211**, scored commit `534e399`; T=0.087 s, M=42 MB, S=1148. These are the displayed values in the latest 2 November 2026 screenshot; the run ID and unrounded measurements are not shown. The final-minute SLVS trial passed correctness but scored 78.653855, so all three changes were reverted.

Rust 2021, standard library only. The existing phased index uses eight-token AVX2 action compaction, sparse postings and transposed bitmaps. Balanced quarter-MiB sampling across twelve 1 MiB claims uses contiguous atomic counters and term-owned reduction. Density-dependent phases and smaller claims preserve u16 offsets. Boolean operators evaluate left to right and phrases anchor on their rarest word. Dynamic dictionary sizing, atomic publication, checked mappings and wrapping prefetch remain.

The restored actual offline Cargo release build passes all 27 reference datasets again at 1/4/12 CPU affinities. The identical source previously passed all 27 ASAN cases at 1/12 affinities. Full output SHA-256: `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`. Source: 1139 Rust plus nine Dockerfile lines. Hash matching retains the previously documented rare-collision limitation.

The unchanged pinned O3/fat-LTO/static x86-64-v3/AES/VAES build has zero external crates. Workers are capped at twelve. Use `--data`/`--out`, `DATA_DIR`/`OUT_DIR`, or `SD_DATA`/`SD_OUT`; defaults are `/data` and `/out`. PERF.md records source provenance, all trials and official results.
