# Speed Demon 2026 — Project State & Architectural Baseline

**Team Name:** Lex  
**Global Rank:** **10**  
**Repository:** [Aryanxp1/speed-demon-2026](https://github.com/Aryanxp1/speed-demon-2026)  
**Submission Branch:** `speed-demon`  
**Current Overall Machine Score:** **79.476** (Championship Triad: Track A, Track C, Track D)  
**Current Timestamp:** 2026-11-01 14:55 UTC (20:25 IST)  
**Code Freeze:** 2026-11-02 15:00 UTC  

---

## 1. Official Live Leaderboard Standing

All 4 tracks are live, passing all correctness and sandbox gates on the official scoring box (Falkenstein Intel Core i5-13500 pinned P-cores).

| Track | Task Summary | Status | Language | Commit | Official $T$ | Official $M$ | Formatted $S$ | Official $K$ | Official Machine Score |
|---|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **Track A (Ingest)** | 3.1M log lines aggregation | **COUNTING (Rank 1)** | C++20 | `60cb66c` | **0.0169 s** | **20.90 MB** | 575 | **49.516286** | **84.737 pts** |
| **Track C (Join)** | 202 MB hostile CSV join | **COUNTING (Rank 2)** | C++20 | `d752aea` | **0.0281 s** | **8.73 MB** | 320 | **40.681817** | **80.470 pts** |
| **Track D (Traverse)** | 2,000 shortest-path queries | **COUNTING (Rank 3, #1 K)** | C++20 | `f94eb34` | **0.0987 s** | **62.05 MB** | 828 | **29.676927** | **73.621 pts** |
| **Track B (Index)** | 250k docs, 50k queries | *DROPPED (Weakest of 4)* | Rust | `2c9f85b` | **0.1728 s** | **47.48 MB** | 647 | **28.977962** | **73.103 pts** |

**Overall Championship Geometric Mean Score:**
$$\sqrt[3]{84.737403 \times 80.470017 \times 73.620947} = \mathbf{79.476 \text{ points}}$$

---

## 2. Multi-Scale & Multi-Seed Verification Matrix

| Scale & Seed | Track A (Ingest) | Track B (Index) | Track C (Join) | Track D (Traverse) |
|---|:---:|:---:|:---:|:---:|
| **Seed 1 Tiny** | PASS | PASS | PASS | PASS |
| **Seed 1 Sample** | PASS (`ca6311...`) | PASS (`eeeec1...`) | PASS (`9035fe...`) | PASS (`a4c76e...`) |
| **Seed 1 Full** | PASS (`495ca0...`) | PASS (`f86973...`) | PASS (`d8088b...`) | PASS (`a23dbd...`) |
| **Test Seed 42** | PASS | PASS | PASS | PASS |
| **Test Seed 999** | PASS | PASS | PASS | PASS |

---

## 3. Mission Status & Current Objectives

1. **Track C Target Achieved:**
   - Crossed the 80.0-point milestone: **80.470 points** ($K = 40.681817$, $T = 0.028118$ s, $M = 8.73$ MB, $S = 320$ lines, commit `d752aea`).
   - Grouped unmapping of completed inputs in 4-piece batches and offline seed-17 PGO delivered a 13.0% runtime reduction and pushed K past 40.
2. **Track D Target Achieved (#1 by K across the entire leaderboard):**
   - Achieved **#1 Track D efficiency**: **73.621 points** ($K = 29.676927$, $T = 0.098744$ s, $M = 62.05$ MB, $S = 828$ lines, commit `f94eb34`).
   - AVX2 delimiter scanning, compact 64 KiB fragment recycling, and persistent 12-worker phase pools established an outright first place lead in K.
3. **Track A Performance Record:**
   - Established team record: **84.737 points** ($K = 49.516286$, $T = 0.016902$ s, $M = 20.90$ MB, $S = 575$ lines, commit `60cb66c`).
4. **Overall Portfolio:**
   - Team Lex holds **Rank 10** on the global leaderboard with **79.476 points**.
