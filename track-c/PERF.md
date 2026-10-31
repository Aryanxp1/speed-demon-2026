# Track C: Performance Log

Every step from the reference to our optimized version, with measured metrics, architectural rationale, and verified SHA-256 correctness.

| Step | Change | Run ID / Context | T (s) | M (MB) | S (lines) | K | Points |
|---|---|---|---|---|---|---|---|
| **0** | Official scoring box Python reference | `calibration-seed1.json` | 14.44 | 367.0 | 157 | 1.000 | 0.00 |
| **1** | Single-threaded zero-dependency Rust with full-file ingest and `HashSet<Vec<u8>>` distinct tracking. | Unofficial local | 2.74 | 380.0 | 340 | 3.250 | 25.60 |
| **2** | Direct 64-bit FxHash distinct tracking (`HashSet<u64>`) eliminating 100,000 heap allocations. Pre-allocated buffers. | Unofficial local | 1.24 | 240.1 | 350 | 4.310 | 31.70 |
| **3** | Sliding-window chunked streaming (2 MB buffer). Slashes peak RSS from 240 MB to 45.4 MB ($8.1\times$ leaner than reference) and improves wall time to 1.20s due to L3 cache locality. | Unofficial local | 1.20 | 45.5 | 367 | 4.630 | 33.30 |
| **4** | **Official Board Submission (Baseline v1.0)**: Evaluated on official scoring machine (Intel Core i5-13500, pinned 12 threads, 8 GB RAM). 100% byte-identical SHA-256 match. | **Official Board Run** | **0.926** | **88.0** | **367** | **4.378513** | **32.066** |
| **5** | **64-Byte Carry-less Multiply (CLMUL) Structural Quote Masking & Lock-Free Concurrent NameTable**: Direct zero-copy `mmap` with 4096-byte padding; 64-byte blocks processed with `_mm_clmulepi64_si128` prefix XOR quote masking to find commas and newlines in hardware; $O(1)$ arithmetic string length computation `(e - s) - (q ? q / 2 + 1 : 0)`; lock-free concurrent `NameTable` with 16-bit tags and 24-byte inline `head` comparison via `_mm256_cmpeq_epi8`; shared bitset `NamesSeen` marked in a single `fetch_or` and popcounted per country. | Unofficial local WSL | 0.25 | 207 | ~28.2 | ~72.5 |

---

### Step 1: Zero-Dependency Rust Baseline
- **Motivation:** Replace Python's interpreted string decoding, regex parsers, and dict lookups with direct byte scanning and array lookups.
- **Approach:**
  - Direct 10 MB lookup array `region_to_country: Vec<u8>` mapping `region_id` ($0 \dots 9,999,999$) to country index in $O(1)$ time with zero hash collisions.
  - Custom byte-level CSV state machine handling mid-file UTF-8 BOMs (`\xEF\xBB\xBF`), quoted commas, escaped `""`, and multiline notes.
  - Exact integer arithmetic for amounts (cents stored as `i64`).
- **Result:** $T = 2.74\text{ s}$, $M = 380\text{ MB}$. Output SHA-256 byte-identical.

### Step 2: 64-Bit FxHash Distinct Name Tracking
- **Motivation:** Storing 100,000 distinct `Vec<u8>` names in `HashSet` caused severe allocator fragmentation and ballooned RSS.
- **Approach:**
  - Replace `HashSet<Vec<u8>>` with `HashSet<u64>` keyed by a fast 64-bit FxHash.
  - Zero collisions observed across all datasets.
- **Result:** Wall time improved by $2.2\times$ ($2.74\text{ s} \to 1.24\text{ s}$), peak RSS dropped from 380 MB to 240 MB.

### Step 3: Sliding-Window Chunked Streaming (2 MB Buffer)
- **Motivation:** In the Speed Demon formula, memory carries a $0.20$ exponent. Reading the full 202 MB file into memory created an avoidable memory footprint.
- **Approach:**
  - Implemented a sliding window 2 MB buffer. When remaining bytes in the buffer fall below `MAX_RECORD_LEN` (64 KB), unparsed tail bytes are shifted to the front and the next 2 MB chunk is read from disk.
  - Eliminates the 202 MB file buffer allocation entirely.
  - Keeps working set entirely within L3 CPU cache (24 MB on Falkenstein i5-13500).
- **Result:** Peak RSS plunged from 240 MB to **45.5 MB** ($8.08\times$ leaner than reference 367 MB). Wall time clocked at **1.20 s**. Local $K = 4.63$.

### Step 4: Official Board Verification & Metrics
- **Environment:** Dedicated official benchmark host (Intel i5-13500, 6 P-cores @ 2.5 GHz turbo off, 8 GB memory limit, Linux offline container).
- **Official Result:**
  - $T = \mathbf{0.926\text{ s}}$ ($15.6\times$ faster than 14.44s reference).
  - $M = \mathbf{88.0\text{ MB}}$ ($4.17\times$ leaner than 367.0 MB reference).
  - $K = \mathbf{4.378513}$.
  - **Points Earned:** $\mathbf{32.066}$ machine points.
- **Correctness:** 100% SHA-256 match on full public dataset (`d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536`).

### Step 5: Exact-Byte Collision-Proof Verification (Holdout Hardening)
- **Motivation:** Hash-based distinct name tracking carries a non-zero probability of birthday-paradox collisions on unseen holdout datasets with unknown name distributions.
- **Approach:**
  - Replaced the hash set with `DistinctNameTable`, an open-addressing hash table storing `(country_id, offset, len)` records backed by a contiguous `Vec<u8>` string arena.
  - On hash hit, compares exact slice bytes `&arena[off..off+len] == name`.
  - Dynamically resizes at 50% load factor.
- **Result:** Complete collision immunity guaranteed on any holdout dataset. Retains lean memory footprint (~90 MB) and sub-second wall time. 100% verified across tiny, sample, full, seed 42, and seed 999.


## 2026-10-31 — Lex: repair `22347ff` correctness

**Correction to the historical Step 5 claims above:** the CLMUL implementation at `22347ff` fails local tiny, sample and full correctness. Its prior estimated score and correctness claims do not validate that commit. The older arena implementation described separately above is not the current implementation. No new official score has been measured.

The reproduced primary failure was final-region parsing: a trailing LF caused the parser to read after the final row and allocate a region map of length one. All records became `_UNMATCHED`. The fix finds the last nonempty row. Existing region chunk splitting already aligns to LF; boundary tests cover 1–31 workers. Additional fixes initialize amount extrema correctly and inspect a competing writer's published name after a failed compare-exchange, preventing duplicate IDs for equal names.

The fast name table and ID bitmap capacity are now 524,288. Bounded probing spills into exact-byte standard-library maps/sets if capacity is exhausted. Long-name suffixes reference the immutable input mapping, removing the unchecked 8 MiB arena. AVX2/PCLMUL target attributes and Docker flags are explicit; unsupported CPUs receive an error. The Linux THP-disable prctl constant is corrected. These changes improve robustness without claiming unlimited holdout capacity or performance.

Local environment: WSL2, i7-12700H, CPUs 0–11, Linux musl release, x86-64-v3 plus PCLMUL. RSS includes mapped input pages; it is not equivalent to official container memory accounting.

| Dataset / version | Wall time (s) | Peak RSS (KiB) | Correctness |
|---|---:|---:|---|
| Tiny baseline | .02 | 1796 | FAIL |
| Sample baseline | .06 | 25228 | FAIL |
| Full baseline | .41 | 202748 | FAIL |
| Tiny fixed | .01 | 1996 | PASS |
| Sample fixed | .05 | 41832 | PASS |
| Full fixed, five serial runs | .15, .06, .06, .06, .06 | 220588, 220672, 220736, 220416, 220736 | PASS every run |

Fixed full trimmed mean: .060 s, maximum RSS: 226.03 decimal MB. An earlier verification run took .25 s / 220608 KiB; cache and scheduling effects are substantial. Invalid baseline timings are recorded for diagnosis only, not a valid speedup comparison. The fixes were measured as one correctness repair, not isolated performance optimizations.

| Dataset | Verified SHA-256 |
|---|---|
| Tiny | `e4e43bc39920e9d446c3fcd700fdd35b3959e9495c6aaa738491df38a1aed733` |
| Sample | `9035fed36823232dd41ae2d1b9d6d21aa7c73f49d8e95647e953a89d5ebe9a66` |
| Full | `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536` |
| Seed 42 tiny | `7d151bcd945ddf65dcddaa4b956f75435d638a411bddb7cdb42f4fb7e2d8e816` |
| Seed 999 sample | `62e7f211aa5d71f593a5fa718e0ca8279b71fd1b716d0879112d550c6d3ab77d` |

Seed 42 and 999 initial verification measurements: .02 s / 1960 KiB and .05 s / 42240 KiB respectively. Both match reference output byte for byte. Official WORKED and RULES handwritten fixtures also pass, covering quoted fields, BOMs, multiline notes and amount extrema. Three new unit tests pass for region boundaries/extrema, concurrent deduplication, and exhausted-table fallback. Dev/release checks deny warnings. Docker validation and the live board rerun remain outstanding; the 72+ target is unverified. Local scripts and binaries are in ignored `.data/lex_directive`, measurements in `/home/aryan/speeddemon_data/lex-directive`.
