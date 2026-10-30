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

