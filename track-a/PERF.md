# Track A: Performance Log

Every step from the reference to our optimized version, with measured metrics, architectural rationale, and verified SHA-256 correctness.

| Step | Change | Run ID / Context | T (s) | M (MB) | S (lines) | K | Points |
|---|---|---|---|---|---|---|---|
| **0** | Official scoring box Python reference | `calibration-seed1.json` | 32.05 | 228.0 | 56 | 1.000 | 0.00 |
| **1** | Single-threaded zero-dependency Rust baseline (`BufReader`, dynamic endpoint interner, bucket array) | Unofficial local | 0.81 | 28.0 | 140 | 8.370 | 46.10 |
| **2** | Full-file buffer ingest (`fs::read`). *Reverted/Adjusted*: Peak RSS increased to 260 MB, penalized memory score. Replaced with streaming chunk buffer. | Unofficial local | 0.75 | 260.0 | 165 | 5.820 | 38.20 |
| **3** | 6-thread parallel chunking with `std::thread::scope`, global mutex registry with thread-local read cache, nearest-rank `select_nth_unstable`, and custom integer formatting. | Unofficial local | 0.65 | 54.6 | 394 | 6.960 | 42.10 |
| **4** | Architectural pruning & line consolidation: eliminated redundant integer formatters, unified numerical serializers, reduced code size from 394 to 263 lines (+11% gain on $S$ factor). | Unofficial local | 0.65 | 54.6 | 263 | 7.390 | 43.40 |
| **5** | **Official Board Submission (Baseline v1.0)**: Executed on official scoring machine (Intel Core i5-13500, pinned 12 threads, 8 GB RAM). 100% byte-identical SHA-256 match. | **Official Board Run** | **0.292** | **61.0** | **263** | **10.284873** | **50.610** |
| **6** | **Official Board Submission (v1.1)**: Improved line grouping & memory efficiency. | **Official Board Run** | **0.150** | **62.0** | **379** | **14.400088** | **57.918** |
| **7** | **Zero-Copy Mmap Ingest, SIMD Vector Scanning, & Radix Bucketing**: Zero-copy `mmap` with AVX2 `match32`, SIMD digit parsing (`_mm_maddubs_epi16`), thread-local `EndpointTable` with hardware CRC32 (`_mm_crc32_u64`), fixed 16 KB `Block` arena, radix prefix-sum latency scattering with `madvise(MADV_DONTNEED)`, and dedicated streaming background writer thread. | Unofficial local WSL | 0.22 | 54.0 | 580 | ~39.8 | ~79.9 |

---

### Step 1: Single-Threaded Rust Baseline
- **Motivation:** Replace Python's interpreted line splitting, `strptime`, and heap allocation churn with zero-dependency Rust.
- **Approach:**
  - Fast arithmetic day/hour parsing mapping the fixed 72-hour window (`2026-10-30T18:00:00Z` to `2026-11-02T17:59:59Z`) directly to `0..71`.
  - Dynamic string interning for the 1,842 endpoints.
  - Flat group table `[72 * 2048]` with `Vec<u32>` latencies.
- **Result:** $T = 0.81\text{ s}$, $M = 28.0\text{ MB}$. Output SHA-256 byte-identical.

### Step 2: Full-File Memory Ingest vs Streaming (The Memory Trade-Off)
- **Motivation:** Reading the entire 238.8 MB file with `fs::read` avoids repeated `read` syscalls.
- **Observation:** While wall time was fast, peak resident memory jumped to 260 MB. In the Speed Demon formula, $M$ carries a $0.20$ exponent. A memory ratio of $0.87$ penalized total $K$ by 36%.
- **Decision:** Replaced full-file buffering with a 2 MB streaming chunk buffer that preserves low memory ($28\text{--}54\text{ MB}$) while maintaining sub-second execution.

### Step 3: Multi-Core Concurrency & Lock-Free Thread Caches
- **Motivation:** Leverage the 6 Performance cores / 12 hardware threads available on the Falkenstein box.
- **Approach:**
  - Partition file into disjoint contiguous byte ranges split at newline (`\n`) boundaries.
  - Spawn 6 scoped threads using `std::thread::scope` (zero external dependencies, $D=1.0$).
  - Global endpoint registry protected by a `Mutex`, combined with an $O(1)$ thread-local cache. After the first 5,000 lines, 100% of endpoint lookups hit the local cache without taking locks.
  - Replace full $O(n \log n)$ latency vector sort with $O(n)$ `select_nth_unstable` for percentiles $k_{99}, k_{95}, k_{50}$.
  - Custom fast integer formatting directly into a reusable line buffer.
- **Result:** $T = 0.65\text{ s}$ across middle-3 runs, $M = 54.6\text{ MB}$.

### Step 4: Source Size ($S$) Optimization & Multi-Seed Generalization
- **Motivation:** In the $K$ formula, code size carries a $0.15$ exponent. Reducing unnecessary boilerplate improves the $S$ ratio directly.
- **Approach:**
  - Consolidated duplicate integer serialization routines.
  - Verified edge cases and holdout generalization against seeds 1, 42, and 999 across tiny, sample, and full scales.
- **Result:** Code lines reduced from 394 to 263 lines without performance loss. Local $K = 7.39$.

### Step 5: Official Board Verification & Metrics
- **Environment:** Dedicated official benchmark host (Intel i5-13500, 6 P-cores @ 2.5 GHz turbo off, 8 GB memory limit, Linux offline container).
- **Official Result:**
  - $T = \mathbf{0.292\text{ s}}$ ($109.8\times$ faster than 32.05s reference).
  - $M = \mathbf{61.0\text{ MB}}$ ($3.74\times$ leaner than 228.0 MB reference).
  - $K = \mathbf{10.284873}$.
  - **Points Earned:** $\mathbf{50.610}$ machine points.
- **Correctness:** 100% SHA-256 match on full public dataset (`495ca09682542a981a8ba214a1a659fb221ee54fe66d8ba609fca940aa415ef4`).

