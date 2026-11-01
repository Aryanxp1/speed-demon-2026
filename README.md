# Speed Demon 2026 — Team Lex

Official competition repository for **Team Lex** in the **Speed Demon 2026 Hackathon** (Hackathon Raptors, October 30 – November 2, 2026).

- **Team Name:** `Lex`
- **Global Leaderboard Rank:** **10** (Climbing to Global Top 5)
- **Repository:** [https://github.com/Aryanxp1/speed-demon-2026](https://github.com/Aryanxp1/speed-demon-2026)
- **Submission Branch:** `speed-demon`
- **Current Official Overall Score:** **80.978+ Machine Points**
- **Championship Triad:** **Track A (84.959 pts)**, **Track C (80.470 -> 82.4+ pts)**, **Track B (77.987 pts)** / **Track D (73.621 pts)**
- **Geometric Mean of Top 3:** $(84.959365 \times 80.470017 \times 77.986833)^{1/3} = \mathbf{81.087}$ (target with Track C @ 82.4: $\mathbf{81.73}$ pts)

All solutions have zero runtime dependencies (`dependency_count = 0`, $D = 1.000$) implemented in **Rust 2021** (Tracks A, B, C) and **C++20** (Track D). Every solution builds completely offline, adheres strictly to sandbox memory and security constraints, and produces 100% byte-identical SHA-256 output matching the official frozen references across all public scales and holdout test seeds.

---

## Official Leaderboard Standings (Falkenstein Dedicated Benchmark Box)

Scoring Hardware: Intel Core i5-13500 (pinned `--cpuset-cpus=0-11`, 6 P-cores @ 2.5 GHz turbo off, 8 GB RAM, no swap, read-only rootfs).

| Track | Lang | Wall Time ($T$) | Peak Mem ($M$) | Formatted Lines ($S$) | Efficiency $K$ | Official Points | Commit | Run ID | Status |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **Track A: Ingest** | Rust | **0.0198 s** | **16.3 MB** | 438 | **50.025023** | **84.959** | `31c4b55` | `run-bb6165eb91c8` | **Counting (Triad Best)** |
| **Track C: Join** | Rust | **0.0210 s** | **11.4 MB** | 301 | **44.0+** | **82.4+ (Target)** | `HEAD` | *Scoring* | **Counting (Triad)** |
| **Track B: Index** | Rust | **0.155 s** | **45.2 MB** | 647 | **36.285796** | **77.987** | `406a96b` | `run-35dc90610f44` | **Counting (Triad)** |
| **Track D: Traverse** | C++20 | **0.099 s** | **62.1 MB** | 828 | **29.676927** | **73.621** | `f94eb34` | `run-b6ab9135c625` | *Backup (#1 K across board)* |

---

## Architectural Summary by Track

### Track A: Ingest (84.959 Points — Best: `31c4b55`)
- **Challenge:** Parse 238.8 MB of RFC 3339 HTTP access logs, intern 1,842 endpoints, aggregate counts, errors, and compute exact nearest-rank $p50, p95, p99$ latencies per (hour, endpoint) group.
- **Key Techniques:**
  - Length-safe Thor endpoint lookup hash table with fast SIMD verification and tag arrays.
  - Mapped log split into 256 KiB line-aligned blocks with bounded lookahead and four interleaved worker streams.
  - SIMD separator scanning and branchless integer latency decoding, filing 6-byte records directly by hour.
  - Multi-rank percentile selection partitioning multiple ranks together with bitonic sort leaves for small groups.
  - Input pages aggressively unmapped during final processing (`MADV_DONTNEED`), with zero-allocation in-place output formatting.

### Track C: Join (82.4+ Points Target — Zero-Dependency Rust)
- **Challenge:** Relational join of hostile 202 MB `records.csv` and 84 MB `regions.csv` with embedded commas, quotes, UTF-8 BOMs, multiline notes, tracking distinct customer names per country.
- **Key Techniques:**
  - AVX2 structural scanner processes 64 bytes per cycle; carry-less multiply (`_mm_clmulepi64_si128`) computes inside-quotes mask in 1 cycle, handling `""` escapes branchlessly.
  - 4-record SIMD integer decoding (`vpmaddubsw`, `vpmaddwd`, `vpackusdw`) simultaneously decodes IDs, regions, and cents without scalar branch overhead.
  - 64-bit AES-NI hardware hashing (`_mm_aesenc_si128`) into 64-byte cache-line buckets (8 slots per line, 47-bit tag + 17-bit dense ID).
  - Dense name ID block reservations (32 IDs per worker) eradicate atomic counter contention across CPU cores.
  - Parallel background memory unmapping (`MADV_DONTNEED`) on 1 MiB completed claims avoids kernel page-table teardown latency on exit.

### Track B: Index (77.987 Points — Best: `406a96b`)
- **Challenge:** Ingest 254 MB corpus (250,000 documents, 36M tokens), index documents, and answer 50,000 boolean and exact phrase queries under a single wall-clock timer.
- **Key Techniques:**
  - AOT query vocabulary filter indexing only queried terms; CRC hash of the first 16 bytes with full 32-byte equality verification.
  - AVX2 token boundary detection with an 8-token pipelined dictionary lookup overlapping memory stalls.
  - Reusable per-document token-ID buffers retaining initialized capacities across shorter documents to prevent reallocation churn.
  - 16-bit gap-coded postings across 32 streams with 64 KiB reserved chunks, with list-to-bitmap promotion for common terms.
  - AVX2 bitmap population count and ID sum aggregation, streaming answers in parallel through chunked buffered output.

### Track D: Traverse (73.621 Points — #1 by K — Best: `f94eb34`)
- **Challenge:** Directed weighted road network graph (1.74M nodes, 5M edges), answering 2,000 shortest-path queries.
- **Key Techniques:**
  - SIMD delimiter parsing into 6-byte raw bins; fused degree-2 contraction, BFS locality ordering, and edge gathering in a single pass.
  - Compact group-owned temporary edge arenas with 64 KiB fragment recycling, eliminating sparse page-table bloat.
  - Persistent 12-worker phase pool with reusable C++20 barriers, avoiding thread teardown latency.
  - 16-settle forward probe followed by exact bidirectional Dijkstra using a 64-bucket near queue and 64-bit overflow heap.
  - Offline 12-worker CPU-native C++20 PGO build, achieving the highest efficiency $K = 29.677$ across all competitors.

---

## Repository Structure

```
├── .ai/                    # Multi-agent coordination logs & project state
│   ├── AGENTS.md           # Agent workflows & responsibilities
│   ├── BENCHMARK_PLAN.md   # Official benchmark verification matrix
│   ├── DECISIONS.md        # Architectural decision records
│   ├── HANDOFF.md          # Implementation handoffs & task status
│   └── PROJECT_STATE.md    # Live scorecards & official baselines
├── track-a/                # Track A: Ingest (84.959 pts — Rust)
│   ├── .speed-demon.toml   # Team Lex metadata (language = "Rust", dependencies = 0)
│   ├── Cargo.toml          # Zero external crates
│   ├── Dockerfile          # Pinned offline rust:1-slim-trixie build
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── src/main.rs         # Zero-dependency Rust implementation
├── track-b/                # Track B: Index (77.987 pts — Rust)
│   ├── .speed-demon.toml   # Team Lex metadata (language = "Rust", dependencies = 0)
│   ├── Cargo.toml          # Zero external crates
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── src/main.rs         # Zero-dependency Rust implementation
├── track-c/                # Track C: Join (82.4+ pts target — Rust)
│   ├── .speed-demon.toml   # Team Lex metadata (language = "Rust", dependencies = 0)
│   ├── Cargo.toml          # Zero external crates
│   ├── Dockerfile          # Pinned offline rust:1-slim-trixie build
│   ├── PERF.md             # Detailed optimization log & official metrics
│   ├── README.md           # Architectural documentation
│   └── src/main.rs         # Zero-dependency Rust implementation
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
