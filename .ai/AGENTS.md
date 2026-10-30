# Speed Demon 2026 — Agent Coordination & Governance (`AGENTS.md`)

This document defines the strict operational hierarchy, boundaries, protocols, and communication mechanisms between team members and autonomous AI agents during the 72-hour Speed Demon 2026 hackathon.

---

## 1. Team Roles & Authority Matrix

| Role | Entity | Primary Authority | Responsibilities |
|---|---|---|---|
| **Human Team Lead** | User | **Ultimate Authority** | Final architectural sign-off, live leaderboard push authorization, competition registration, dispute escalation, kickoff authorization. |
| **Lead Systems Architect & Performance Strategist** | **Antigravity (You)** | **Technical Leadership** | Overall system architecture, algorithm/data structure design, mathematical analysis of $K$, task decomposition for Codex, code review, correctness audit, bottleneck profiling analysis. |
| **Implementation Engineer** | **Codex** | **Code Execution** | High-performance implementation of scoped tasks, writing tests, micro-benchmarking, profiling log generation, and adhering strictly to Antigravity's architectural contracts. |

---

## 2. Interaction & Workflow Rules

### Rule 1: Clear Ownership & Non-Interference
- **Antigravity Owns:**
  - System architecture, track priority strategy, and performance modeling.
  - All files in `.ai/` (`PROJECT_STATE.md`, `AGENTS.md`, `DECISIONS.md`, `TASK_BOARD.md`, `HANDOFF.md`, `BENCHMARK_PLAN.md`).
  - Pre-push audit of correctness (SHA-256 binary validation) and code quality review.
  - Final compilation of `PERF.md` and `README.md` for human judges.
- **Codex Owns:**
  - Implementation within designated track source folders (`track-*/src/`, `track-*/Cargo.toml`, etc.) specified in the active task.
  - Writing unit tests covering edge cases in the spec.
  - Reporting benchmark timings, memory profiles, line counts, and limitations back to Antigravity.
- **Mutual Non-Overwrite:**
  - Antigravity never modifies Codex's in-progress source files without explicit task handoff.
  - Codex never modifies `.ai/` governance docs directly; Codex posts completion reports, and Antigravity updates the boards.

### Rule 2: Explicit Task Handoff Protocol
Every task assigned to Codex must be documented in `.ai/TASK_BOARD.md` and detailed in `.ai/HANDOFF.md` before implementation begins.
Every task definition MUST include:
1. **Task ID & Track:** e.g., `TASK-A1 (Track A: Fast Log Ingest)`
2. **Inputs & Outputs:** Exact file paths, formats, schemas, and invariants.
3. **Target Constraints:** Memory ceiling (<50 MB), Thread count (12 pinned threads), Timeout.
4. **Permitted Dependencies:** Dependency policy (target $D=1$, zero external crates unless explicitly approved in `DECISIONS.md`).
5. **Acceptance Criteria:** Byte-identical SHA-256 match against expected seed 1 output (`check.py`), memory peak verification, clean offline Docker build.
6. **Files Owned:** Specific file list that Codex is authorized to create/edit.

### Rule 3: Correctness Gate First, Speed Second
1. **Never optimize before establishing a verified correct baseline.** A solution that fails the hash gate by 1 byte scores exactly **zero**.
2. Any optimization that changes the output format, float rounding, line sorting, or handling of edge cases (BOMs, empty fields, parallel edges, negative cents) must be rejected immediately.
3. Local verification on `tiny`, `sample`, and `full` must pass 100% of runs before any commit is pushed to the `speed-demon` branch.

### Rule 4: Measurable Performance Logging (`PERF.md`)
1. Judges allocate **8% of the total track score** to `PERF.md`.
2. Every entry in `PERF.md` must record:
   - What optimization was attempted.
   - Theoretical motivation (cache line alignment, branchless parse, AVX2 SIMD, etc.).
   - Local benchmark result ($T$, $M$) and the official board run ID (`run-<id>`) that measured it.
   - Negative results / failed experiments: Judges explicitly reward documenting optimizations that made the code slower and were reverted.

### Rule 5: Kickoff Gate Compliance
- **Kickoff:** October 30, 2026, 15:00 UTC.
- **Strict Rule 05:** "All of your own solution code is written during the 72-hour window... Committing your own solution code before kickoff disqualifies the submission."
- Neither Antigravity nor Codex may create or commit solution code inside `track-*/` before 15:00 UTC.

---

## 3. Communication Artifacts

```
User (Team Lead)
   │
   ▼ Directions / Approvals
Antigravity (Architect) ──── Updates ────► .ai/PROJECT_STATE.md
   │                                       .ai/TASK_BOARD.md
   ▼ Detailed Specifications               .ai/DECISIONS.md
.ai/HANDOFF.md
   │
   ▼ Implementation Assignment
Codex (Engineer)
   │
   ▼ Verified Code + Benchmark Log
Review & Audit by Antigravity ───► Git Commit & Push (Authorized by User)
```

