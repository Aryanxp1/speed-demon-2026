# Track D: Traverse

High-performance shortest path engine for directed weighted graphs on road network topology.

## Approach

1. **Compact CSR Graph**:
   - `offsets`: `Vec<u32>` of size $N + 1$ (starts for each node's forward edge list).
   - `edges_to`: `Vec<u32>` of size $M$ (target node IDs).
   - `edges_weight`: `Vec<u32>` of size $M$ (uint32 edge weights).
   - Total graph memory for 1.74M nodes and 5M edges: ~47 MB.
   - Self-loops ($u == v$) filtered out during load.

2. **$O(1)$ Reachability Filtering**:
   - `s == t` answered immediately with `0`.
   - `out_degree[s] == 0` or `in_degree[t] == 0` answered immediately with `-1`.

3. **Epoch-Based Distance Reset**:
   - Each thread maintains a flat `visited_epoch: Vec<u32>` and `dist: Vec<u64>` of length $N$.
   - Advancing `cur_epoch += 1` resets distance queries in $O(1)$ time without allocating or looping over 1.74M entries.

4. **Multi-Threaded Query Processing**:
   - 2,000 queries partitioned across 6 worker threads using `std::thread::scope`.
   - Zero external crates ($D = 1.000$).

5. **Direct ASCII Output Serialization**:
   - Custom integer serialization to a buffered `BufWriter` with no heap string allocations.

## Dependencies

- Pure Rust standard library (`std::thread`, `std::collections::BinaryHeap`, `std::fs`, `std::io`).
- External runtime dependencies: **0**.
