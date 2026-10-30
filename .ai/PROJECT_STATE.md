# Speed Demon 2026 — Project State & Architectural Baseline

**Team Name:** Lex  
**Status:** Active Competition Execution — 3-Track Championship Triad Complete & Verified  
**Current Timestamp:** 2026-10-30 17:15 UTC  
**Code Freeze:** 2026-11-02 15:00 UTC (T+2.25h into 72h window)

---

## 1. Executive Summary of Current Progress

**The required 3-track championship triad (Tracks A, D, C) is now FULLY IMPLEMENTED, VERIFIED, and BENCHMARKED.**

All three solutions:
- Written in clean, idiomatic, zero-dependency Rust ($D = 1.000$).
- Strictly allowlisted Dockerfiles with digest pinning.
- Tested and verified 100% byte-identical SHA-256 against frozen official specifications and multiple random holdout seeds (42, 999).
- Substantial speedups ($12\times$ to $49\times$) and memory reductions ($4.2\times$ to $8.1\times$).

---

## 2. Track Portfolio Status

| Track | Task Summary | Status | Verified Full SHA-256 (Seed 1) | Ref Time | Measured $T$ | Ref Mem | Measured $M$ | Deps ($D$) | Local $K$ | Machine Points |
|---|---|---|:---:|---|---|---|---|:---:|:---:|:---:|
| **Track A (Ingest)** | 3.1M log lines aggregation | **COMPLETE & VERIFIED** | `495ca09682b96...` | 32.05 s | **0.65 s** | 228 MB | **54.6 MB** | 1 (0 crates) | **7.39** | **43.4** |
| **Track D (Traverse)** | 2,000 shortest-path queries | **COMPLETE & VERIFIED** | `a23dbdf0a3e49...` | 46.80 s | **2.71 s** | 915 MB | **170.8 MB** | 1 (0 crates) | **4.79** | **34.0** |
| **Track C (Join)** | 202 MB hostile CSV join | **COMPLETE & VERIFIED** | `d8088b1cd42ba...` | 14.44 s | **1.20 s** | 367 MB | **45.5 MB** | 1 (0 crates) | **4.63** | **33.3** |
| **Track B (Index)** | 50k boolean/phrase queries | `READY_FOR_DEV` | `f86973c210228...` | 56.35 s | Pending | 2.20 GB | Pending | 1 (Target) | ~16 (Proj) | ~60 (Proj) |

---

## 3. Verified Hashes & Gate Audits (Comprehensive)

### Track A (Ingest)
- **Tiny (Seed 1):** `c24fa507c963eaeac9224fd8435019b4f4e8c002ece7e32f316681a3957eb92f` $\to$ **PASS**
- **Sample (Seed 1):** `ca6311bbeb536fa55d85265bf6fd3d24cc9586f13de1ac73bd2a23ac3893aac3` $\to$ **PASS**
- **Sample (Seed 42):** `835c36377118af19264750c9b1031225ead44a8555e913c22d31a827dbe486c8` $\to$ **PASS**
- **Tiny (Seed 999):** `1bec029f790ae2ad7764880a7aa499a21ca9f13ea5aa3fb10caa9664b358f8c9` $\to$ **PASS**
- **Full (Seed 1):** `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3` $\to$ **PASS**

### Track D (Traverse)
- **Tiny (Seed 1):** `ecaca77d3221fe3928109da36480cfdb39f288f2e57ecbc61d3d7706d5cb4bd5` $\to$ **PASS**
- **Sample (Seed 1):** `a4c76ed030bc7e365db33191f6cf74708fba7f25eb48d64c8c4279f2e57046a0` $\to$ **PASS**
- **Tiny (Seed 42):** diff-verified identical against Python reference $\to$ **PASS**
- **Sample (Seed 999):** diff-verified identical against Python reference $\to$ **PASS**
- **Full (Seed 1):** `a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e` $\to$ **PASS**

### Track C (Join)
- **Tiny (Seed 1):** diff-verified identical against Python reference $\to$ **PASS**
- **Sample (Seed 1):** `9035fed36823232dd41ae2d1b9d6d21aa7c73f49d8e95647e953a89d5ebe9a66` $\to$ **PASS**
- **Tiny (Seed 42):** diff-verified identical against Python reference $\to$ **PASS**
- **Sample (Seed 999):** diff-verified identical against Python reference $\to$ **PASS**
- **Full (Seed 1):** `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536` $\to$ **PASS**

---

## 4. Next Strategic Step: Track B (Index) / Optimization Sprints

We now possess a complete, valid submission across three tracks meeting all eligibility requirements for the Overall Championship.
Next options:
1. **Option A (Hedge & Dominance):** Implement Track B (Index) to compete across all 4 tracks and give us 4 scores for the $\max(3)$ geometric mean formula.
2. **Option B (Deep Optimization):** Further optimize Tracks A, D, and C using multi-threading and SIMD to push $K$ higher.
