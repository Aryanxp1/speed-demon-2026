# Track A: Ingest — Team Lex

Latest leaderboard score: **86.225 points, K=53.027431**, scored commit `2ae1a6c`; T=0.018 s, M=16 MB, S=406. These are the displayed values in the latest 2 November 2026 screenshot. PERF.md records provenance, rejected trials and historical results.

The Rust engine retains active-hour writer leases, exclusive sealing, a complete arbitrary-order rerun, corrected endpoint-length fingerprints, compact escaped rows, exact radix percentiles and ordered output. The supplied trackaa version contributed eight-byte SIMD space detection, multiply-add decoding for common latencies, common-eight-digit decimal output, and the fixed contest interface. Full uint32 input/output paths remain. Its original hash and unguarded reclamation are not substituted.

Rust 2021, standard library only, unchanged pinned offline O3/fat-LTO/static AVX2/AES/VAES build. Workers follow available CPU affinity, capped at twelve. Input: `/data/requests.log`; output: `/out/summary.txt`. The harness supplies the output directory. Optional flag and environment overrides are omitted. Source: 397 Rust plus nine Dockerfile lines.

All 18 independent reference datasets pass in the actual Cargo executable at 1/4/12 workers and CPU affinities. AddressSanitizer passes all 18 at 1/12 affinities with only main's fixture paths substituted. Checks include empty/reordered input, endpoint identities, decimal boundaries and full uint32 values. Full SHA-256: `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. Rare same-length fingerprint collisions remain possible; universal exact matching is not claimed.
