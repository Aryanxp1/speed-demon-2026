# Speed Demon 2026 — Task Board (`TASK_BOARD.md`)

**Current Sprint:** All 4 Tracks Complete — Leaderboard Evaluation & Final Tuning  
**Competition Status:** Hour 9 of 72  

---

## Task Overview & Pipeline

```
[Phase 0: Pre-Kickoff Prep]
   │
   ├── TASK-01: Establish .ai Context & Verification Harness [DONE]
   └── TASK-02: Generate Baseline Datasets & Validate Tooling [DONE]
   │
[Phase 1: Track A — Ingest]
   │
   ├── TASK-A1: Zero-Dependency Rust Parser & Single-Threaded Core [DONE]
   ├── TASK-A2: 6-Thread Parallel Chunk Ingest & Lock-Free Thread Caching [DONE]
   ├── TASK-A3: Nearest-Rank Percentiles & Fast Number Formatting [DONE]
   └── TASK-A4: Verified Multi-Seed & Multi-Scale Correctness Suite [DONE]
   │
[Phase 2: Track D — Traverse]
   │
   ├── TASK-D1: Compact CSR Graph & Fast Single-Pass ASCII Ingest [DONE]
   ├── TASK-D2: Reachability Pruning & Epoch Distance Resets [DONE]
   ├── TASK-D3: 6-Thread Query Parallelism & QuadHeap/Bidir Experiments [DONE]
   └── TASK-D4: Track D Multi-Scale & Multi-Seed Verification (Tiny, Sample, Full, 42, 999) [DONE]
   │
[Phase 3: Track C — Join]
   │
   ├── TASK-C1: Hostile CSV State Machine Parser (BOMs, Multiline, Escapes) [DONE]
   ├── TASK-C2: Direct Region Lookup & Integer Cents Aggregation [DONE]
   ├── TASK-C3: 64-bit FxHash Distinct Tracking & 2 MB Sliding-Window Buffer [DONE]
   └── TASK-C4: Track C Multi-Scale & Multi-Seed Verification (Tiny, Sample, Full, 42, 999) [DONE]
   │
[Phase 4: Track B — Index (4th Track Completed)]
   │
   ├── TASK-B1: Ahead-of-Time Query Vocabulary Filter [DONE]
   ├── TASK-B2: Parallel Corpus Ingest with Monotonic Slices [DONE]
   ├── TASK-B3: Sparse & Dense Left-to-Right Bitset Boolean Evaluation [DONE]
   ├── TASK-B4: Zero-Heap Stack-Based In-Place Phrase Verification [DONE]
   └── TASK-B5: Multi-Scale & Multi-Seed Verification (Tiny, Sample, Full, 42, 999) [DONE]
   │
[Phase 5: Final Tuning & Submission] (CURRENT FOCUS)
   │
   └── TASK-F1: Multi-Track Final Regression & Public Freeze Readiness [ACTIVE]
```

---

## Active Task: `TASK-F1` (All 4 Tracks Complete & Push to Remote)

- **Track:** All Tracks (A, B, C, D)
- **Status:** `IN_PROGRESS`
- **Portfolio Summary:**
  - Track A: $T = 0.65\text{ s}$, $M = 54.6\text{ MB}$, $K = 7.39$ (43.4 pts)
  - Track B: $T = 3.07\text{ s}$, $M = 532.0\text{ MB}$, $K = 4.46$ (32.5 pts)
  - Track C: $T = 1.20\text{ s}$, $M = 45.5\text{ MB}$, $K = 4.63$ (33.3 pts)
  - Track D: $T = 2.71\text{ s}$, $M = 170.8\text{ MB}$, $K = 5.48$ (36.9 pts)
- **Next Action:** Commit verified Track B solution and push to `origin/speed-demon` for official leaderboard evaluation.
