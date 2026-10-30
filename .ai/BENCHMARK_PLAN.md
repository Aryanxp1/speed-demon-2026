# Speed Demon 2026 — Benchmarking & Profiling Plan (`BENCHMARK_PLAN.md`)

This document defines the testing methodology, profiling tools, metrics calculation, and performance logging protocols.

---

## 1. Benchmarking Hierarchy & Environments

### A. Falkenstein Scoring Box (Ground Truth)
- **CPU:** Intel Core i5-13500 (Raptor Lake, 6 P-cores pinned: `--cpuset-cpus=0-11`, fixed 2.5 GHz base clock, turbo disabled).
- **RAM:** 8 GB limit (`--memory=8g --memory-swap=8g`, no swap).
- **Disk / Cache:** Host pre-warms dataset with `vmtouch` into page cache before container execution.
- **Timing Protocol:** 5 runs serialized; 2 discarded (highest & lowest); mean of middle 3 is official wall time $T$.
- **Memory Protocol:** cgroup v2 `memory.peak` read by host kernel after container exit.

### B. Local Development Machine (Iteration & Regression)
- **Role:** Fast development, correctness testing, and relative profiling.
- **Warning:** Absolute local wall times differ from the Falkenstein box. Always calibrate against the local reference run to compute local $K$.
- **Local Test Tool:** Official `check.py` with Docker:
  ```bash
  python check.py <track> --data .data/<track>/sample-seed1 --expected <sha256>
  ```

---

## 2. Efficiency Coefficient ($K$) Calculation Formula

$$K = \left(\frac{T_{\text{ref}}}{T}\right)^{0.50} \times \min\left(\frac{M_{\text{ref}}}{M}, 50\right)^{0.20} \times \min\left(\frac{S_{\text{ref}}}{S}, 1.0\right)^{0.15} \times \left(\frac{1}{D}\right)^{0.15}$$

### Metric Definitions:
- **$T$ (Wall Time):** Seconds from container start to exit 0. Weight: **0.50 (uncapped)**.
- **$M$ (Peak Memory):** Container peak memory in bytes. Weight: **0.20 (capped at 50×)**.
- **$S$ (Source Code Lines):** Non-comment, non-blank lines counted by `scc` after canonical formatting (`rustfmt`). Weight: **0.15 (capped at 1.0×)**.
- **$D$ (Dependencies):** $1 + \text{count of direct non-stdlib dependencies}$. Weight: **0.15 (capped at 1.0×)**.

### Target Performance Matrix by Track

| Metric | Track A (Ingest) | Track D (Traverse) | Track C (Join) | Track B (Index) |
|---|---|---|---|---|
| **$T_{\text{ref}}$ (Reference)** | 32.05 s | 72.69 s | 14.44 s | 56.35 s |
| **Target $T$** | **0.08 s** | **0.08 s** | **0.08 s** | **0.60 s** |
| **Speedup Ratio ($T_{\text{ref}} / T$)** | ~400× | ~900× | ~180× | ~94× |
| **$M_{\text{ref}}$ (Reference)** | 228 MB | 893 MB | 367 MB | 2.20 GB |
| **Target $M$** | **35 MB** | **110 MB** | **25 MB** | **150 MB** |
| **Memory Ratio ($M_{\text{ref}} / M$)** | 6.5× | 8.1× | 14.7× | 14.6× |
| **$S_{\text{ref}}$ (Reference Lines)** | 56 lines | 43 lines | 108 lines | 91 lines |
| **Target $S$ (Lines)** | ~120 lines | ~140 lines | ~180 lines | ~220 lines |
| **Target $D$ (Dependencies)** | **1.0 (0 deps)** | **1.0 (0 deps)** | **1.0 (0 deps)** | **1.0 (0 deps)** |
| **Projected $K$** | **~26.5** | **~35.2** | **~21.8** | **~16.4** |
| **Projected Speed Score** | **71.1 / 100** | **77.3 / 100** | **66.9 / 100** | **60.7 / 100** |

---

## 3. Profiling & Measurement Tools

1. **Linux `perf` & Flamegraphs (Inside Linux / Docker):**
   - Identify CPU hotspots, cache misses (L1/L2/LLC), and branch mispredictions:
     ```bash
     perf record -g -- ./solution
     perf report
     ```
2. **Memory High-Water Mark:**
   - Under Linux/Docker, inspect cgroup v2 `memory.peak`:
     ```bash
     cat /sys/fs/cgroup/memory.peak
     ```
3. **Source Size Measurement (`scc`):**
   - Measure code lines exactly as the harness does:
     ```bash
     scc --by-file track-a/
     ```

---

## 4. `PERF.md` Logging Standard

Each track requires a `PERF.md` log documenting every iteration. Judges award **8% of total score** based on this document.

### Required Entry Template:
```markdown
### Step N: [Descriptive Optimization Title]
- **Date/Time:** 2026-10-31 04:30 UTC
- **Board Run ID:** `run-a-9418`
- **Hypothesis / Motivation:**
  Parsing timestamps using string splits caused excessive branch mispredictions. Replaced with direct arithmetic on byte offsets.
- **Changes Made:**
  - File: `src/main.rs:45-62`
  - Removed per-line substring allocation.
  - Implemented branchless day/hour decoding.
- **Measured Results:**
  - Local Wall Time: 0.42 s -> 0.18 s (2.33x speedup)
  - Live Board Result: T = 0.19 s, Peak RSS = 32 MB, K = 18.4
- **Decision:** Kept in main branch.
```

Failed experiments must also be logged! Showing why a naive SIMD or multi-threading strategy failed and was reverted proves engineering depth to the judges.

