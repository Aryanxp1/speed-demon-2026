# Track A: Ingest — Team Lex

Rust 2021, standard library only. Combines the earlier Team Lex Rust input/output and exact-name behavior with the user-provided Thor streaming engine. Unnecessary source comments are removed; provenance and measurements remain in PERF.md.

Workers parse 1 MiB claims in 64 KiB SIMD batches, store compact four-byte rows with an escape for large latencies, and summarize completed hours using radix sorting. Hour storage is reused for output and released after writing. Endpoint hashes select candidates; endpoint bytes must match exactly. Active-claim guards prevent reclaiming an hour while a parser writes it. Late records trigger an exact rerun with early sealing disabled.

The pinned offline Docker build links statically with AVX2/AES/VAES support. Workers are capped at twelve. Use --data and --out, or DATA_DIR/OUT_DIR; SD_DATA_DIR, SD_OUT_DIR and SD_THREADS are also supported. Defaults are /data and /out.

Reference and AddressSanitizer checks cover 16 datasets, including reordered input, endpoint lengths and uint32 boundaries. Forced endpoint-hash collisions also pass. The Thor folder's reported 86.17 points is historical evidence for its source, not a result claimed for this merge. Official results are recorded in PERF.md.
