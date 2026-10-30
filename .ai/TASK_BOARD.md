# Speed Demon 2026 — Task Board (`TASK_BOARD.md`)

**Current Sprint:** Track B (Index) or Optimization Pass  
**Competition Status:** Hour 2 of 72  

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
[Phase 4: Track B — Index (Championship Hedge / 4th Track)] (CURRENT FOCUS)
   │
   ├── TASK-B1: Ahead-of-Time Query Vocabulary Filter [READY]
   └── TASK-B2: Inverted Index & Fast Postings Intersection [SCHEDULED]
   │
[Phase 5: Final Tuning & Freeze]
   │
   └── TASK-F1: Multi-Track Final Regression & Public Freeze Readiness [SCHEDULED]
```

---

## Active Task: `TASK-B1` (Track B Inverted Index Engine)

- **Track:** B (Index)
- **Status:** `READY_FOR_DEV`
- **Inputs:** `/data/corpus.txt`, `/data/queries.txt`
- **Outputs:** `/out/results.txt`
- **Invariants:**
  - Inverted index built from corpus of documents.
  - Boolean AND, OR, NOT and exact phrase queries.
  - Zero external crates ($D = 1.000$).
- **Target Metrics:** $T < 1.0\text{ s}$, $M < 300\text{ MB}$, $D = 1.0$.
