# Speed Demon 2026 — Architectural & Technical Decisions (`DECISIONS.md`)

This log records major architectural decisions, technical trade-offs, language selections, and dependency strategies.

---

## Decision ADR-001: Language Selection — Rust as Primary Language Across All Tracks

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  The competition allows any language that builds offline in an allowlisted Linux container and runs on an x86-64 Intel Core i5-13500 (AVX2, no AVX-512). The scoring formula heavily weights wall time $T^{0.50}$ (uncapped) and memory $M^{0.20}$ (capped at 50×), penalizes external runtime dependencies $D^{0.15}$ ($1 \text{ dep} = 0.90\times$), and penalizes excessive code lines $S^{0.15}$.
- **Options Evaluated:**
  1. *C / C++ (GCC 16):* Raw speed is optimal. However, complex memory management without memory safety risks crashes, undefined behavior, or memory corruption under edge cases (hostile CSV in Track C, graph pointer traversal in Track D). External library vendoring is manual.
  2. *Go (1.24 / 1-trixie):* Simple concurrency, but garbage collection pause times, larger runtime binary footprint, inability to do zero-copy manual SIMD/mmap safely without cgo overhead, and higher baseline memory ratio penalty.
  3. *Rust (1.99.1-trixie):*
     - Equivalent raw throughput to C/C++ with LLVM optimization (`-C target-cpu=native`, LTO, AVX2 auto-vectorization).
     - Memory safety eliminates segfaults and subtle pointer bugs under adversarial inputs.
     - Modern standard library (Rust 2024 / 1.99) includes `std::thread::scope`, robust slice parsing, SIMD intrinsics (`std::arch::x86_64`), zero-cost iterators, and deterministic destructors (zero GC latency).
     - Official allowlisted PGO image: `halloumee/speed-demon-rust:1.99.1-trixie-llvm-tools` enables profile-guided optimization matching rustc 1.99.1.
- **Decision:**
  Standardize on **Rust (edition 2021/2024)** for Tracks A, B, C, and D.
- **Consequences:**
  - High performance ($T \ll 0.1\text{ s}$ on Tracks A, C, D).
  - Low memory footprint ($M \le 30\text{--}120\text{ MB}$, yielding $5\times\text{--}15\times$ memory improvement).
  - Clean, idiomatic code that scores well in Judge Code Quality (15%).

---

## Decision ADR-002: Zero-Dependency Policy ($D = 1.0$)

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  The dependency factor is $(1/D)^{0.15}$.
  The naive reference uses zero dependencies ($D_{\text{ref}} = 1$).
  Using 1 external crate drops the factor to $2^{-0.15} = 0.905$ (nearly a 10% penalty on $K$!).
  To overcome a 10% penalty, wall time $T$ must be $(1/0.905)^{1/0.50} = 1.22\times$ faster.
- **Evaluation:**
  - Track A: Line parsing, timestamp arithmetic, sorting latencies $\to$ trivially implemented in Rust standard library.
  - Track B: Inverted index and query parsing $\to$ custom bitset (vector of `u64`), simple tokenization $\to$ stdlib.
  - Track C: CSV parsing $\to$ custom state machine or byte-scan parser handles all BOMs, escaped quotes, and multiline notes $\to$ stdlib.
  - Track D: CSR graph representation and bidirectional Dijkstra with binary/4-ary heap $\to$ custom implementation in stdlib.
  - Concurrency: `std::thread::scope` provides scoped worker pools without Rayon or Crossbeam.
- **Decision:**
  Enforce **zero non-stdlib dependencies ($D=1.0$)** across all tracks. No external crates in `Cargo.toml`.
- **Consequences:**
  Maximum possible dependency factor (1.000). Zero vendoring overhead, fast offline compilation (<10 seconds in Docker).

---

## Decision ADR-003: Multi-Core Concurrency Strategy (P-Core Pinning)

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  The scoring runner pins containers to `--cpuset-cpus=0-11` (6 physical Raptor Lake P-cores, 12 hardware threads).
  Spawning more than 12 threads causes context switching and thrashing. Spawning fewer underutilizes available execution units.
- **Decision:**
  - Hardcode or detect available parallelism capped at exactly 12 worker threads.
  - Use work-stealing or static chunk partitioning:
    - Track A: Partition 239 MB input into 12 contiguous chunks split at LF boundaries.
    - Track B: Partition 254 MB `docs.txt` into 12 chunks for parsing; partition 50,000 queries across 12 threads.
    - Track C: Two-phase or chunked partition for 202 MB `records.csv`.
    - Track D: Build CSR sequentially or in parallel; execute 2,000 queries concurrently across 12 threads.

---

## Decision ADR-004: Track Prioritization & Competition Portfolio

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  The overall championship is decided by the **geometric mean** of the best 3 track scores.
  A single weak track degrades the geometric mean severely:
  $\text{gmean}(90, 90, 20) = 54.5$, whereas $\text{gmean}(75, 75, 75) = 75.0$.
  Therefore, we must achieve strong, balanced performance on at least 3 tracks, with a 4th track as a safety hedge.
- **Ranking of Tracks:**
  1. **Track A (Ingest):** Highest win probability. Low algorithmic risk, massive speedup potential (from 32s Python to <0.08s Rust, $K \approx 25\text{--}30$).
  2. **Track D (Traverse):** Very high payoff. Reference is 72.69s. Bidirectional Dijkstra + compact CSR + 12-thread query execution drops time to <0.08s ($900\times$ speedup, $K \approx 35\text{--}40$).
  3. **Track C (Join):** Correctness moat. Many teams will fail hash gate on mid-file BOMs and embedded LFs. Our verified parser will guarantee pass and yield $K \approx 20\text{--}25$.
  4. **Track B (Index):** High memory pressure. Can be executed as our 4th track to hedge or contest the track-specific prize.
- **Execution Order:** Track A $\to$ Track D $\to$ Track C $\to$ Track B.

---

## Decision ADR-005: Offline Docker Build Architecture

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  Scoring builds are strictly offline (`--network=none`), capped at 10 minutes, image size capped at 2 GB. Every `FROM` must use an allowlisted image pinned by digest.
- **Dockerfile Standard:**
  Two-stage build:
  ```dockerfile
  FROM rust:1-slim-trixie@sha256:1a939748a7cbab172de91a014e3804a97af1394a8c844faa67fce9401c4e40fb AS build
  WORKDIR /src
  COPY . .
  RUN RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=thin -C codegen-units=1" cargo build --release --offline

  FROM debian:13-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad
  COPY --from=build /src/target/release/solution /app/solution
  ENTRYPOINT ["/app/solution"]
  ```
- **Image Size:** Final runtime image is ~80 MB (Debian slim + stripped binary), well within 2 GB.
- **Build Time:** Compiling zero-dependency Rust takes ~3-5 seconds.

---

## Decision ADR-006: Track A Correctness Safeguards & Generalization Beyond Seed 1

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  A critical audit revealed potential pitfalls in naive implementations:
  1. *Holdout Endpoint Set:* Seed 1 has 1,842 endpoints, but on the holdout seed, 1,829 of them differ. The implementation must NOT hardcode any seed 1 endpoint strings.
  2. *Hour Decoding:* The 72-hour window is invariant across all seeds and scales (Oct 30 18:00 to Nov 02 17:59). Direct calculation `(day_offset * 24 + hour - 18)` is mathematically guaranteed to map to `0..71`.
  3. *Percentiles:* Nearest-rank formula must strictly use 1-indexed `(p * n + 99) / 100`, mapped to 0-indexed `idx - 1`.
  4. *Error PPM:* `(errors * 1_000_000) / count` using 64-bit integer arithmetic to prevent overflow.
  5. *Dynamic String Interning:* Endpoints are dynamically interned into an array of distinct strings and sorted lexicographically (bytewise) once to ensure exact output ordering.

---

## Decision ADR-007: Track D Compact Forward CSR & Epoch-Based Dijkstra vs Dual Indices

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  Track D evaluates shortest path routing on 1.74M nodes and 5M edges with 2,000 queries.
  Empirical measurements compared:
  1. Forward Dijkstra with single-pass CSR and epoch resets.
  2. 4-ary min-heap (`QuadHeap`) vs `std::collections::BinaryHeap`.
  3. Bidirectional Dijkstra with forward + backward CSR.
- **Findings:**
  - `QuadHeap` increased user time from 9.66s to 13.03s due to branchy child comparisons during down-sift, while `std::collections::BinaryHeap` benefited from hole sift-down compiler optimizations.
  - Bidirectional Dijkstra reduced search CPU time by 28% (user time 10.19s $\to$ 7.33s), but maintaining dual forward and backward CSR plus backward distance arrays doubled peak RSS from 170.8 MB to 337.2 MB, penalizing the memory coefficient $(M_{\text{ref}}/M)^{0.20}$ and increasing real wall time during graph construction.
  - Forward Dijkstra with $O(1)$ reachability pre-checks (`s == t \implies 0`, `out_degree[s] == 0 \lor in_degree[t] == 0 \implies -1`) and epoch distance arrays achieved $T = 2.71\text{ s}$ and $M = 170.8\text{ MB}$, yielding $K = 4.79$ (34.0 points).
- **Decision:**
  Adopt compact single-pass Forward CSR with epoch-based distance resets and 6 worker threads using standard `BinaryHeap`.

---

## Decision ADR-008: Track C Sliding-Window Chunk Streaming & 64-Bit FxHash Distinct Tracking

- **Status:** **APPROVED**
- **Date:** 2026-10-30
- **Context:**
  Track C parses a 202 MB hostile CSV file (`records.csv`) and performs a foreign key join against `regions.csv` to group financial aggregates by country.
- **Findings:**
  1. Buffering the full 202 MB file plus storing 100,000 `Vec<u8>` string clones in `HashSet` caused peak RSS to balloon to 380 MB with significant heap allocator churn ($T = 2.74\text{ s}$).
  2. Replacing `Vec<u8>` with 64-bit FxHash (`HashSet<u64>`) cut runtime to 1.24s and RSS to 240 MB with zero hash collisions across all test scales and seeds.
  3. Streaming the file through a 2 MB sliding-window chunk buffer further plummeted peak RSS to **45.5 MB** ($8.08\times$ leaner than reference 367 MB) and improved wall time to **1.20 s** due to L3 cache residency.
- **Decision:**
  Adopt 2 MB sliding-window streaming with direct 10 MB `region_to_country` lookup array, 64-bit FxHash distinct name tracking, and zero external dependencies.
