# Speed Demon 2026 — Agent Handoff Protocol & Instructions for Codex (`HANDOFF.md`)

**Target Agent:** Codex (Implementation Engineer)  
**Author:** Antigravity (Lead Systems Architect)  
**Status:** Active Competition Execution — Transitioning to Phase 3: Track C (Join)  

---

## 1. Operating Rules for Codex

When implementing any task assigned by Antigravity:

1. **Strictly Respect Scope:**
   - Only edit files explicitly listed under `Files Owned` in the active task.
   - Do not touch `.ai/` governance files (Antigravity maintains these).
   - Do not touch other track directories while working on a designated track.
2. **Zero-Dependency Mandate ($D=1.0$):**
   - Do not add any dependencies to `Cargo.toml`.
   - Use only the Rust standard library (`std`).
   - Concurrency must use `std::thread::scope` or `std::sync` primitives.
3. **Correctness Before Speed:**
   - Always run verification against `tiny` and `sample` datasets first:
     ```bash
     python check.py <track> --data .data/<track>/sample-seed1 --expected <sha256>
     ```
   - If the SHA-256 differs by even one byte, the task is **NOT COMPLETE**. Debug immediately.
4. **Log Benchmark Measurements:**
   - Run local benchmarks before and after every optimization step.
   - Report the exact wall time ($T$), memory footprint ($M$), and line count ($S$) to Antigravity so that `PERF.md` can be updated.
5. **Clean Code & Self-Explanatory Logic:**
   - Human judges account for **35% of the score** (Code Quality 15%, Technique 12%).
   - Avoid obfuscation. Comment complex bitwise operations, SIMD intrinsics, or pointer arithmetic.

---

## 2. Completed Milestones

- **Phase 1: Track A (Ingest)**: Verified $K = 7.39$, 100% byte-identical SHA-256 (`495ca09682...`), $T = 0.65\text{ s}$, $M = 54.6\text{ MB}$.
- **Phase 2: Track D (Traverse)**: Verified $K = 4.79$, 100% byte-identical SHA-256 (`a23dbdf0a3...`), $T = 2.71\text{ s}$, $M = 170.8\text{ MB}$.

---

## 3. Active Assignment: `TASK-C1` (Track C Hostile CSV Parser & Join Engine)

- **Track:** C (Join)
- **Status:** `READY_FOR_DEV`
- **Files Owned:**
  - `track-c/src/main.rs`
  - `track-c/Cargo.toml`
  - `track-c/Dockerfile`
  - `track-c/.speed-demon.toml`
- **Inputs:**
  - `/data/customers.csv` (~10 MB, up to 100,000 customers).
  - `/data/orders.csv` (~200 MB, up to 2,000,000 orders).
- **Outputs:**
  - `/out/aggregates.txt` (Aggregates grouped by `(region, currency)`).
- **Specification Invariants:**
  1. *Hostile CSV Handling:*
     - Mid-file UTF-8 BOM (`\xEF\xBB\xBF`) appears at chunk boundaries or in line starts; must be stripped.
     - Unescaped quotes inside description fields (e.g. `He said "hello"` without RFC 4180 escaping).
     - Both CRLF (`\r\n`) and LF (`\n`) line endings occur in the same file.
     - Unquoted leading/trailing whitespace around fields.
  2. *Foreign Key Join:*
     - Map `customer_id` from order to `customer.region`.
     - Direct table or array lookup indexed by `customer_id` (size $\le 100,000 \implies$ flat array of `u8` region IDs, takes $<1\text{ MB}$!).
  3. *Currency & Financial Arithmetic:*
     - Amounts are decimal strings with 2 decimal places. Parse directly into exact `i64` integer cents (e.g. `$12.34 \to 1234$). Never use floating-point arithmetic.
  4. *Zero External Crates ($D = 1.000$):*
     - No external CSV parsers (banned and dependency penalty). Standard library byte scanner only.
- **Verification Targets:**
  - Sample Seed 1 expected hash: `11c20689b0...`
  - Full Seed 1 expected hash: `d8088b1cd42ba5e56eebfd9981eeb5c3a3b77ce8e7ef9e6231940a02f37e4be6`
