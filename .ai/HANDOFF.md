# Speed Demon 2026 — Agent Handoff Protocol & Instructions for Codex (`HANDOFF.md`)

**Target Agent:** Codex (Implementation Engineer)  
**Author:** Antigravity (Lead Systems Architect)  
**Status:** Active Competition Execution — 3-Track Triad Complete; Staging Track B (Index)  

---

## 1. Operating Rules for Codex

When implementing any task assigned by Antigravity:

1. **Strictly Respect Scope:**
   - Only edit files explicitly listed under `Files Owned` in the active task.
   - Do not touch `.ai/` governance files (Antigravity maintains these).
   - Do not touch other track directories while working on a designated track.
2. **Zero-Dependency Mandate ($D=1.0$):**
   - Do not add any dependencies to `Cargo.toml`.
   - Use only the Rust standard library (`std`).
   - Concurrency must use `std::thread::scope` or `std::sync` primitives.
3. **Correctness Before Speed:**
   - Always run verification against `tiny` and `sample` datasets first:
     ```bash
     python check.py <track> --data .data/<track>/sample-seed1 --expected <sha256>
     ```
   - If the SHA-256 differs by even one byte, the task is **NOT COMPLETE**. Debug immediately.
4. **Log Benchmark Measurements:**
   - Run local benchmarks before and after every optimization step.
   - Report the exact wall time ($T$), memory footprint ($M$), and line count ($S$) to Antigravity so that `PERF.md` can be updated.
5. **Clean Code & Self-Explanatory Logic:**
   - Human judges account for **35% of the score** (Code Quality 15%, Technique 12%).
   - Avoid obfuscation. Comment complex bitwise operations, SIMD intrinsics, or pointer arithmetic.

---

## 2. Completed Milestones

- **Phase 1: Track A (Ingest)**: Verified $K = 7.39$, 100% byte-identical SHA-256 (`495ca09682...`), $T = 0.65\text{ s}$, $M = 54.6\text{ MB}$.
- **Phase 2: Track D (Traverse)**: Verified $K = 4.79$, 100% byte-identical SHA-256 (`a23dbdf0a3...`), $T = 2.71\text{ s}$, $M = 170.8\text{ MB}$.
- **Phase 3: Track C (Join)**: Verified $K = 4.63$, 100% byte-identical SHA-256 (`d8088b1cd4...`), $T = 1.20\text{ s}$, $M = 45.5\text{ MB}$.

---

## 3. Active Assignment: `TASK-B1` (Track B Inverted Index Engine)

- **Track:** B (Index)
- **Status:** `READY_FOR_DEV`
- **Files Owned:**
  - `track-b/src/main.rs`
  - `track-b/Cargo.toml`
  - `track-b/Dockerfile`
  - `track-b/.speed-demon.toml`
- **Inputs:**
  - `/data/corpus.txt` (~254 MB, ~1,000,000 documents).
  - `/data/queries.txt` (50,000 queries: single terms, boolean AND/OR/NOT, exact phrases).
- **Outputs:**
  - `/out/results.txt` (Document IDs matching queries, sorted).
- **Specification Invariants:**
  1. *Ahead-of-Time Query Vocabulary Filter:*
     - Read `/data/queries.txt` FIRST.
     - Collect set of query terms and phrases.
     - While indexing `corpus.txt`, ignore terms that never appear in any query! (Massive memory reduction: index only query vocabulary).
  2. *Compact Postings / Bitsets:*
     - Documents are indexed dense $0 \dots N-1$.
     - Flat posting lists of `u32` document IDs, or bitset vectors (`Vec<u64>`) for fast bitwise AND / OR / NOT.
  3. *Zero External Crates ($D = 1.000$):*
     - No external search engine or indexing crates.
