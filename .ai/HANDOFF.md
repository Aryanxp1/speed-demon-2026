# Speed Demon 2026 — Agent Handoff Protocol & Instructions for Codex (`HANDOFF.md`)

**Target Agent:** Codex (Implementation Engineer)  
**Author:** Antigravity (Lead Systems Architect)  
**Status:** All 4 Tracks Live on Official Leaderboard (Rank 10, Score: **79.476**)  

---

## 1. Operating Rules for Codex

When implementing any task assigned by Antigravity:

1. **Strictly Respect Scope:**
   - Only edit files explicitly listed under `Files Owned` in the active task.
   - Do not touch `.ai/` governance files unless instructed.
   - Do not touch other track directories while working on a designated track.
2. **Zero-Dependency Mandate ($D=1.0$):**
   - Do not add any external runtime dependencies (`dependency_count = 0`).
   - Use only language standard libraries and platform intrinsics.
3. **Correctness Before Speed:**
   - Always run verification against `tiny`, `sample`, and `full` datasets:
     ```bash
     python check.py <track> --data /path/to/data --expected <sha256>
     ```
   - If the SHA-256 differs by even one byte, the task is **NOT COMPLETE**. Debug immediately.
4. **Log Benchmark Measurements:**
   - Run local benchmarks before and after every optimization step.
   - Report the exact wall time ($T$), memory footprint ($M$), and line count ($S$) so that `PERF.md` can be updated.
5. **Clean Code & Self-Explanatory Logic:**
   - Human judges evaluate Code Quality and Technique.
   - Avoid obfuscation. Comment complex bitwise operations, SIMD intrinsics, or pointer arithmetic.

---

## 2. Completed Milestones

- **Track A (Ingest)**: Official Score: **84.737 pts** ($T = 0.0169\text{ s}$, $M = 20.90\text{ MB}$, $K = 49.516286$, commit `60cb66c`). 100% byte-identical SHA-256 (`495ca09682...`).
- **Track C (Join)**: Official Score: **80.470 pts** ($T = 0.0281\text{ s}$, $M = 8.73\text{ MB}$, $K = 40.681817$, commit `d752aea`). 100% byte-identical SHA-256 (`d8088b1cd4...`). Target 80+ reached!
- **Track D (Traverse)**: Official Score: **73.621 pts** ($T = 0.0987\text{ s}$, $M = 62.05\text{ MB}$, $K = 29.676927$, commit `f94eb34`). **#1 by K across the entire leaderboard!**
- **Track B (Index)**: Official Score: **73.103 pts** ($T = 0.1728\text{ s}$, $M = 47.48\text{ MB}$, $K = 28.977962$, commit `2c9f85b`).
- **Overall Geometric Mean:** $\sqrt[3]{84.737403 \times 80.470017 \times 73.620947} = \mathbf{79.476 \text{ points}}$ (**Rank 10 Global**).
