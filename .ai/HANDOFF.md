# Speed Demon 2026 — Agent Handoff Protocol & Instructions for Codex (`HANDOFF.md`)

**Target Agent:** Codex (Implementation Engineer)  
**Author:** Antigravity (Lead Systems Architect)  
**Status:** All 4 Tracks Live on Leaderboard (Score: 45.686) — Holdout Hardening & Optimization Pass Complete  

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
   - Always run verification against `tiny`, `sample`, and `full` datasets:
     ```bash
     python check.py <track> --data /path/to/data --expected <sha256>
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

- **Track A (Ingest)**: Official Score: **50.610 pts** ($T = 0.292\text{ s}$, $M = 61.0\text{ MB}$, $K = 10.284873$). 100% byte-identical SHA-256 (`495ca09682...`).
- **Track B (Index)**: Official Score: **43.265 pts** ($T = 1.071\text{ s}$, $M = 534.0\text{ MB}$, $K = 7.333291$). 100% byte-identical SHA-256 (`f86973c210...`).
- **Track C (Join)**: Official Score: **32.066 pts** ($T = 0.926\text{ s}$, $M = 88.0\text{ MB}$, $K = 4.378513$). 100% byte-identical SHA-256 (`d8088b1cd4...`). Hardened with `DistinctNameTable` arena.
- **Track D (Traverse)**: Official Score: **43.550 pts** ($T = 1.512\text{ s}$, $M = 177.0\text{ MB}$, $K = 7.430230$). 100% byte-identical SHA-256 (`a23dbdf0a3...`). Hardened with dynamic atomic scheduling and interleaved 8-byte edge CSR.

---

## 3. Current Staged Work & Next Actions

- **Task Status:** Sprint 2 (Holdout Hardening & Leaderboard Synchronization) complete across all tracks.
- **Verified Code Changes:**
  - `track-b/src/main.rs`: Holdout phrase span safety `(u32, u32)` with stack/heap fallback; branchless static ASCII lookup tables; intermediate memory reclamation.
  - `track-c/src/main.rs`: Exact byte deduplication arena `DistinctNameTable` guaranteeing holdout collision immunity.
  - `track-d/src/main.rs`: Dynamic atomic work pool `AtomicUsize`; interleaved 8-byte edge target CSR; node query bounds and epoch rollover safety.
  - Documentation, README, and `PERF.md` files updated across all tracks.
- **Next Action:** Request user authorization to commit and push to `origin/speed-demon`.
