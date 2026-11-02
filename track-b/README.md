# Track B: Index — Team Lex

Verified standing best: **78.599644 points, K=37.324403**, run `run-0815c7a9fffa`, using the early-exit implementation. The new balanced-sampling candidate awaits official measurement; 79+ is not yet verified. See PERF.md for evidence and source provenance.

Rust 2021, standard library only. Queries are parsed first; the corpus is scanned in phases using sparse postings and transposed 64-document bitmaps. Eight-token AVX2 action compaction preserves per-document boundaries. Boolean operators evaluate left to right, and phrases anchor on their rarest word.

The candidate uses 1 MiB claims, twelve spread-out quarter-MiB sample jobs, and a term-owned reduction over contiguous atomic counters. Document-density checks shrink phases and claims as needed to preserve u16 offsets. Dynamic dictionary bounds, checked mappings, atomic publication and wrapping prefetch remain. The source combines the existing Rust engine with measured ideas from user-supplied versions; PERF.md records the adaptations.

Matching uses the previously authorized probabilistic hash fingerprints. Rare collisions can confuse distinct words; universal exact matching is not claimed.

The unchanged pinned offline O3/fat-LTO build links statically with x86-64-v3, AES and VAES and zero external crates. Workers are capped at twelve. Use `--data` and `--out`, `DATA_DIR`/`OUT_DIR`, or `SD_DATA`/`SD_OUT`; defaults are `/data` and `/out`.

The actual Cargo executable passes all 27 reference datasets at 1/4/12 CPU affinities and all 27 with AddressSanitizer at 1/12 affinities, including SIMD tails and the 250,000-document density fallback. Full output SHA-256: `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`. Candidate canonical source: 1139 Rust plus nine Dockerfile lines. All official candidate metrics remain pending.
