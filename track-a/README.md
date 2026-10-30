# Track A: Ingest

The task, the input files, and the exact output format are defined in the [Track A Specification](https://github.com/hackathon-raptors/speed-demon-2026/blob/main/specs/a-ingest.md).

## Approach

Our solution is a zero-dependency, parallel log aggregation engine written in Rust:

1. **Deterministic Hour Arithmetic:** Rather than parsing RFC 3339 timestamps with a general-purpose date library, the parser exploits the invariant 72-hour competition window (`2026-10-30T18:00:00Z` to `2026-11-02T17:59:59Z`). Day and hour bytes are mapped directly to an integer slot in `0..71` using branchless byte arithmetic, correctly accounting for month rollover.
2. **Dynamic Endpoint Interning:** To generalize safely to unknown holdout seeds with distinct endpoint catalogues, endpoints are dynamically interned into a global registry. A thread-local cache layer filters lookups so that after the first few thousand records, 100% of endpoint lookups complete in $O(1)$ without locking or atomics.
3. **Multi-Core Work Partitioning:** The file is partitioned into 6 disjoint contiguous chunks split along newline boundaries. Each thread processes its assigned chunk independently with a 1 MB streaming buffer, keeping peak memory under 61 MB (3.7x leaner than the reference).
4. **Nearest-Rank Percentiles:** Groups with small counts ($n \le 2$) resolve in 1–2 comparisons. Groups with $n > 2$ use `select_nth_unstable` to partition percentiles $k_{99}, k_{95}, k_{50}$ in $O(n)$ time, avoiding full $O(n \log n)$ quicksorts.
5. **Direct ASCII Output Serialization:** Formats the final 10.1 MB output directly into reusable memory buffers using fast integer-to-ASCII conversion, eliminating format string interpretation overhead.

## Performance Metrics (Official Board Seed 1 Full)

- **Reference Wall Time ($T_{\text{ref}}$):** 32.05 s
- **Official Measured Wall Time ($T$):** 0.292 s (109.8x speedup)
- **Reference Peak Memory ($M_{\text{ref}}$):** 228.0 MB
- **Official Measured Peak Memory ($M$):** 61.0 MB (3.74x leaner)
- **Official Machine Score ($K$):** 10.284873 (50.610 points)
- **Correctness:** 100% byte-identical SHA-256 match (`495ca09682542a981a8ba214a1a659fb221ee54fe66d8ba609fca940aa415ef4`).

## Dependencies

None ($D = 1.0$). Written entirely using the Rust standard library (`std`).

## With another 24 hours

1. **AVX2 SIMD Newline and Delimiter Scanning:** Utilize `_mm256_cmpeq_epi8` vector instructions to locate spaces and newlines across 32-byte vectors simultaneously.
2. **Lock-Free Flat HashTable:** Implement a lock-free open-addressing table using `AtomicU32` CAS for initial endpoint registration to eliminate the remaining mutex during the warmup phase.
3. **Profile-Guided Optimization (PGO):** Integrate with `halloumee/speed-demon-rust:1.99.1-trixie-llvm-tools` to run PGO profile collection on generated sample logs.
