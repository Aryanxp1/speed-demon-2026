# Track B: Index — Lex

Dependency-free Rust inverted index for Boolean and phrase queries. The executable targets Linux x86-64 with AVX2.

## Approach

Queries establish the vocabulary before corpus ingestion. The engine maps the input, tokenizes with SIMD, and partitions ingestion at document boundaries. Frequent terms use dense bitmaps; sparse terms use sorted postings.

Sparse postings store 16-bit offsets relative to each worker's first document. If any partition exceeds 65,536 documents, construction selects full 32-bit IDs. Packed temporary records and early bucket release reduce peak memory. Queries reuse scratch buffers to decode sparse segments.

Boolean queries preserve left-to-right semantics with hybrid list/bitmap operations and AVX2 bitmap combinations. Phrase queries intersect rare terms first, then verify candidates against mapped document text. Twelve workers claim batches of 64 queries; results retain input order.

## Validation and performance

Tiny, sample, full and extra seeds 42/999 match reference outputs. Full SHA-256: `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`.

The current local full-run trimmed mean is .4067 s, with maximum RSS 348.96 MB, versus .4333 s / 448.45 MB at `22347ff`. These WSL measurements include mapped pages and do not predict official scores. See [PERF.md](PERF.md) for raw runs, profiling, build checks and limitations. No crates were added.
