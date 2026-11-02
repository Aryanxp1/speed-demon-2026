# Track A: Ingest — Team Lex

Verified official best: **86.101357 points, K=52.726280**, run `run-9e16dea2adc9`, source `421ac2a`. All gates passed: T=0.018202 s, M=16,470,016 bytes, S=406, D=1. The exact evaluated source and pinned build are retained. This exceeds the previous 85.847678-point best and meets the 86+ target. PERF.md records provenance, rejected trials and results.

The Rust engine retains active-hour writer leases, exclusive sealing, a complete arbitrary-order rerun, corrected endpoint-length fingerprints, compact escaped rows, exact radix percentiles and ordered output. The supplied trackaa version contributed eight-byte SIMD space detection, multiply-add decoding for common latencies, common-eight-digit decimal output, and the fixed contest interface. Full uint32 input/output paths remain. Its original hash and unguarded reclamation are not substituted.

Rust 2021, standard library only, unchanged pinned offline O3/fat-LTO/static AVX2/AES/VAES build. Workers follow available CPU affinity, capped at twelve. Input: `/data/requests.log`; output: `/out/summary.txt`. The harness supplies the output directory. Optional flag and environment overrides are omitted. Source: 397 Rust plus nine Dockerfile lines.

All 18 independent reference datasets pass in the actual Cargo executable at 1/4/12 workers and CPU affinities. AddressSanitizer passes all 18 at 1/12 affinities with only main's fixture paths substituted. Checks include empty/reordered input, endpoint identities, decimal boundaries and full uint32 values. Full SHA-256: `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. Rare same-length fingerprint collisions remain possible; universal exact matching is not claimed.
