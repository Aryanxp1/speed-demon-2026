# Track B: Index

The task, the input files, and the exact output format are defined in the [Track B Specification](https://github.com/hackathon-raptors/speed-demon-2026/blob/main/specs/b-index.md).

## Approach

Our solution is a zero-runtime-dependency Rust engine ($D = 1.000$) designed around four key architectural principles:

1. **Ahead-of-Time (AOT) Query Vocabulary Filtering:**
   Because all 50,000 queries are known at startup, we parse `queries.txt` before ingesting the corpus. Across the 250,000 documents and 36 million tokens, only 75,747 distinct terms are queried (out of 609,833 total vocabulary terms). By matching tokens against an internal fast FNV-1a query term table during ingest, over 87% of corpus vocabulary is ignored, avoiding redundant memory allocations and drastically compressing index construction.

2. **Parallel Ingest with Sorted Monotonic Postings:**
   The 254 MB `docs.txt` is memory-buffered and divided along newline boundaries across CPU worker threads. Each worker thread parses documents and emits strictly ascending document IDs for observed query terms. Because thread boundaries partition document ID ranges ($0..N-1$), merging thread postings is a linear concatenation that produces strictly ascending posting lists with zero sorting overhead. Thread-local intermediate postings are explicitly dropped (`drop(thread_results)`) immediately after concatenation, reclaiming ~89 MB before query execution.

3. **High-Throughput Left-to-Right Boolean & Phrase Query Processing:**
   - **Boolean Queries:** Evaluated strictly left-to-right according to specification grammar. For terms with document frequency $df < 256$, bit-set and bit-clear operations are performed directly in $O(df)$ time, bypassing full 3,907-word bitset iterations. Only when large sets intersect are dense 64-bit word bitsets used.
   - **Phrase Queries:** For each phrase query $t_1 \dots t_k$, terms are sorted by document frequency ascending. Candidate documents are filtered by intersecting against the rarest anchor term first, quickly narrowing candidates down to a tiny set (median 1, 99th percentile 5).
   - **Safe In-Place Verification:** Each candidate document is verified in-place using stack-allocated `(u32, u32)` token spans for documents up to 1,024 tokens, with automatic dynamic heap fallback for unusually long holdout documents. Pre-computed 256-byte static lookup tables (`IS_TOKEN_CHAR` and `LOWER_ASCII`) eliminate branch mispredictions during tokenization.
   - **Parallel Query Execution:** All 50,000 queries are evaluated in parallel across worker threads, formatting results directly into pre-allocated memory buffers.

## Performance Metrics (Official Board Seed 1 Full)

- **Reference Wall Time ($T_{\text{ref}}$):** 56.35 s
- **Official Measured Wall Time ($T$):** 1.071 s (52.6x speedup)
- **Reference Peak Memory ($M_{\text{ref}}$):** 2,200 MB
- **Official Measured Peak Memory ($M$):** 534 MB (4.12x leaner)
- **Official Machine Score ($K$):** 7.333291 (43.265 points)
- **Correctness:** 100% byte-identical SHA-256 match (`f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`).

## Dependencies

None. Standard library only (`dependency_count = 0`, $D = 1.000$).

## With another 24 hours

1. **SIMD Tokenization:** Use AVX2 vector instructions (`_mm256_cmpeq_epi8`) to vectorize ASCII token boundary detection across 32-byte chunks of `docs.txt`.
2. **Direct Memory Mapping (`mmap`):** Map `docs.txt` via zero-copy OS paging to reduce initial file read overhead.
3. **Pipelined Ingest and Query Execution:** Begin evaluating boolean queries containing only completed document ranges while corpus ingest runs concurrently.
