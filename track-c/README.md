# Track C: Join

High-performance relational join and aggregation engine for hostile CSV records.

## Approach

1. **Direct $O(1)$ Region Lookup**:
   - Flat 10 MB lookup array `region_to_country: Vec<u8>` mapping dense `region_id` values directly to country IDs ($0 \dots 240$).
   - Completely eliminates hash map overhead and collision handling.
   - `_UNMATCHED` assigned index 0.

2. **Sliding-Window Streaming Parser**:
   - Streams 202 MB `records.csv` through a 2 MB sliding-window chunk buffer.
   - Preserves low memory footprint ($45.5\text{ MB}$ peak RSS vs $367\text{ MB}$ reference).
   - Handles mid-file UTF-8 BOM (`\xEF\xBB\xBF`), escaped quotes (`""`), quoted commas, and embedded newlines (`\n`) in multiline notes.

3. **64-bit FxHash Distinct Name Tracking**:
   - Tracks unique names via 64-bit FxHash in `HashSet<u64>` per country.
   - Avoids heap allocation of 100,000 distinct string objects.

4. **Exact Integer Financial Math**:
   - Parses dollar amounts directly into exact integer cents (`i64`), avoiding floating-point inaccuracies.

5. **Direct ASCII Output Serialization**:
   - Zero-allocation decimal and money formatters writing into a buffered `BufWriter`.

## Dependencies

- Pure Rust standard library.
- External runtime dependencies: **0** ($D = 1.000$).
