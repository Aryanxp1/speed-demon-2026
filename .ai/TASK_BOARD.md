# Speed Demon 2026 — Task Board (`TASK_BOARD.md`)

**Current Sprint:** Track C (Join) Engineering  
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
[Phase 3: Track C — Join] (CURRENT FOCUS)
   │
   ├── TASK-C1: Hostile CSV State Machine Parser (BOMs, Multiline, Escapes) [READY]
   ├── TASK-C2: Direct Region Lookup & Integer Cents Aggregation [SCHEDULED]
   └── TASK-C3: Track C Multi-Seed Verification [SCHEDULED]
   │
[Phase 4: Track B — Index (Championship Hedge)]
   │
   ├── TASK-B1: Ahead-of-Time Query Vocabulary Filter [SCHEDULED]
   └── TASK-B2: AVX2 Bitset Boolean & Anchor Phrase Evaluation [SCHEDULED]
   │
[Phase 5: Final Tuning & Freeze]
   │
   └── TASK-F1: Multi-Track Final Regression & Public Freeze Readiness [SCHEDULED]
```

---

## Active Task: `TASK-C1` (Track C Hostile CSV Parser & Join Engine)

- **Track:** C (Join)
- **Status:** `READY_FOR_DEV`
- **Inputs:** `/data/customers.csv`, `/data/orders.csv`
- **Outputs:** `/out/aggregates.txt`
- **Invariants:**
  - Robust streaming CSV parser handling mid-file UTF-8 BOM, unescaped quotes in descriptions, `\r\n` line endings, unquoted whitespace, and integer scale factors for currency.
  - Foreign key join on `customer_id` mapping to customer country/region.
  - Zero external crates ($D = 1.000$).
  - Missing paths/unmatched records handled strictly per specification.
- **Target Metrics:** $T < 0.25\text{ s}$, $M < 40\text{ MB}$, $D = 1.0$.
