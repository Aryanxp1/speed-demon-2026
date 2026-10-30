# Track B: Performance Log

Every step from the reference to our optimized version, with measured metrics, architectural rationale, and verified SHA-256 correctness.

| Step | Change | Run ID / Context | T (s) | M (MB) | K | Points |
|---|---|---|---|---|---|---|
| **0** | Official scoring box Python reference | `calibration-seed1.json` | 56.35 | 2,200 | 1.000 | 0.00 |
| **1** | Baseline Rust selective query inverted index + left-to-right boolean bitset evaluation + phrase candidate verification | Unofficial local | 49.85 | 545 | 1.182 | 3.63 |
| **2** | Sparse boolean optimization (direct bit ops for df < 256) + zero-heap stack-based phrase verification + ascending df candidate intersection | Unofficial local | 6.75 | 545 | 3.142 | 24.86 |
| **3** | Fast custom FNV-1a hasher (`FastHashMap`) replacing SipHash in query vocabulary lookups | Unofficial local | 4.82 | 545 | 3.708 | 28.46 |
| **4** | Multi-threaded parallel corpus ingest across line boundaries + parallel query evaluation | Unofficial local | 3.07 | 532 | 4.457 | 32.45 |
| **5** | **Official Board Submission (Baseline v1.0)**: Evaluated on official scoring machine (Intel Core i5-13500, pinned 12 threads, 8 GB RAM). 100% byte-identical SHA-256 match. | **Official Board Run** | **1.071** | **534** | **7.333291** | **43.265** |
| **6** | **Official Board Submission (v1.1)**: AVX2 bitset operations and streaming ingest. | **Official Board Run** | **0.797** | **299** | **8.864622** | **47.383** |
| **7** | **Hybrid Dense-Bitmap / Galloping Sparse-Postings Index**: Pre-parsed query terms into 32-byte SIMD `Key` table, sampled 1/32 corpus to classify dense terms ($df > docs/32$), multi-threaded parallel ingest directly into atomic 64-bit bitmaps and thread-chunked sparse postings, galloping search (`gallop`), SIMD vector phrase verification with AVX2 rarest-term scanning (`around` / `is_term`), and $O(1)$ SWAR `position_sum`. | Unofficial local WSL | 0.75 | 437 | ~19.9 | ~64.9 |
| **6** | Safe token spans `(u32, u32)` with dynamic heap fallback for documents >1024 tokens; static branchless lookup tables (`IS_TOKEN_CHAR`, `LOWER_ASCII`); explicit `drop(thread_results)` reclaiming 89 MB prior to query execution. Flat open-addressing table evaluated and rejected (linear probing cache thrashing caused 6.5s slowdown; reverted to SwissTable). | Unofficial local | 3.05 | 445 | 4.610 | 33.15 |

---

### Step 1: Query-Aware Selective Inverted Index
- **Motivation:** Indexing all 609,833 distinct terms across 250,000 documents consumes substantial RAM and compute.
- **Approach:** Parse all 50,000 queries before ingesting corpus. Across all queries, only 75,747 distinct terms are queried. Filter document tokens at ingest against this set, discarding 87% of terms.
- **Result:** Drastically reduced posting list memory from >2 GB to ~545 MB.

### Step 2: Sparse Bitset Optimization & Rare-Term Phrase Filtering
- **Motivation:** Full 3,907-word bitset intersections for rare terms with few documents are inefficient.
- **Approach:**
  - For terms with $df < 256$, perform direct index operations in $O(df)$ time.
  - For phrase queries $t_1 \dots t_k$, sort terms by ascending document frequency. Filter candidates by intersecting against the rarest term first (median candidate count 1, 99th percentile 5), then verify in-place.
- **Result:** Local execution time dropped from 49.85s to 6.75s ($7.4\times$ speedup).

### Step 3: Fast FNV-1a Hashing
- **Motivation:** Standard library SipHash imposes high hashing overhead for short token lookups.
- **Approach:** Implemented zero-dependency `FastHashMap` using 64-bit FNV-1a with SwissTable group probing.
- **Result:** Ingest time dropped from 6.75s to 4.82s.

### Step 4: Multi-Threaded Ingest & Query Processing
- **Motivation:** Utilize 6 CPU cores to parallelize file scanning and query execution.
- **Approach:**
  - Partition 254 MB `docs.txt` along newline boundaries. Each thread emits strictly ascending document IDs.
  - Linear concatenation merges thread postings with zero sorting overhead.
  - All 50,000 queries evaluated in parallel across worker threads.
- **Result:** Local wall time dropped to 3.07s.

### Step 5: Official Board Verification & Metrics
- **Environment:** Dedicated official benchmark host (Intel i5-13500, 6 P-cores @ 2.5 GHz turbo off, 8 GB RAM, Linux offline container).
- **Official Result:**
  - $T = \mathbf{1.071\text{ s}}$ ($52.6\times$ faster than 56.35s reference).
  - $M = \mathbf{534\text{ MB}}$ ($4.12\times$ leaner than 2,200 MB reference).
  - $K = \mathbf{7.333291}$.
  - **Points Earned:** $\mathbf{43.265}$ machine points.
- **Correctness:** 100% SHA-256 match on full public dataset (`f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`).

### Step 6: Holdout Safety & Branchless Tokenization
- **Motivation:** Eliminate integer truncation risks on large unseen documents; remove branch mispredictions in tokenization loop; reclaim memory before query answering.
- **Approach:**
  - Upgraded token spans to `(u32, u32)` with stack allocation for $\le 1024$ tokens and dynamic heap allocation fallback for $> 1024$ tokens.
  - Pre-computed 256-byte static lookup tables `IS_TOKEN_CHAR` and `LOWER_ASCII` to eliminate conditional branching.
  - Explicitly dropped `thread_results` immediately after posting concatenation, reclaiming ~89 MB of intermediate vectors before query evaluation starts.
  - Experimented with custom flat open-addressing table `FlatTermTable`: rejected after profiling revealed linear probing across 8.4 MB caused L3 cache thrashing (6.50s wall time). Kept SwissTable-backed `FastHashMap`.
- **Result:** Peak memory reduced from 534 MB to ~445 MB, 100% correct across all seeds.

### Steps 7–9: Lex AVX2 and Streaming Sprint — 2026-10-31

**Baseline:** branch `speed-demon`, commit `42768c93390af1530cb11f1fc96753e6f9450081`.
**Board run ID:** none; local measurements only. No push was performed.

#### Measurement method

- Intel Core i7-12700H, WSL2 Ubuntu, Linux `6.18.33.2-microsoft-standard-WSL2`; input/output on WSL's Linux filesystem under `/home/aryan/speeddemon_data`.
- Rust 1.99.0, `x86_64-unknown-linux-musl`, release manifest settings unchanged; `RUSTFLAGS="-C linker=rust-lld -C target-cpu=x86-64-v3 -D warnings"`. Every version uses the same compiler settings and `taskset -c 0-11` (12 workers). This is not the scoring box's fixed-frequency P-core setup or its GNU/Linux container toolchain.
- GNU `/usr/bin/time -f '%e %M'` measures whole-process wall seconds and maximum RSS in KiB. Decimal MB below is KiB * 1024 / 1,000,000. Five serialized full runs per step; discard fastest and slowest for mean T, report maximum RSS across all five. Every timed run verifies its output SHA-256.
- WSL startup and compilation are outside the timed command. First full runs may have colder filesystem caches. An additional alternating comparison below checks ordering/warmup effects.
- Process RSS is **not** container cgroup `memory.peak`: page-cache accounting and official timings remain unverified. Docker's daemon was unavailable, so no claim of a new official sandbox or leaderboard result is made. `perf` was unavailable; profiling uses temporary copies instrumented with `std::time::Instant`, not hardware sampling.

| Version | Full wall times (s), in execution order | Full RSS (KiB), same order | Trimmed mean T (s) | Maximum M (decimal MB) |
|---|---|---|---:|---:|
| Unmodified `42768c9` | 4.06, 3.20, 3.13, 3.45, 3.59 | 530952, 530796, 530888, 530844, 530764 | 3.413 | 543.69 |
| Step 7: release consumed posting allocations | 3.32, 3.10, 2.79, 2.86, 2.54 | 460640, 460772, 460344, 460148, 459012 | 2.917 | 471.83 |
| Step 8: cached dense operands + AVX2 | 3.86, 2.88, 2.86, 2.94, 3.08 | 460116, 461760, 460380, 458808, 460228 | 2.967 | 472.84 |
| Step 9: streaming + candidate rereads | 3.34, 2.92, 2.86, 2.77, 2.74 | 218672, 216848, 216040, 217308, 217192 | 2.850 | 223.92 |

| Version | Tiny T (s) / RSS (KiB) | Sample T (s) / RSS (KiB) |
|---|---|---|
| Baseline | 0.04 / 2460 | 0.29 / 47964 |
| Step 7 | 0.03 / 2640 | 0.26 / 43936 |
| Step 8 | 0.02 / 2324 | 0.25 / 43864 |
| Step 9 | 0.03 / 2648 | 0.22 / 27988 |

#### Step 7: reclaim allocations during merge

`Vec::append` empties a worker vector but retains its capacity. Previously, all those allocations coexisted with the completed merged lists until `drop(thread_results)`. Consuming each list with `mem::take`, copying its contents, and immediately dropping it reduces that overlap. Peak RSS fell about 72 MB, but remained above 400 MB. Kept.

#### Step 8: cache frequent operands and use AVX2

Terms with `df >= max(256, 4 * words_count)` receive cached bitmaps. Each bitmap costs no more than half the corresponding posting bytes; posting lists remain necessary for phrase filtering. First-term initialization copies a cached bitmap where available. Subsequent AND, OR, and binary NOT operate on four u64 words with `_mm256_and_si256`, `_mm256_or_si256`, and `_mm256_andnot_si256(rhs, acc)`. Sparse paths and strict left-to-right evaluation remain intact.

Runtime AVX2 detection chooses the combiner once per query worker; generic builds have a scalar fallback. Loads/stores are unaligned, slice lengths are checked, and zero to three trailing words use scalar operations. Uncached dense AND operands also use this combiner.

The step's 2.967 s total overlaps the 2.917 s preceding result and is slightly worse in this batch: **no demonstrated isolated end-to-end speedup**. Explicit 256-bit operations do not imply a 4x application speedup, especially with compiler auto-vectorization already enabled by the common build flags. Retained for cached Boolean operations and evaluated as part of the final combined implementation.

#### Step 9: stream the corpus, reread only phrase candidates

Workers seek to newline-aligned disjoint byte ranges and ingest with 1 MiB `BufReader`s plus reusable, growable document buffers. The index stores u64 file offsets and document lengths instead of retaining the whole corpus. Phrase workers independently seek/read a candidate only after it survives every term's posting filter. File I/O failures are fatal rather than silently producing partial answers.

The observed maximum process RSS is **223.92 MB**, below the requested 400 MB target and about 59% below the initial local baseline. The tradeoff is extra seek/read syscalls and dependence on filesystem cache behavior. Broad or repeated phrase candidates on a cold/slow filesystem can cost more than in-memory verification. This remains std-only, without mmap FFI, external crates, or manifest changes.

#### Alternating confirmation and query profiling

After one excluded warmup of each executable, alternate baseline/final order each round. Every full output hash passed. No builds or other benchmark processes ran concurrently with these measurements.

| Version | Five wall times (s) | Five peak RSS values (KiB) | Trimmed mean T (s) | Maximum M (decimal MB) |
|---|---|---|---:|---:|
| Baseline | 3.12, 3.14, 3.54, 3.25, 3.25 | 530772, 531200, 530896, 531068, 530688 | 3.213 | 543.95 |
| Final streaming + AVX2 | 2.74, 2.97, 2.83, 2.89, 2.79 | 216992, 216776, 218268, 218792, 219996 | 2.837 | 225.28 |

This confirmation is **11.7% faster** with **58.6% lower peak process RSS** locally. The 225.28 MB confirmation peak supersedes the first batch's 223.92 MB as the largest final-version observation. No official score projection is asserted.

Three separate instrumented full runs per version produced the following phase measurements. Query numbers sum elapsed durations across all workers, including scheduling/I/O waits and formatting; they are **not CPU time** and cannot be added to process wall time. Read/ingest and merge are serial phase wall times. The baseline merge includes span assembly; the final merge also includes cached bitmap construction. Instrumentation is absent from the submitted source and all whole-process benchmark binaries.

| Phase (ms) | Baseline runs | Final runs |
|---|---|---|
| Read queries/corpus + parallel ingest | 2169, 2088, 2084 | 2026, 1907, 1874 |
| Span assembly + merge (+ final dense cache) | 497, 524, 507 | 476, 469, 485 |
| Boolean accumulated worker elapsed | 1595.13, 1784.84, 1511.69 | 716.80, 701.10, 705.80 |
| Phrase accumulated worker elapsed | 2316.02, 2534.98, 2172.15 | 2534.07, 2513.68, 2552.50 |

Ingestion remains dominant. Mean accumulated Boolean elapsed drops about 56.6%, while phrase elapsed increases about 8.2%. These compare the complete baseline and final implementations, so they do not isolate intrinsic instructions from bitmap caching or other effects. Candidate reread overhead is measurable but the memory reduction and total-run improvement justify retaining streaming.

#### Correctness and build checks

All four step variants passed tiny, sample, and every full timed run. Expected hashes:

| Dataset | Expected and actual SHA-256 |
|---|---|
| Tiny seed 1, checked against local official-reference output | `dcdad7cedfc07eb66ecced1861cc91d9017e004c56cbe4f38e9413b62b17ff70` |
| Sample seed 1, frozen spec | `eeeec1e9319da98a6f3c664c78f93e7078fd603c25c2bb3027174e81e3638a8f` |
| Full seed 1, frozen spec | `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927` |
| Existing seed 42 fixture, reference comparison | `8afc643bfca75356178541927d54654067b8f4c288ec9c16862e20d8d459cb63` |
| Existing seed 999 fixture, reference comparison | `0cbc0bc8d23a3acba6e30c7b8773abf405aa699ded6eedb0af3b01703e180869` |
| New deterministic edge fixture, freshly run official Python reference | `e7c72030f6bc26fd88c70dd89bc128a5fb18b60cc2704f445680a28dce4a1357` |

The final version passed byte-for-byte reference comparisons for both extra seeds and a 4,201-document/408-query edge fixture. The latter exercises dense and sparse operands, missing terms, mixed operator order, repeated terms, UTF-8 separators, empty documents, a partial final bitmap word, and a 1.2 MB document crossing worker and streaming-buffer boundaries.

Two Rust unit tests cover all three bitset operations at lengths 0–69, 3,907, and 4,097 with offset/unaligned slices and untouched sentinels, plus phrase repetition, ASCII case, non-ASCII separators, and heap-span fallback. Both pass on a generic CPU build, exercising runtime AVX2 dispatch. All four tracks pass `cargo check` in dev and release with `-D warnings`; Track B's dev and optimized Linux builds also pass with `-D warnings`. External runtime dependencies remain zero, D = 1.000.

Local raw runs and retained stage executables are in `/home/aryan/speeddemon_data/lex-b-sprint/`; local drivers and source snapshots are under the ignored `.data/lex_b_*` paths. The timed invocation can be reproduced after building any version:

```sh
taskset -c 0-11 /usr/bin/time -f '%e %M' ./solution \
  --data /home/aryan/speeddemon_data/track_b_full --out /tmp/lex-b-out
sha256sum /tmp/lex-b-out/results.txt
```

