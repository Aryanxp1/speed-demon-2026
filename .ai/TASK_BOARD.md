# Speed Demon 2026 — Task Board (`TASK_BOARD.md`)

**Current Sprint:** Official Leaderboard Optimization & Judge-Readiness  
**Team Name:** Lex  
**Official Score:** 45.686 Machine Points (Championship Triad: A, D, B)  

---

## Task Pipeline

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
   └── TASK-D4: Track D Multi-Scale & Multi-Seed Verification [DONE]
   │
[Phase 3: Track C — Join]
   │
   ├── TASK-C1: Hostile CSV State Machine Parser (BOMs, Multiline, Escapes) [DONE]
   ├── TASK-C2: Direct Region Lookup & Integer Cents Aggregation [DONE]
   ├── TASK-C3: 64-bit FxHash Distinct Tracking & 2 MB Sliding-Window Buffer [DONE]
   └── TASK-C4: Track C Multi-Scale & Multi-Seed Verification [DONE]
   │
[Phase 4: Track B — Index]
   │
   ├── TASK-B1: Ahead-of-Time Query Vocabulary Filter [DONE]
   ├── TASK-B2: Parallel Corpus Ingest with Monotonic Slices [DONE]
   ├── TASK-B3: Sparse & Dense Left-to-Right Bitset Boolean Evaluation [DONE]
   ├── TASK-B4: Zero-Heap Stack-Based In-Place Phrase Verification [DONE]
   └── TASK-B5: Multi-Scale & Multi-Seed Verification [DONE]
   │
[Phase 5: Official Leaderboard Onboarding]
   │
   ├── TASK-L1: GitHub Repository Creation & Push to speed-demon branch [DONE]
   └── TASK-L2: Speed Demon Board GitHub App Integration & Baseline Scoring [DONE]
   │
[Phase 6: Leaderboard Optimization & Correctness Audits] (CURRENT SPRINT)
   │
   ├── TASK-O1: Reconcile Official Scoring Metrics & Calibration Baselines [DONE]
   ├── TASK-O2: Track C Exact-Byte Arena Deduplication (Holdout Collision Immunity) [DONE]
   ├── TASK-O3: Track B Holdout Phrase Span Safety & Static ASCII Branchless Tables [DONE]
   ├── TASK-O4: Track D Dynamic Atomic Work Scheduling & Interleaved Edge CSR [DONE]
   ├── TASK-O5: Comprehensive Judge Documentation & Real PERF.md Calibration Logs [DONE]
   └── TASK-O6: Push Verified Sprint Improvements to GitHub [READY]
```

---

## Active Task: `TASK-O6` (Commit and Push Sprint Improvements)

- **Track:** All Tracks (A, B, C, D) + Governance
- **Status:** `READY_FOR_PUSH`
- **Summary of Sprint Improvements:**
  - **Track C:** Replaced dual 64-bit polynomial hashes with `DistinctNameTable` backed by a contiguous `Vec<u8>` string arena, providing absolute collision immunity on unseen holdouts while maintaining ~90 MB memory and 0.926s speed.
  - **Track B:** Upgraded token spans to `(u32, u32)` with stack allocation for $\le 1024$ tokens and automatic dynamic heap fallback for longer documents; added static lookup tables `IS_TOKEN_CHAR` and `LOWER_ASCII` to eliminate branch mispredictions; explicitly freed intermediate postings prior to query evaluation.
  - **Track D:** Converted static query chunking to dynamic work scheduling via an `AtomicUsize` work counter; interleaved edge targets into an 8-byte contiguous `EdgeTarget { to: u32, weight: u32 }` structure, reducing CPU user time by 23%; added vertex out-of-bounds safety guards and epoch counter rollover handling.
  - **Documentation & PERF:** Fully reconciled official reference baselines (e.g. Track D 72.69s Falkenstein reference), replaced local informal numbers with official board run metrics, and documented all rejected experiments with empirical measurements.
- **Next Action:** Request user authorization to commit and push to `origin/speed-demon`.
