# Speed Demon 2026 — Project State & Architectural Baseline

**Team Name:** Lex  
**Repository:** [Aryanxp1/speed-demon-2026](https://github.com/Aryanxp1/speed-demon-2026)  
**Submission Branch:** `speed-demon`  
**Current Overall Machine Score:** **45.686** (Championship Triad: A, D, B)  
**Baseline Git Tag:** `v1.0-official-baseline` (Commit `0d10837`)  
**Current Timestamp:** 2026-10-31 00:30 UTC  
**Code Freeze:** 2026-11-02 15:00 UTC

---

## 1. Official Live Leaderboard Standing

All 4 tracks are live, passing all correctness and sandbox gates on the official scoring box (Falkenstein Intel Core i5-13500 pinned P-cores).

| Track | Task Summary | Status | Official $T$ | Official $M$ | Official $K$ | Official Machine Score | Verified Seed-1 Full Hash |
|---|---|:---:|:---:|:---:|:---:|:---:|:---:|
| **Track A (Ingest)** | 3.1M log lines aggregation | **ACTIVE TRIO (Rank 1)** | **0.292 s** | **61 MB** | **10.284873** | **50.610 pts** | `495ca09682...` (PASS) |
| **Track D (Traverse)** | 2,000 shortest-path queries | **ACTIVE TRIO (Rank 2)** | **1.512 s** | **177 MB** | **7.430230** | **43.550 pts** | `a23dbdf0a3...` (PASS) |
| **Track B (Index)** | 250k docs, 50k queries | **ACTIVE TRIO (Rank 3)** | **1.071 s** | **534 MB** | **7.333291** | **43.265 pts** | `f86973c210...` (PASS) |
| **Track C (Join)** | 202 MB hostile CSV join | **RESERVE HEDGE** | **0.926 s** | **88 MB** | **4.378513** | **32.066 pts** | `d8088b1cd4...` (PASS) |

**Overall Championship Geometric Mean Score:** **45.686** (Geometric mean of A: 50.610, D: 43.550, B: 43.265).

---

## 2. Multi-Scale & Multi-Seed Verification Matrix

| Scale & Seed | Track A (Ingest) | Track B (Index) | Track C (Join) | Track D (Traverse) |
|---|:---:|:---:|:---:|:---:|
| **Seed 1 Tiny** | PASS | PASS | PASS | PASS |
| **Seed 1 Sample** | PASS (`ca6311...`) | PASS (`eeeec1...`) | PASS (`9035fe...`) | PASS (`a4c76e...`) |
| **Seed 1 Full** | PASS (`495ca0...`) | PASS (`f86973...`) | PASS (`d8088b...`) | PASS (`a23dbd...`) |
| **Test Seed 42** | PASS | PASS | PASS | PASS |
| **Test Seed 999** | PASS | PASS | PASS | PASS |

*Note: Seeds 42 and 999 are synthetic test seeds generated locally using the official generator to test robustness against parameter perturbations; they are not the organizer's secret holdout dataset.*

---

## 3. Mission Objectives & Focus

Our current championship triad consists of **A, D, and B**.
1. **Track B Optimization (Target: $T \le 0.45\text{ s}$, $M \le 250\text{ MB}$):**
   - Official scoring box: $T = 1.071\text{ s}$, $M = 534\text{ MB}$.
   - Focus: Reduce memory footprint (from 534 MB down to ~250 MB) and SIMD/streaming phrase verification to boost $K$ from 7.33 to $\ge 12$.
2. **Track D Optimization (Target: $T \le 0.80\text{ s}$, $M \le 120\text{ MB}$):**
   - Official scoring box: $T = 1.512\text{ s}$, $M = 177\text{ MB}$.
   - Focus: Cache-friendly Dijkstra priority queue, dynamic work stealing across worker threads, and memory footprint reduction to boost $K$ from 7.43 to $\ge 12$.
3. **Track C Correctness & Robustness Audit:**
   - Replace 128-bit hash distinct-name tracking with exact byte comparison to remove any theoretical collision risk on unseen holdout distributions while preserving sub-1.0s runtime.
4. **Track A Fine-Tuning:**
   - Official score is already 50.610 pts ($T = 0.292\text{ s}$, $M = 61\text{ MB}$). Profile for micro-optimizations that preserve 100% correctness.
