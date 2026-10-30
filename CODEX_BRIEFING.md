# Speed Demon 2026 — Master Engineering Briefing for Codex

**To:** Codex (Senior Systems & Performance Implementation Engineer)  
**From:** Antigravity (Lead Systems Architect & Performance Strategist)  
**Team Name:** `Lex` (STRICT: All configs, commit metadata, and documentation MUST identify team as `Lex`)  
**Repository:** [https://github.com/Aryanxp1/speed-demon-2026](https://github.com/Aryanxp1/speed-demon-2026)  
**Submission Branch:** `speed-demon` (Current Head: `42768c9`)  
**Current Live Official Score:** **45.686 Machine Points** (Top 3 Triad: Track A, Track D, Track B)  
**Strict Safety Rule:** **DO NOT RUN `git push`. Pushes are strictly authorized and executed by the human team lead.**

---

## 1. Executive Summary & Where We Stand

We are competing in **Speed Demon 2026** (Hackathon Raptors, October 30 – November 2, 2026).
All four tracks (A, B, C, D) are implemented in pure, zero-dependency **Rust (Edition 2021)** and are live on the official Falkenstein benchmark server.

### Official Scoring Hardware (The "Falkenstein Box")
- **CPU:** Intel Core i5-13500 pinned to `--cpuset-cpus=0-11` (6 Performance Cores @ 2.5 GHz base, Turbo OFF, 12 hardware threads, 24 MB L3 cache).
- **RAM:** 8.0 GB RAM, **NO SWAP** (`--memory=8g --memory-swap=8g`).
- **Sandbox Environment:** Linux offline container (`--network=none`), read-only root filesystem, unprivileged user `65534:65534`, 512 process limit, 1 GB tmpfs at `/tmp:rw,size=1g,exec`.
- **Allowed Instructions:** AVX2, FMA, BMI1/2. **NO AVX-512** (i5-13500 Raptor Lake disabled AVX-512).

### Current Official Benchmark Baseline

| Track | Task | Official Ref ($T_{\text{ref}}$) | Ref Mem ($M_{\text{ref}}$) | Our Time ($T$) | Our Mem ($M$) | Speedup | Mem Ratio | Formula $K$ | Machine Points | Role in Portfolio |
|:---:|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **Track A** | **Ingest** | 32.05 s | 228.0 MB | **0.292 s** | **61.0 MB** | **109.8x** | **3.74x** | **10.284873** | **50.610** | **Championship Triad** |
| **Track B** | **Index** | 56.35 s | 2,200.0 MB | **1.071 s** | **534.0 MB** | **52.6x** | **4.12x** | **7.333291** | **43.265** | **Championship Triad** |
| **Track C** | **Join** | 14.44 s | 367.0 MB | **0.926 s** | **88.0 MB** | **15.6x** | **4.17x** | **4.378513** | **32.066** | Safety Hedge / Contender |
| **Track D** | **Traverse** | 72.69 s | 915.0 MB | **1.512 s** | **177.0 MB** | **48.1x** | **5.17x** | **7.430230** | **43.550** | **Championship Triad** |

$$\text{Current Overall Score} = (50.610 \times 43.550 \times 43.265)^{1/3} = \mathbf{45.686}$$

Our goal is to push the geometric mean from **45.686 to 52.0+ and 56.0+**.

---

## 2. The Official Scoring Formula & Optimization Calculus

To make sound engineering decisions, you must master the scoring mechanics:

### The Performance Coefficient ($K$)
$$K = \left(\frac{T_{\text{ref}}}{T}\right)^{0.50} \times \min\left(\frac{M_{\text{ref}}}{M}, 50\right)^{0.20} \times \min\left(\frac{S_{\text{ref}}}{S}, 1.0\right)^{0.15} \times \left(\frac{1}{D}\right)^{0.15}$$

Where:
- $T$: Wall-clock execution time in seconds. **Exponent 0.50 (Highest leverage).**
- $M$: Peak Resident Set Size (RSS) in megabytes. **Exponent 0.20.**
- $S$: Effective non-empty code lines. **Exponent 0.15 (Capped at 1.0, no bonus for shrinking below reference).**
- $D$: Direct external runtime dependencies count. Naive Python has $D=1$.
  - With 0 external crates: $D = 1 \implies (1/1)^{0.15} = 1.000$.
  - With 1 external crate: $D = 2 \implies (1/2)^{0.15} = 0.901$ (**-9.9% penalty on $K$ across the board!**).
  - **Rule:** **STRICT ZERO DEPENDENCY ($D=1.000$). Never add external crates to `Cargo.toml`.**

### Overall Championship Score
- **Track Final Score:** $0.65 \times \text{Machine Points} + 0.35 \times \text{Judge Score}$.
- **Overall Championship:** Geometric mean of top 3 track final scores.
- **Human Judging (35%):**
  - Code Quality (15%): Clarity, idiomatic Rust, zero warnings, maintainability.
  - Technique (12%): Sophistication of data structures, cache awareness, hardware mechanical sympathy.
  - Optimization Log (8%): Completeness of `PERF.md`, documenting failures and measurements.

---

## 3. High-Leverage Leaderboard Strategy: Where to Gain Points

The geometric mean rewards lifting the lower components of the top 3:

```
Current Top 3:
  Track A: 50.610 pts
  Track D: 43.550 pts
  Track B: 43.265 pts
```

### Priority 1: Track B (Index) — Highest Marginal Point ROI
- **Current:** $T = 1.071\text{ s}$, $M = 534\text{ MB} \implies 43.265\text{ pts}$.
- **Target:** $T < 0.55\text{ s}$, $M < 350\text{ MB} \implies \mathbf{51.5\text{+ pts}}$.
- **Why:** The reference time is huge ($56.35\text{ s}$), and reference memory is massive ($2,200\text{ MB}$). Cutting wall time to ~0.5s with lower memory yields massive $K$ gains ($K \approx 11.5$).
- **If Track B hits 51.5 pts:** Geometric mean jumps to $(50.610 \times 43.550 \times 51.5)^{1/3} = \mathbf{48.4}$.

### Priority 2: Track D (Traverse) — Second Highest Marginal ROI
- **Current:** $T = 1.512\text{ s}$, $M = 177\text{ MB} \implies 43.550\text{ pts}$.
- **Target:** $T < 0.85\text{ s}$, $M \le 170\text{ MB} \implies \mathbf{50.5\text{+ pts}}$.
- **Why:** In sprint 2, we already dropped CPU user time by 23% (dynamic work scheduling + interleaved CSR). Pushing real wall time under 0.9s will boost $K$ from 7.43 to 10.0+.
- **If Track D hits 50.5 pts:** Geometric mean jumps to $(50.610 \times 50.5 \times 51.5)^{1/3} = \mathbf{50.9}$!

### Priority 3: Track C (Join) — The Dark Horse / Safety Triad Replacement
- **Current:** $T = 0.926\text{ s}$, $M = 88\text{ MB} \implies 32.066\text{ pts}$.
- **Target:** $T < 0.40\text{ s}$, $M < 60\text{ MB} \implies \mathbf{45.0\text{+ pts}}$.
- **Why:** If Track C achieves multi-threaded chunk parsing and hits 45+ points, it can displace Track B or D if either has holdout anomalies, guaranteeing a top podium.

### Priority 4: Track A (Ingest) — Extending Championship Dominance
- **Current:** $T = 0.292\text{ s}$, $M = 61\text{ MB} \implies \mathbf{50.610\text{ pts}}$.
- **Target:** $T < 0.16\text{ s}$, $M < 45\text{ MB} \implies \mathbf{56.0\text{+ pts}}$.

---

## 4. Track Architectures & Concrete Optimization Playbooks

### Track B: Index (`track-b/src/main.rs`)

#### Current Architecture
1. **Ahead-of-Time (AOT) Query Filtering:** Reads `queries.txt` before ingesting `docs.txt`. Only 75,747 queried terms (out of 609,833 total vocabulary) are indexed. 87% of corpus words are skipped.
2. **Parallel Ingest with Monotonic Slices:** Worker threads parse chunks of `docs.txt` along `\n` boundaries. Each thread emits strictly ascending document IDs. Merging thread postings is a linear concatenation without sorting.
3. **Sparse/Dense Left-to-Right Boolean Evaluation:** Evaluates strictly left-to-right. Terms with $df < 256$ use direct array operations ($O(df)$). Larger terms use 64-bit word bitsets (`[u64; 3907]`).
4. **Ascending $df$ Phrase Candidate Filtering:** Phrase terms sorted by ascending document frequency; candidate documents filtered by intersecting the rarest anchor first, then verified in-place using stack-allocated `(u32, u32)` spans with dynamic heap fallback.

#### Concrete Optimization Opportunities for Codex
1. **SIMD Tokenization with AVX2:**
   - In `tokenize_text`, we currently test characters byte-by-byte.
   - Using AVX2 intrinsics (`_mm256_loadu_si256`, `_mm256_cmpeq_epi8`, `_mm256_movemask_epi8`), scan 32 bytes at a time for delimiters (` ` and `\n`) or non-alphanumeric bytes.
2. **Zero-Copy Memory-Mapped / Streaming IO:**
   - Instead of allocating a 254 MB `Vec<u8>` for `docs.txt`, investigate if file streaming or chunked processing reduces peak resident memory ($M$) from 534 MB to ~300 MB.
   - Note: In $K$, memory has exponent 0.20. Reducing $M$ from 534 MB to 320 MB gives $(534/320)^{0.20} = 1.107\times$ (+10.7% boost to $K$!).
3. **Parallel Query Processing Chunk Tuning:**
   - We currently partition 50,000 queries across threads. Benchmark dynamic work distribution or tune thread chunk sizes.
4. **AVX2 Bitset Operations for Boolean Queries:**
   - In `evaluate_boolean`, large bitset intersections use `acc[i] &= op[i]`.
   - Vectorize with `_mm256_and_si256` / `_mm256_or_si256` over 256-bit registers (4 `u64` per instruction), reducing boolean execution cycles by $4\times$.

---

### Track D: Traverse (`track-d/src/main.rs`)

#### Current Architecture
1. **Interleaved 8-byte CSR Graph:** `offsets: Vec<u32>` and contiguous `edges: Vec<EdgeTarget>` where `struct EdgeTarget { to: u32, weight: u32 }` (8 bytes). Single cache-line fetch per edge relaxation.
2. **Epoch-Based Distance Reset:** Thread-local `visited_epoch: Vec<u32>` and `dist: Vec<u64>` of size $N=1.74\text{M}$. Incrementing `cur_epoch` resets search state without allocating or clearing arrays.
3. **Dynamic Work Scheduling:** 2,000 queries dynamically claimed via `AtomicUsize::fetch_add(1, Relaxed)`.
4. **Pruning:** Out-of-bounds guards, `s == t \implies 0`, `out_deg[s] == 0 \lor in_deg[t] == 0 \implies -1`.

#### Concrete Optimization Opportunities for Codex
1. **Thread Count & Affinity Tuning:**
   - We currently use `const NUM_THREADS: usize = 6;`.
   - On the Falkenstein box, there are 6 P-cores with 12 hardware threads (`--cpuset-cpus=0-11`).
   - Test whether `NUM_THREADS = 12` (or 8, or 10) achieves faster wall time than 6 threads, balancing hyperthreading contention vs core utilization.
2. **Fast ASCII Parsing for `graph.txt`:**
   - `graph.txt` is 108 MB (5M lines of `u v w`). Parsing takes ~0.25s of the 1.51s total.
   - Use multi-threaded chunking to parse `graph.txt` in parallel, or custom branchless integer parsing (e.g. `parse_u32` with byte-shift multiplication).
3. **Heap Pop / Sift-Down Unrolling:**
   - In the Dijkstra inner loop, `heap.pop()` and `heap.push()` are called millions of times.
   - Profile `BinaryHeap` vs an inline specialized 2-ary or 4-ary heap that stores `(dist: u64, node: u32)` in a single contiguous flat buffer with branchless sift-up.
4. **Distance Array Cache Footprint Reduction:**
   - `dist: Vec<u64>` for 1.74M nodes = 13.9 MB per thread.
   - Across 6 threads, distance arrays take ~84 MB.
   - If max distance fits in `u32`, or if distances are packed, memory drops and L3 cache pollution is halved. Verify maximum path distance from specifications and generators.

---

### Track C: Join (`track-c/src/main.rs`)

#### Current Architecture
1. **Sliding-Window 2 MB Streaming Buffer:** Streams 202 MB `records.csv` through a 2 MB circular buffer, maintaining peak RSS at 88 MB.
2. **Direct Region Lookup Array:** Flat 10 MB `region_to_country: Vec<u8>` for $O(1)$ foreign key lookup.
3. **Collision-Proof Name Arena (`DistinctNameTable`):** Open-addressing table backed by a contiguous `Vec<u8>` string arena comparing exact raw bytes (`existing == name`).
4. **Exact Financial Cents:** `i64` integer cents for amounts.

#### Concrete Optimization Opportunities for Codex
1. **Multi-Threaded Speculative Chunking:**
   - Currently single-threaded ($T = 0.926\text{ s}$).
   - Because CSV has multiline quoted notes, splitting across threads requires speculative record boundary detection: a line starting with an unquoted integer ID followed by a comma after a newline outside quotes is a valid record start.
   - Splitting `records.csv` across 6 threads can plunge wall time from 0.926s to **<0.30s**, boosting Track C points from 32 to **48+ points**!
2. **AVX2 Quote / Comma Scanning:**
   - In the streaming state machine, use AVX2 vector comparison to find `,`, `"`, `\r`, `\n` across 32 bytes simultaneously.

---

### Track A: Ingest (`track-a/src/main.rs`)

#### Current Architecture
1. **6-Thread Parallel Chunking:** `requests.log` partitioned across newline boundaries.
2. **Deterministic Day/Hour Arithmetic:** Invariant 72-hour window mapped directly to `0..71` slots.
3. **Thread-Local Endpoint Caches:** $O(1)$ read-only cache filtering lookups to avoid mutex locks after warmup.
4. **Nearest-Rank Percentiles:** $O(n)$ `select_nth_unstable` partitioning for $p50, p95, p99$.

#### Concrete Optimization Opportunities for Codex
1. **AVX2 Line / Space Scanning:**
   - Use `_mm256_cmpeq_epi8` to find spaces and newlines in 32-byte blocks.
2. **Lock-Free Flat Endpoint Registry:**
   - Replace the mutex around the global endpoint table with a lock-free open-addressing table using `AtomicU32`.

---

## 5. Verification, Testing & Rules of Engagement

### Ground Rules (Non-Negotiable)
1. **NEVER RUN `git push`:** The human team lead must review and push. Keep all changes committed locally on branch `speed-demon`.
2. **NEVER ADD EXTERNAL CRATES:** `Cargo.toml` must have empty `[dependencies]` ($D = 1.000$).
3. **ZERO COMPILER WARNINGS:** Always enforce `cargo check` clean across dev and release profiles.
4. **BYTE-IDENTICAL CORRECTNESS:** Every change must match the official expected SHA-256 checksum byte-for-byte. A 1-byte discrepancy is an automatic 0 points on the board.
5. **MEASURE AND RECORD:** Log every change and benchmark in the track's `PERF.md`. Never fabricate run metrics.

### Local Verification Commands
To test and verify tracks locally:

```bash
# Check compilation across all tracks
cargo check --manifest-path track-a/Cargo.toml
cargo check --manifest-path track-b/Cargo.toml
cargo check --manifest-path track-c/Cargo.toml
cargo check --manifest-path track-d/Cargo.toml

# Offline Docker sandbox check (the official test harness)
python check.py a --data /path/to/data/a --expected 495ca09682542a981a8ba214a1a659fb221ee54fe66d8ba609fca940aa415ef4
python check.py b --data /path/to/data/b --expected f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927
python check.py c --data /path/to/data/c --expected d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536
python check.py d --data /path/to/data/d --expected a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e
```

### Full Scale Seed 1 Expected Checksums
- **Track A:** `495ca09682542a981a8ba214a1a659fb221ee54fe66d8ba609fca940aa415ef4`
- **Track B:** `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`
- **Track C:** `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536`
- **Track D:** `a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e`

---

## 6. Suggested Immediate Next Task: Sprint on Track B & Track D

Start with **Track B (Index)**:
1. Profile `evaluate_boolean` and `check_phrase_in_doc` under release builds.
2. Implement AVX2 vectorization for bitset intersections (`_mm256_and_si256` / `_mm256_or_si256`) to accelerate boolean queries.
3. Test streaming / memory reduction to lower peak memory below 400 MB.
4. Verify SHA-256 match and record timing in `track-b/PERF.md`.

