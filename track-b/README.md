# Track B: Index — Team Lex

Standing verified best: **78.471404 points, K=37.104627**, run `run-489677098d0a`, using the unchanged 1a718ef implementation. Early exit's first official sample scored 78.471095, K=37.104100; a second unchanged-source sample is pending. No 79+ result is claimed. Detailed evidence is in PERF.md.

Rust 2021, standard library only. The engine reads queries first and scans the corpus in phases, using sparse postings or transposed 64-document bitmaps. A cache-line bucket contains seven hash tags and eight actions. Phrases compare neighboring token hashes around their rarest word. Boolean operators remain strictly left to right.

The retained adaptation borrows the supplied trackalternate engine's eight-token AVX2 compaction idea. Per-worker action arrays, masked stable lane permutations and per-document boundaries replace scalar stores for every token. Phrase anchors are extracted from vector masks. The existing stored bucket shift, dynamic dictionary sizing, density-safe phase sizing, atomic publication and checked mappings are preserved.

Matching retains the previously authorized probabilistic fingerprints: rare collisions can confuse distinct words. Full-word collision verification is absent, so universal exact matching is not claimed.

The pinned offline O3/fat-LTO build links statically with x86-64-v3, AES and VAES support and uses zero external crates. Workers are capped at twelve. Use `--data` and `--out`, or `DATA_DIR`/`OUT_DIR`; `SD_DATA` and `SD_OUT` are also supported. Defaults are `/data` and `/out`.

The actual release executable passes all 26 reference datasets at 1/4/12 CPU affinities. AddressSanitizer passes all 26 at 1/12 affinities, including a dedicated SIMD-tail and document-boundary reference case. Full output SHA-256 remains `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`. Candidate canonical count is 1114 Rust plus nine Dockerfile lines. PERF.md records provenance, tests, timing and official evidence. Only Track B is changed by this pass.

Official measurements: T=0.094077 s, M=42,975,232 bytes, S=1122, D=1, K=36.716871, **78.243283 points**.
