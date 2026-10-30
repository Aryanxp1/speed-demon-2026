# Speed Demon 2026 — Project State & Architectural Baseline

**Status:** Active Competition Execution — Track A & Track D Milestones Complete  
**Current Timestamp:** 2026-10-30 17:00 UTC  
**Code Freeze:** 2026-11-02 15:00 UTC (T+2.0h into 72h window)

---

## 1. Executive Summary of Current Progress

Two out of four tracks are now **COMPLETE, VERIFIED, and BENCHMARKED**:
1. **Track A (Ingest)**:
   - **Correctness:** 100% byte-identical SHA-256 match across all scales (`tiny`, `sample`, `full`) and multiple seeds (1, 42, 999).
   - **Execution Speed:** Wall time $T \approx \mathbf{0.65\text{ s}}$ on full 3.1M records (vs 32.05s Python reference $\implies \mathbf{49.1\times\text{ faster}}$).
   - **Memory Footprint:** Peak RSS $M \approx \mathbf{54.6\text{ MB}}$ (vs 228 MB Python reference $\implies \mathbf{4.2\times\text{ leaner}}$).
   - **Dependencies:** Strict zero external dependencies ($D = 1.000$, factor $1.0$).
   - **Source Size:** 263 code lines ($S_{\text{factor}} \approx 0.793$).
   - **Local Estimated Efficiency Coefficient:** $\mathbf{K = 7.39}$ ($\approx \mathbf{43.4\text{ Speed Points}}$ out of 100).

2. **Track D (Traverse)**:
   - **Correctness:** 100% byte-identical SHA-256 match across all scales (`tiny`, `sample`, `full`) and multiple seeds (1, 42, 999).
   - **Execution Speed:** Wall time $T \approx \mathbf{2.71\text{ s}}$ on full 5M edges and 2,000 queries (vs 46.80s Python reference $\implies \mathbf{17.3\times\text{ faster}}$).
   - **Memory Footprint:** Peak RSS $M \approx \mathbf{170.8\text{ MB}}$ (vs 915 MB Python reference $\implies \mathbf{5.36\times\text{ leaner}}$).
   - **Dependencies:** Strict zero external dependencies ($D = 1.000$, factor $1.0$).
   - **Source Size:** 266 code lines.
   - **Local Estimated Efficiency Coefficient:** $\mathbf{K = 4.79}$ ($\approx \mathbf{34.0\text{ Speed Points}}$ out of 100).

---

## 2. Track Portfolio Status

| Track | Task Summary | Status | Verified SHA-256 | Ref Time | Measured $T$ | Ref Mem | Measured $M$ | Deps ($D$) | Local $K$ | Points |
|---|---|---|:---:|---|---|---|---|:---:|:---:|:---:|
| **Track A (Ingest)** | 3.1M log lines aggregation | **MILESTONE VERIFIED** | `495ca09682b96...` (Seed 1) | 32.05 s | **0.65 s** | 228 MB | **54.6 MB** | 1 (0 crates) | **7.39** | **43.4** |
| **Track D (Traverse)** | 2,000 shortest-path queries | **MILESTONE VERIFIED** | `a23dbdf0a3e49...` (Seed 1) | 46.80 s | **2.71 s** | 915 MB | **170.8 MB** | 1 (0 crates) | **4.79** | **34.0** |
| **Track C (Join)** | 202 MB hostile CSV join | `READY_FOR_DEV` | `d8088b1cd42ba...` (Seed 1) | 14.44 s | Pending | 367 MB | Pending | 1 (Target) | ~22 (Proj) | ~67 (Proj) |
| **Track B (Index)** | 50k boolean/phrase queries | `SCHEDULED` | `f86973c210228...` (Seed 1) | 56.35 s | Pending | 2.20 GB | Pending | 1 (Target) | ~16 (Proj) | ~60 (Proj) |

---

## 3. Verified Hashes & Gate Audits

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

---

## 4. Next Strategic Step: Track C (Join)

We are immediately progressing to **Track C (Join)** to complete the 3-track requirement for the Overall Hackathon Championship.
Track C involves hostile CSV parsing (mid-file UTF-8 BOM, unescaped quotes in descriptions, `\r\n` line endings, unquoted whitespace, integer scale factors for currency), foreign key joins on customer country/region, and computing financial aggregates grouped by ISO currency code.
Target: $T \le 0.25\text{ s}$ (vs 14.44s reference $\implies 58\times\text{ speedup}$), $M \le 40\text{ MB}$ (vs 367 MB reference $\implies 9\times\text{ leaner}$), zero dependencies.
