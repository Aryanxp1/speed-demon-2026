# Track B: Index - Team Lex

The active implementation returns to our Rust engine from `4a879b9`. Selected scan ideas from the user-supplied `Downloads/trackb/trackb` snapshot are adapted into that engine; the supplied file is not substituted wholesale. Its reported 75.365 score is not a result for this submission.

Changes: bootstrap the eight-token lookup pipeline to remove its startup branch, sample 1/64 of each share, prefetch input, use carry-based token-end masks, check one non-anchor phrase term before the full exact comparison, and release finished interior file pages with MADV_DONTNEED. Full 32-byte term equality and the original guarded worker pool remain. The dictionary experiments that lost locally were rejected.

Local full-data medians: old Rust 128.526 ms, selected version 124.608 ms (17 interleaved rounds). Outputs match on 23 datasets under 1, 4 and 12 CPU affinities, and under AddressSanitizer. Full output SHA-256: `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`.

Rust 2021, no external crates. CLI: `--data` and `--out`; environment: `DATA_DIR` and `OUT_DIR`; defaults `/data` and `/out`. Twelve persistent workers. The pinned Rust Docker build uses native optimization, LTO and an offline Cargo build. Official score for this restored Rust submission is pending; historical best C++ result was 73.996964, and old Rust was 73.103392. See PERF.md for evidence and experiment history.
