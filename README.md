# Speed Demon 2026 — Team Lex

Official competition repository for **Team Lex** in the **Speed Demon 2026 Hackathon** (Hackathon Raptors, October 30 – November 2, 2026).

- **Team Name:** `Lex`
- **Global Leaderboard Rank:** **10**
- **Repository:** [https://github.com/Aryanxp1/speed-demon-2026](https://github.com/Aryanxp1/speed-demon-2026)
- **Submission Branch:** `speed-demon`
- **Current Official Overall Score:** **79.476 Machine Points**
- **Championship Triad:** **Track A (84.737 pts)**, **Track C (80.470 pts)**, **Track D (73.621 pts)**
- **Geometric Mean of Top 3:** $(84.737403 \times 80.470017 \times 73.620947)^{1/3} = \mathbf{79.476}$

All solutions have zero runtime dependencies (`dependency_count = 0`, $D = 1.000$) implemented in **C++20** (Tracks A, C, D) and **Rust 2021** (Track B). Every solution builds completely offline, adheres strictly to sandbox memory and security constraints, and produces 100% byte-identical SHA-256 output matching the official frozen references across all public scales and holdout test seeds.

---

## Official Leaderboard Standings (Falkenstein Dedicated Benchmark Box)

Scoring Hardware: Intel Core i5-13500 (pinned `--cpuset-cpus=0-11`, 6 P-cores @ 2.5 GHz turbo off, 8 GB RAM, no swap, read-only rootfs).

| Track | Lang | Wall Time ($T$) | Peak Mem ($M$) | Formatted Lines ($S$) | Efficiency $K$ | Official Points | Commit | Run ID | Status |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **Track A: Ingest** | C++20 | **0.017 s** | **21 MB** | 575 | **49.516286** | **84.737** | `60cb66c` | `run-6e5a4c297a13` | **Counting (Triad)** |
| **Track C: Join** | C++20 | **0.028 s** | **9 MB** | 320 | **40.681817** | **80.470** | `d752aea` | `run-fe6bb38f59d2` | **Counting (Triad)** |
| **Track D: Traverse** | C++20 | **0.099 s** | **62 MB** | 828 | **29.676927** | **73.621** | `f94eb34` | `run-b6ab9135c625` | **Counting (Triad, #1 by K)** |
| **Track B: Index** | Rust | **0.173 s** | **47 MB** | 647 | **28.977962** | **73.103** | `2c9f85b` | `run-237d62c0cf7c` | *Dropped (Weakest of 4)* |

**Geometric Mean of Best Three Tracks (A, C, D):**
$$\sqrt[3]{84.737403 \times 80.470017 \times 73.620947} = \mathbf{79.476 \text{ points}}$$

---

## Architectural Summary by Track

### Track A: Ingest (84.737 Points — Best: `60cb66c`)
- **Challenge:** Parse 238.8 MB of RFC 3339 HTTP access logs, intern 1,842 endpoints, aggregate counts, errors, and compute exact nearest-rank $p50, p95, p99$ latencies per (hour, endpoint) group.
- **Key Techniques:**
  - Mapped log split into 256 KiB line-aligned blocks with bounded lookahead and four interleaved worker streams.
  - SIMD separator scanning and branchless integer latency decoding, filing 6-byte records directly by hour.
  - Private endpoint lookup mirrors over an append-only shared dictionary, comparing endpoint bytes with AVX2.
  - AVX2 multi-rank percentile selection partitioning multiple ranks together with bitonic sort leaves for small groups.
  - Input pages aggressively unmapped during final processing, with zero-allocation in-place output formatting.

### Track C: Join (80.470 Points — Best: `d752aea`)
- **Challenge:** Relational join of hostile 202 MB `records.csv` and 84 MB `regions.csv` with embedded commas, quotes, UTF-8 BOMs, multiline notes, tracking distinct customer names per country.
- **Key Techniques:**
  - Compact fingerprint hash table with atomic SIMD tag group probing; CAS assigns slots with relaxed ordering.
  - Two-record pipeline overlapping region lookups, name interning, and atomic bitmap marking.
  - Atomic bitmap `fetch_or` (lock BTS) marks first sightings directly in worker-private distinct counters, avoiding post-join bitmap scans.
  - Grouped input unmapping releases completed mapping interiors in 4-piece batches to reduce page-table overhead.
  - Offline seed-17 Profile-Guided Optimization (PGO) using the organizer's published generator blob under the official source-counting exemption.

### Track D: Traverse (73.621 Points — #1 by K — Best: `f94eb34`)
- **Challenge:** Directed weighted road network graph (1.74M nodes, 5M edges), answering 2,000 shortest-path queries.
- **Key Techniques:**
  - SIMD delimiter parsing into 6-byte raw bins; fused degree-2 contraction, BFS locality ordering, and edge gathering in a single pass.
  - Compact group-owned temporary edge arenas with 64 KiB fragment recycling, eliminating sparse page-table bloat.
  - Persistent 12-worker phase pool with reusable C++20 barriers, avoiding thread teardown latency.
  - 16-settle forward probe followed by exact bidirectional Dijkstra using a 64-bucket near queue and 64-bit overflow heap.
  - Offline 12-worker CPU-native C++20 PGO build, achieving the highest efficiency $K = 29.677$ across all competitors.

### Track B: Index (73.103 Points — Best: `2c9f85b`)
- **Challenge:** Ingest 254 MB corpus (250,000 documents, 36M tokens), index documents, and answer 50,000 boolean and exact phrase queries under a single wall-clock timer.
- **Key Techniques:**
  - AOT query vocabulary filter indexing only queried terms; CRC hash of the first 16 bytes with full 32-byte equality verification.
  - AVX2 token boundary detection with an 8-token pipelined dictionary lookup overlapping memory stalls.
  - Reusable per-document token-ID buffers retaining initialized capacities across shorter documents to prevent reallocation churn.
  - 16-bit gap-coded postings across 32 streams with 64 KiB reserved chunks, with list-to-bitmap promotion for common terms.
  - AVX2 bitmap population count and ID sum aggregation, streaming answers in parallel through chunked buffered output.

---

## Repository Structure

```
├── .ai/                    # Multi-agent coordination logs & project state
│   ├── AGENTS.md           # Agent workflows & responsibilities
│   ├── BENCHMARK_PLAN.md   # Official benchmark verification matrix
│   ├── DECISIONS.md        # Architectural decision records
│   ├── HANDOFF.md          # Implementation handoffs & task status
│   └── PROJECT_STATE.md    # Live scorecards & official baselines
├── track-a/                # Track A: Ingest (84.737 pts — C++20)
│   ├── .speed-demon.toml   # Team Lex metadata (language = "C++", dependencies = 0)
│   ├── Dockerfile          # Pinned offline GCC static build
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── main.cpp            # Zero-dependency C++20 implementation
├── track-b/                # Track B: Index (73.103 pts — Rust)
│   ├── .speed-demon.toml   # Team Lex metadata (language = "Rust", dependencies = 0)
│   ├── Cargo.toml          # Zero external crates
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── src/main.rs         # Zero-dependency Rust implementation
├── track-c/                # Track C: Join (80.470 pts — C++20)
│   ├── .speed-demon.toml   # Team Lex metadata (language = "C++", dependencies = 0)
│   ├── Dockerfile          # Pinned offline GCC static PGO build
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── main.cpp            # Zero-dependency C++20 implementation
├── track-d/                # Track D: Traverse (73.621 pts — C++20)
│   ├── .speed-demon.toml   # Team Lex metadata (language = "C++", dependencies = 0)
│   ├── Dockerfile          # Pinned offline GCC static PGO build
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── main.cpp            # Zero-dependency C++20 implementation
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
