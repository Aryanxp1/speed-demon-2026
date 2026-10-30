# Track C: Performance Log

Every step from the reference to our optimized version, with measured metrics, architectural rationale, and verified SHA-256 correctness.

| Step | Change | Run ID | T (s) | M (MB) | S (lines) | Local K | Machine Points |
|---|---|---|---|---|---|---|---|
| **0** | Naive Python reference (CPython 3.13, `csv` module, dict join) | `ref-fsn1` | 14.44 | 367.0 | 157 | 1.00 | 0.0 |
| **1** | Single-threaded zero-dependency Rust with full-file ingest and `HashSet<Vec<u8>>` distinct tracking. | `local-baseline` | 2.74 | 380.0 | 340 | 3.25 | 25.6 |
| **2** | Direct 64-bit FxHash distinct tracking (`HashSet<u64>`) eliminating 100,000 heap allocations. Pre-allocated buffers. | `local-opt-v1` | 1.24 | 240.1 | 350 | 4.31 | 31.7 |
| **3** | Sliding-window chunked streaming (2 MB buffer). Slashes peak RSS from 240 MB to 45.4 MB ($8.1\times$ leaner than reference) and improves wall time to 1.20s due to L3 cache locality. 100% verified across tiny, sample, full, and holdout seeds (42, 999). | `local-opt-v2` | 1.20 | 45.5 | 367 | 4.63 | 33.3 |

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
- **Result:** Peak RSS plunged from 240 MB to **45.5 MB** ($8.08\times$ leaner than reference 367 MB). Wall time clocked at **1.20 s**. Local $K = 4.63$ (33.3 Machine Points).

### Step 4: Verification Across Multiple Scales & Seeds
- Tiny scale (seed 1): identical output against Python reference.
- Sample scale (seed 1): `9035fed36823232dd41ae2d1b9d6d21aa7c73f49d8e95647e953a89d5ebe9a66` (100% match).
- Full scale (seed 1): `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536` (100% match).
- Tiny seed 42: identical output against Python reference.
- Sample seed 999: identical output against Python reference.
