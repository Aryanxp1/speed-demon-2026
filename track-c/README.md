# Track C: Join

High-performance relational join and aggregation engine for hostile CSV records.

## Approach

1. **Direct $O(1)$ Region Lookup**:
   - Flat 10 MB lookup array `region_to_country: Vec<u8>` mapping dense `region_id` values directly to country IDs ($0 \dots 240$).
   - Completely eliminates hash map overhead and collision handling.
   - `_UNMATCHED` assigned index 0.

2. **Sliding-Window Streaming Parser**:
   - Streams 202 MB `records.csv` through a 2 MB sliding-window chunk buffer.
   - Preserves low memory footprint ($45.5\text{ MB}$ peak RSS locally, $88.0\text{ MB}$ on official scoring box vs $367\text{ MB}$ reference).
   - Handles mid-file UTF-8 BOM (`\xEF\xBB\xBF`), escaped quotes (`""`), quoted commas, and embedded newlines (`\n`) in multiline notes.

3. **Collision-Proof Exact-Byte Distinct Name Table**:
   - Replaced dual polynomial 64-bit hashing with `DistinctNameTable`, an open-addressing hash table storing `(country_id, offset, len)` records backed by a contiguous `Vec<u8>` string arena.
   - On hash lookup, directly verifies raw slice bytes (`existing == name`), eliminating theoretical birthday-paradox collision risks on unseen holdout distributions while retaining a lean memory profile (~90 MB) and sub-second speed.

4. **Exact Integer Financial Math**:
   - Parses dollar amounts directly into exact integer cents (`i64`), avoiding floating-point inaccuracies.

5. **Direct ASCII Output Serialization**:
   - Zero-allocation decimal and money formatters writing into a buffered `BufWriter`.

## Performance Metrics (Official Board Seed 1 Full)

- **Reference Wall Time ($T_{\text{ref}}$):** 14.44 s
- **Official Measured Wall Time ($T$):** 0.926 s (15.6x speedup)
- **Reference Peak Memory ($M_{\text{ref}}$):** 367.0 MB
- **Official Measured Peak Memory ($M$):** 88.0 MB (4.17x leaner)
- **Official Machine Score ($K$):** 4.378513 (32.066 points)
- **Correctness:** 100% byte-identical SHA-256 match (`d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536`).

## Dependencies

- Pure Rust standard library.
- External runtime dependencies: **0** ($D = 1.000$).

## With another 24 hours

1. **Multi-Threaded Chunk Parsing:** Speculative CSV record boundary detection to split large CSV streams across multiple cores.
2. **SIMD Quote & Delimiter Scanning:** Use AVX2 vector instructions to scan for commas, double quotes, and newlines 32 bytes at a time.
