# Track D: Traverse

High-performance shortest path engine for directed weighted graphs on road network topology.

## Approach

1. **Compact Cache-Interleaved CSR Graph**:
   - `offsets`: `Vec<u32>` of size $N + 1$ (starts for each node's forward edge list).
   - `edges`: `Vec<EdgeTarget>` of size $M$ where `struct EdgeTarget { to: u32, weight: u32 }` (8 bytes per edge). Interleaving destination vertex and weight into a single contiguous array eliminates dual cache misses during edge relaxation.
   - Total graph memory for 1.74M nodes and 5M edges: ~47 MB.
   - Self-loops ($u == v$) filtered out during load.

2. **$O(1)$ Reachability & Bounds Filtering**:
   - Out-of-bounds node query IDs (`sq >= N || tq >= N`) safely return `-1`.
   - `s == t` answered immediately with `0`.
   - `out_degree[s] == 0` or `in_degree[t] == 0` answered immediately with `-1`.

3. **Epoch-Based Distance Reset**:
   - Each worker thread maintains a flat `visited_epoch: Vec<u32>` and `dist: Vec<u64>` of length $N$.
   - Advancing `cur_epoch += 1` resets distance queries in $O(1)$ time without allocating or looping over 1.74M entries.
   - Epoch rollover protection: when `cur_epoch == u32::MAX`, arrays are cleanly cleared and reset.

4. **Dynamic Multi-Threaded Work Scheduling**:
   - Rather than static query chunking, queries are dynamically distributed across 6 worker threads using an `AtomicUsize` work counter.
   - Threads dynamically claim the next query index with `fetch_add(1, Relaxed)`, eliminating thread idle time caused by uneven Dijkstra search depths.
   - Zero external crates ($D = 1.000$).

5. **Direct ASCII Output Serialization**:
   - Custom integer serialization to a buffered `BufWriter` with no heap string allocations.

## Performance Metrics (Official Board Seed 1 Full)

- **Reference Wall Time ($T_{\text{ref}}$):** 72.69 s
- **Official Measured Wall Time ($T$):** 1.512 s (48.1x speedup)
- **Reference Peak Memory ($M_{\text{ref}}$):** 915.0 MB
- **Official Measured Peak Memory ($M$):** 177.0 MB (5.17x leaner)
- **Official Machine Score ($K$):** 7.430230 (43.550 points)
- **Correctness:** 100% byte-identical SHA-256 match (`a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e`).

## Dependencies

- Pure Rust standard library (`std::thread`, `std::sync::atomic`, `std::collections::BinaryHeap`, `std::fs`, `std::io`).
- External runtime dependencies: **0** ($D = 1.000$).

## With another 24 hours

1. **SIMD Graph Parser:** Accelerate space and newline parsing using AVX2 vector instructions during initial CSR ingestion.
2. **ALT (A*, Landmarks, Triangle Inequality) or CH (Contraction Hierarchies):** Precompute landmarks to accelerate long-distance goal-directed Dijkstra queries.
