# Track A: Ingest

Zero-dependency C++20 implementation. Reads `/data/requests.log`, writes `/out/summary.txt`. Adapted from the user-supplied sd-ta snapshot; local validation is in [CANDIDATE.md](CANDIDATE.md) and the optimization history is in [PERF.md](PERF.md).

## Approach

The mapped log is split into 768 KiB line-aligned blocks with bounded lookahead of one block per worker. Each worker parses four interleaved streams using SIMD separator scans and integer latency decoding, filing six-byte records by hour. Private endpoint tables mirror an append-only shared dictionary; every match compares endpoint bytes exactly.

Once the completed input prefix advances sufficiently, workers finalize an hour with counting scatter and exact nearest-rank p50/p95/p99 selection. AVX2 partitions find multiple ranks together, with bitonic sorting for small groups. Completed hours are formatted and written in order. If any record arrives after its hour was finalized, the program truncates the output and reruns without early finalization, preserving correctness for arbitrary input ordering.

Transparent huge pages are disabled and workers are capped at twelve. Input pages are released during the final work; output writeback and cache advice are best effort. The process exits after all workers join and writes complete.

## Build and local overrides

The pinned Dockerfile builds a static binary offline. No external libraries are used. `SD_DATA_DIR`, `SD_OUT_DIR`, and `SD_THREADS` override the default directories and worker count.

## Further work

Measure official wall time, cgroup memory and formatted source size together before judging the score. Laptop timing improvements alone cannot establish a full-K gain.
