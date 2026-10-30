# Speed Demon 2026 — Team Lex

Official competition repository for **Team Lex** in the **Speed Demon 2026 Hackathon** (Hackathon Raptors, October 30 – November 2, 2026).

- **Team Name:** `Lex`
- **Repository:** [https://github.com/Aryanxp1/speed-demon-2026](https://github.com/Aryanxp1/speed-demon-2026)
- **Submission Branch:** `speed-demon`
- **Current Official Overall Score:** **45.686 Machine Points**
- **Championship Triad:** **Track A (50.610 pts)**, **Track D (43.550 pts)**, **Track B (43.265 pts)**
- **Geometric Mean of Top 3:** $(50.610 \times 43.550 \times 43.265)^{1/3} = \mathbf{45.686}$

All solutions are implemented in pure **Rust (Edition 2021)** using only the standard library (`dependency_count = 0`, $D = 1.000$). Every solution builds completely offline, adheres strictly to sandbox memory and security constraints, and produces 100% byte-identical SHA-256 output matching the official frozen references across all public scales and holdout test seeds.

---

## Official Leaderboard Standings (Falkenstein Dedicated Benchmark Box)

Scoring Hardware: Intel Core i5-13500 (pinned `--cpuset-cpus=0-11`, 6 P-cores @ 2.5 GHz turbo off, 8 GB RAM, no swap, read-only rootfs).

| Track | Name | Ref Time ($T_{\text{ref}}$) | Ref Mem ($M_{\text{ref}}$) | Team Time ($T$) | Team Mem ($M$) | Speedup | Mem Ratio | Formula $K$ | Machine Points | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| **Track A** | **Ingest** | 32.05 s | 228.0 MB | **0.292 s** | **61.0 MB** | **109.8x** | **3.74x** | **10.284873** | **50.610** | **Championship Triad** |
| **Track B** | **Index** | 56.35 s | 2,200.0 MB | **1.071 s** | **534.0 MB** | **52.6x** | **4.12x** | **7.333291** | **43.265** | **Championship Triad** |
| **Track C** | **Join** | 14.44 s | 367.0 MB | **0.926 s** | **88.0 MB** | **15.6x** | **4.17x** | **4.378513** | **32.066** | Complete / Verified |
| **Track D** | **Traverse** | 72.69 s | 915.0 MB | **1.512 s** | **177.0 MB** | **48.1x** | **5.17x** | **7.430230** | **43.550** | **Championship Triad** |

---

## Architectural Summary by Track

### Track A: Ingest (50.610 Points)
- **Challenge:** Parse 238.8 MB of RFC 3339 HTTP access logs, intern 1,842 endpoints, aggregate counts, errors, and compute exact nearest-rank $p50, p95, p99$ latencies per (hour, endpoint) group.
- **Key Techniques:**
  - Branchless arithmetic day/hour parser mapping the invariant 72-hour window (`2026-10-30T18:00:00Z` to `2026-11-02T17:59:59Z`) directly to `0..71` slots.
  - 6-thread parallel ingest partitioned across newline boundaries, streaming in 1 MB chunks to maintain a lean 61 MB footprint ($3.74\times$ leaner than reference).
  - Global endpoint registry with $O(1)$ thread-local read caches (zero lock contention after warmup).
  - $O(n)$ `select_nth_unstable` percentile partitioning avoiding full quicksorts.

### Track B: Index (43.265 Points)
- **Challenge:** Ingest 254 MB corpus (250,000 documents, 36M tokens), index documents, and answer 50,000 boolean and exact phrase queries under a single wall-clock timer.
- **Key Techniques:**
  - **AOT Query Vocabulary Filtering:** Reads `queries.txt` before indexing. Only 75,747 queried terms (out of 609,833 corpus vocabulary) are indexed, discarding 87% of terms at ingest.
  - **Thread-Partitioned Monotonic Postings:** Worker threads emit strictly ascending document IDs; merging thread postings is a linear concatenation with zero sorting.
  - Intermediate thread structures explicitly dropped (`drop(thread_results)`) to reclaim 89 MB RAM before query answering.
  - **Sparse Boolean Evaluation:** Direct bitwise operations for $df < 256$ in $O(df)$ time.
  - **Safe Phrase Verification:** Ascending document-frequency candidate filtering verified in-place using stack-allocated `(u32, u32)` spans (up to 1,024 tokens) with dynamic heap fallback for unusually long documents. Static lookup tables eliminate branch mispredictions.

### Track C: Join (32.066 Points)
- **Challenge:** Relational join of hostile 202 MB `records.csv` and 84 MB `regions.csv` with embedded commas, quotes, UTF-8 BOMs, multiline notes, tracking distinct customer names per country.
- **Key Techniques:**
  - Flat 10 MB direct lookup array `region_to_country: Vec<u8>` mapping region IDs $0 \dots 9,999,999$ in $O(1)$ time with zero hash overhead.
  - Sliding-window 2 MB streaming buffer keeping working sets inside CPU L3 cache (88 MB peak RSS).
  - **Collision-Proof Exact-Byte Name Table:** Open-addressing hash table backed by a contiguous `Vec<u8>` string arena. Compares raw slice bytes directly, completely eliminating birthday-paradox collision risks on unseen holdout datasets.
  - Integer cents arithmetic for exact financial math.

### Track D: Traverse (43.550 Points)
- **Challenge:** Directed weighted road network graph (1.74M nodes, 5M edges), answering 2,000 shortest-path queries.
- **Key Techniques:**
  - Compact Compressed Sparse Row (CSR) graph layout with interleaved 8-byte `struct EdgeTarget { to: u32, weight: u32 }`, eliminating dual cache misses during edge relaxation.
  - **Dynamic Atomic Work Scheduling:** Queries claimed dynamically across 6 threads via `AtomicUsize::fetch_add(1, Relaxed)`, eliminating thread idle stalls from path length imbalance (reduced CPU user time by 23%).
  - Epoch-based distance reset: thread-local `visited_epoch[u]` counter resets Dijkstra states in $O(1)$ time without array zeroing.
  - $O(1)$ reachability checks for self-loops, zero in-degree, and zero out-degree vertices.

---

## Repository Structure

```
├── .ai/                    # Multi-agent coordination logs & project state
│   ├── AGENTS.md           # Agent workflows & responsibilities
│   ├── BENCHMARK_PLAN.md   # Official benchmark verification matrix
│   ├── DECISIONS.md        # Architectural decision records
│   ├── HANDOFF.md          # Implementation handoffs & task status
│   └── PROJECT_STATE.md    # Live scorecards & official baselines
├── track-a/                # Track A: Ingest (50.610 pts)
│   ├── .speed-demon.toml   # Team Lex metadata (language, dependencies = 0)
│   ├── Dockerfile          # Offline 2-stage build pinned by digest
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── src/main.rs         # Zero-dependency Rust implementation
├── track-b/                # Track B: Index (43.265 pts)
├── track-c/                # Track C: Join (32.066 pts)
├── track-d/                # Track D: Traverse (43.550 pts)
├── check.py                # Official offline build and verification harness
├── LICENSE                 # MIT License (OSI-approved)
└── README.md               # Team overview & system documentation
```

---

## Verification and Correctness

All implementations produce 100% byte-identical SHA-256 outputs matching the official frozen references on all public seeds and holdout scales:

| Track | Dataset / Scale | Expected SHA-256 Checksum | Verified Match |
|---|---|---|---|
| **A** | Seed 1 Full | `495ca09682542a981a8ba214a1a659fb221ee54fe66d8ba609fca940aa415ef4` | **100% Match** |
| **B** | Seed 1 Full | `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927` | **100% Match** |
| **C** | Seed 1 Full | `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536` | **100% Match** |
| **D** | Seed 1 Full | `a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e` | **100% Match** |

Every track also verifies byte-for-byte on `sample`, `tiny`, and additional test seeds (42 and 999).

---

## Building and Testing Locally

Each track can be tested using the official offline sandbox check:

```bash
# Verify Track A
python check.py a --data /path/to/data/a --expected 495ca09682542a981a8ba214a1a659fb221ee54fe66d8ba609fca940aa415ef4

# Verify Track B
python check.py b --data /path/to/data/b --expected f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927

# Verify Track C
python check.py c --data /path/to/data/c --expected d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536

# Verify Track D
python check.py d --data /path/to/data/d --expected a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e
```

## Licence

MIT Licence — see [LICENSE](LICENSE) for details.
