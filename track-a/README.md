# Track A: Ingest — Team Lex

Rust 2021, standard library only. Combines the existing Team Lex Rust input/output support and streaming guards with the fast endpoint lookup from the user-provided `Downloads/thorta/thorta/track-a` source. Unnecessary source comments are removed; provenance and measurements remain in PERF.md.

Workers parse 1 MiB claims in 64 KiB SIMD batches, store compact four-byte rows with an escape for large latencies, and summarize completed hours using radix sorting. Hour storage is reused for output and released after writing. Endpoint identities use a hash fingerprint and seven explicit length bits; the original name bytes are retained for output. Active-claim guards prevent reclaiming an hour while a parser writes it. Late records trigger a complete rerun with early sealing disabled.

The supplied hash merged some different repeated-prefix names in the endpoint-length fixture. The integration retains length explicitly and uses different constant keys across hash lanes. Matching remains probabilistic: distinct names of the same length can collide in the fingerprint. Full-name checks from the previous merge are removed to follow the user's priority on points and K, so universal exact matching is not claimed.

The pinned offline Docker build links statically with AVX2/AES/VAES support. Workers are capped at twelve. Use --data and --out, or DATA_DIR/OUT_DIR; SD_DATA_DIR, SD_OUT_DIR and SD_THREADS are also supported. Defaults are /data and /out.

Reference checks cover 17 datasets at 1/4/12 workers, and the same 17 pass AddressSanitizer at 1/12 workers. This includes reordered input, endpoint lengths 1..125, uint32 boundaries, and 1,677 names with single-byte variations and block permutations. These tests do not remove the fingerprint-collision limitation. The supplied source's reported 86.174 points is historical evidence, not a score claimed for this integration. Official results are recorded in PERF.md.

Verified official best for the retained f7e6bf8 source: **85.767155 points, K=51.921007**, run `run-d0163e91b459`; all gates passed. The first evaluation was 85.377705 points, K=50.998113. The selected implementation seals and claims each hour with its writer guard and aborts a speculative claim on late records, retaining the complete fallback without a skip check on every record. A zero-error/stack-buffer trial scored lower and has been reverted; detailed profiling and rejected experiments are in PERF.md.
