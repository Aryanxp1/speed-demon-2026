# Track D: Performance Log

Every step from the reference to our optimized version, with measured metrics, architectural rationale, and verified SHA-256 correctness.

| Step | Change | Run ID | T (s) | M (MB) | S (lines) | Local K | Machine Points |
|---|---|---|---|---|---|---|---|
| **0** | Naive Python reference (CPython 3.13, `heapq`, adjacency list) | `ref-fsn1` | 46.80 | 915.0 | 73 | 1.00 | 0.0 |
| **1** | Multi-threaded Forward Dijkstra in zero-dependency Rust with single-pass CSR, epoch-based distance tracking (zero array resets), and $O(1)$ in-degree/out-degree reachability pruning. | `local-forward-6th` | 2.71 | 170.8 | 266 | 4.79 | 34.0 |
| **2** | Experiment: Naive 4-ary Min-Heap (`QuadHeap`). *Rejected*: Bubble-down comparisons increased user CPU time from 9.66s to 13.03s, real time 3.67s. Reverted to standard `BinaryHeap`. | `local-exp-quadheap` | 3.67 | 170.8 | 280 | 4.12 | 30.7 |
| **3** | Experiment: Bidirectional Dijkstra with forward + backward CSR. *Evaluated*: User CPU time dropped from 10.19s to 7.33s (-28%), but graph setup and backward distance arrays increased peak RSS to 337 MB and real time to 3.42s. Retained forward Dijkstra as primary lean solution. | `local-exp-bidir` | 3.42 | 337.2 | 315 | 3.73 | 28.6 |
| **4** | Multi-threaded Forward Dijkstra with single-pass ASCII parsing and in-memory CSR scatter. Verified 100% byte-identical SHA-256 across tiny, sample, full, and holdout seeds (42, 999). | `local-opt-v1` | 2.71 | 170.8 | 266 | 4.79 | 34.0 |

---

### Step 1: Multi-Threaded Forward Dijkstra Baseline
- **Motivation:** Replace Python's interpreted graph representation and single-threaded search with a compact Compressed Sparse Row (CSR) graph and multi-core query execution.
- **Approach:**
  - Flat compact CSR graph: `offsets: Vec<u32>`, `edges_to: Vec<u32>`, `edges_weight: Vec<u32>`. Total graph memory: ~47 MB.
  - $O(1)$ pre-check: If $s == t$, return 0 immediately. If `out_degree[s] == 0` or `in_degree[t] == 0`, return -1 immediately.
  - Self-loops ($u == v$) discarded during graph construction as they never contribute to shortest paths.
  - Epoch-based distance reset: Nodes store `visited_epoch[u]` alongside `dist[u]`. Incrementing `cur_epoch` resets search state without allocating or clearing 1.74M elements.
  - 6 worker threads using `std::thread::scope` with zero external crates ($D=1.000$).
- **Result:** $T = 2.71\text{ s}$ (vs 46.8s Python reference $\implies \mathbf{17.3\times\text{ speedup}}$), $M = 170.8\text{ MB}$ (vs 915 MB Python reference $\implies \mathbf{5.36\times\text{ leaner}}$). SHA-256 matches frozen reference hash byte-for-byte.

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
