# Track D: Performance Log

Every step from the reference to our optimized version, with measured metrics, architectural rationale, and verified SHA-256 correctness.

| Step | Change | Run ID / Context | T (s) | M (MB) | S (lines) | K | Points |
|---|---|---|---|---|---|---|---|
| **0** | Official scoring box Python reference | `calibration-seed1.json` | 72.69 | 915.0 | 73 | 1.000 | 0.00 |
| **0-local** | Python reference (Unofficial local WSL run) | Local reference | 46.80 | 915.0 | 73 | 1.000 | 0.00 |
| **1** | Multi-threaded Forward Dijkstra in zero-dependency Rust with single-pass CSR, epoch-based distance tracking (zero array resets), and $O(1)$ in-degree/out-degree reachability pruning. | Unofficial local | 2.71 | 170.8 | 266 | 4.790 | 34.00 |
| **2** | Experiment: Naive 4-ary Min-Heap (`QuadHeap`). *Rejected*: Bubble-down comparisons increased user CPU time from 9.66s to 13.03s, real time 3.67s. Reverted to standard `BinaryHeap`. | Unofficial local | 3.67 | 170.8 | 280 | 4.120 | 30.70 |
| **3** | Experiment: Bidirectional Dijkstra with forward + backward CSR. *Evaluated*: User CPU time dropped from 10.19s to 7.33s (-28%), but graph setup and backward distance arrays increased peak RSS to 337 MB and real time to 3.42s. Retained forward Dijkstra as primary lean solution. | Unofficial local | 3.42 | 337.2 | 315 | 3.730 | 28.60 |
| **4** | Multi-threaded Forward Dijkstra with single-pass ASCII parsing and in-memory CSR scatter. | Unofficial local | 2.71 | 170.8 | 266 | 4.790 | 34.00 |
| **5** | **Official Board Submission (Baseline v1.0)**: Evaluated on official scoring machine (Intel Core i5-13500, pinned 12 threads, 8 GB RAM). 100% byte-identical SHA-256 match. | **Official Board Run** | **1.512** | **177.0** | **266** | **7.430230** | **43.550** |
| **6** | Dynamic query scheduling via `AtomicUsize` work counter; interleaved 8-byte `EdgeTarget { to: u32, weight: u32 }` cache layout; out-of-bounds query node safety checks; epoch counter rollover protection. Replaced static chunking to eliminate thread straggler stalls on unbalanced query paths. Measured 23% reduction in CPU user time (17.88s $\to$ 13.83s). | Unofficial local | 2.38 | 170.8 | 295 | 5.080 | 35.80 |
| **7** | **Monotone Radix Heap & 12 Hardware Threads**: Implemented 65-bucket RadixHeap exploiting Dijkstra's non-decreasing key property (O(1) amortized operations bucketed by highest differing bit, eliminating comparison overhead). Increased NUM_THREADS to 12 matching all available hardware threads on the official box. Full seed 1 verified byte-for-byte. | Official board run `045916e` | 0.886 | 314.0 | 332 | 8.214 | 45.728 |
| **8** | **BFS Reordering + 4-byte Packed CSR + Dial's Ring Algorithm**: BFS node renumbering puts adjacent nodes close together; packs edge destination $\Delta v$ (16-bit) and weight $w$ (16-bit) into a single 32-bit `pk` entry (20 MB total edge storage). Implemented Dial's algorithm with 4096-bucket ring buffer ($O(V + E)$ without heap operations), unchecked memory accesses, THP disable via `prctl`, and instant reachability pruning. Wall time dropped from 4.19s to 0.50s locally; peak RSS dropped from 469 MB to 143 MB. All three scales (tiny, sample, full) verified 100% byte-for-byte SHA-256 match. | Unofficial local WSL | 0.50 | 143.2 | 600 | 33.5 | ~76.2 |

---

### Step 0: Official Reference Reconciliation
- The official scoring host calibration time for the Python reference on seed 1 full is **72.69 s**, with peak RSS of **915.0 MB**.
- Earlier informal local reports cited 46.80s, which was measured on an unthrottled local developer workstation. All official scoring and K ratios are anchored against the 72.69s Falkenstein baseline.

### Step 1: Multi-Threaded Forward Dijkstra Baseline
- **Motivation:** Replace Python's interpreted graph representation and single-threaded search with a compact Compressed Sparse Row (CSR) graph and multi-core query execution.
- **Approach:**
  - Flat compact CSR graph: `offsets: Vec<u32>`, `edges_to: Vec<u32>`, `edges_weight: Vec<u32>`. Total graph memory for 1.74M nodes and 5M edges: ~47 MB.
  - $O(1)$ pre-check: If $s == t$, return 0 immediately. If `out_degree[s] == 0` or `in_degree[t] == 0`, return -1 immediately.
  - Self-loops ($u == v$) discarded during graph construction as they never contribute to shortest paths.
  - Epoch-based distance reset: Nodes store `visited_epoch[u]` alongside `dist[u]`. Incrementing `cur_epoch` resets search state without allocating or clearing 1.74M elements.
  - 6 worker threads using `std::thread::scope` with zero external crates ($D=1.000$).
- **Result:** $T = 2.71\text{ s}$ locally, $M = 170.8\text{ MB}$. SHA-256 matches frozen reference hash byte-for-byte.

### Step 2: 4-ary Min-Heap Experiment
- **Motivation:** Test whether a 4-ary heap reduces tree depth and memory traffic.
- **Finding:** A naive 4-ary heap requires multiple comparisons per level during sift-down. Standard `std::collections::BinaryHeap` benefits from hole/sift-down compiler optimizations and fewer child comparisons, outperforming naive 4-ary heap.
- **Decision:** Reverted to standard `BinaryHeap`.

### Step 3: Bidirectional Dijkstra Experiment
- **Motivation:** Test meeting-in-the-middle search to halve settled nodes.
- **Finding:** CPU search time dropped by 28% (user time 10.19s $\to$ 7.33s), but maintaining dual forward and backward CSR and per-thread backward distance arrays doubled memory from 170 MB to 337 MB. In the Speed Demon formula, the memory penalty offset the search speedup.
- **Decision:** Forward Dijkstra with epoch arrays delivers a higher overall $K$ coefficient ($K = 4.79$ vs $K = 3.73$).

### Step 4: Verification Across Multiple Scales & Seeds
- Tiny scale (seed 1): hash matches Python reference.
- Sample scale (seed 1): `a4c76ed030bc7e365db33191f6cf74708fba7f25eb48d64c8c4279f2e57046a0` (100% match).
- Full scale (seed 1): `a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e` (100% match).
- Tiny seed 42: identical output against Python reference.
- Sample seed 999: identical output against Python reference.

### Step 5: Official Board Verification & Metrics
- **Environment:** Dedicated official benchmark host (Intel i5-13500, 6 P-cores @ 2.5 GHz turbo off, 8 GB RAM, Linux offline container).
- **Official Result:**
  - $T = \mathbf{1.512\text{ s}}$ ($48.1\times$ faster than 72.69s reference).
  - $M = \mathbf{177.0\text{ MB}}$ ($5.17\times$ leaner than 915.0 MB reference).
  - $K = \mathbf{7.430230}$.
  - **Points Earned:** $\mathbf{43.550}$ machine points.
- **Correctness:** 100% SHA-256 match on full public dataset (`a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e`).

### Step 6: Dynamic Work Stealing & Cache-Interleaved CSR
- **Motivation:** Queries have highly uneven latency distributions (some paths settle in 5 steps; others explore 50,000 nodes). Static `chunks_mut` partitioning left threads idle while stragglers finished long queries. Separated `edges_to` and `edges_weight` arrays also caused dual cache misses per edge traversal.
- **Approach:**
  - Switched to dynamic query scheduling: an `AtomicUsize` counter where idle threads claim queries dynamically with `fetch_add(1, Relaxed)`, eliminating thread starvation.
  - Interleaved edge representation: unified target and weight into `struct EdgeTarget { to: u32, weight: u32 }` (8 bytes), ensuring single cache-line fetches during edge relaxation.
  - Added node query bounds checks (`sq >= n || tq >= n`) and epoch counter rollover handling.
  - Attempted a `touched: Vec<u32>` reset array to replace epoch arrays: measurement revealed 63% CPU slowdown (user time jumped from 13.8s to 22.6s). Retained epoch arrays based on empirical proof.
- **Result:** Total user CPU time decreased by **23%** (from 17.88s to 13.83s).

### Step 8: BFS Renumbering + 4-Byte Packed CSR + Dial's Ring Algorithm
- **Motivation:** Track D had been the dropped weakest track at 45.728 points (T=0.886s, M=314MB). Previous implementations stored 8-byte edges (or 16 bytes for forward+backward) and used priority queues (BinaryHeap or RadixHeap) with pointer-indirected distance tables.
- **Approach:**
  1. **BFS Node Renumbering:** A single BFS discovery traversal assigns contiguous IDs to topologically adjacent nodes.
  2. **4-Byte Packed CSR:** Because neighboring nodes have small index offsets $|\Delta v| \le 32767$ and road network weights are $w < 65535$, each edge is packed into a single 32-bit integer: `((dv as i16 as u16 as u32) << 16) | (w & 0xFFFF)`. Total edge memory for 5M edges dropped from 80 MB (or 160 MB) to **20 MB**, fitting inside CPU L3 cache. Infrequent escape edges go into a compact sorted binary-search table.
  3. **Dial's Algorithm with 4096-Bucket Ring Buffer:** Replaced priority heap with Dial's algorithm using a 4096-entry ring buffer. Edge relaxations are $O(1)$ direct array appends (`ring[nd & 4095].push(v)`), avoiding heap bubble-up/down or radix bucket redistribution.
  4. **Flat Pointer Distances & Touched Cleanup:** Per-thread direct distance array `d32` with unchecked memory access; visited nodes recorded in a flat `touched` buffer and cleared in microseconds.
  5. **Dijkstra Fallback:** Rare queries with $w \ge 4096$ or large sums seamlessly fall back to an exact 64-bit Dijkstra min-heap.
- **Results:**
  - Local wall time on WSL dropped from **4.19s** to **0.50s** (**7.2x faster**).
  - Peak RSS dropped from **469 MB** to **143 MB** (**70% reduction**).
  - 100% exact SHA-256 match on all three scales:
    - Tiny: `ecaca77d3221fe3928109da36480cfdb39f288f2e57ecbc61d3d7706d5cb4bd5`
    - Sample: `a4c76ed030bc7e365db33191f6cf74708fba7f25eb48d64c8c4279f2e57046a0`
    - Full: `a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e`
  - Zero external runtime dependencies ($D = 1.000$), zero compiler warnings.
  - Expected official score: $K \ge 33$, projected **75+ points**.
