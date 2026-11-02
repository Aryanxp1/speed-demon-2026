# Track A: Ingest — Team Lex

Standing verified official best: **85.847678 points, K=52.113899**, run `run-3a1919ec7fb8`, evaluated commit `9e4b798`. Every gate passed using the exact f7e6bf8 source and original pinned O3 build. The eight-digit formatter candidate is awaiting official evaluation; 86+ has not yet been verified. Full evidence and rejected trials are in PERF.md.

Rust 2021, standard library only. Combines Team Lex's streaming guards with the user-provided thorta endpoint lookup. Workers parse 1 MiB claims in 64 KiB SIMD batches, store four-byte rows with an escape for large uint32 latencies and compute exact nearest-rank percentiles with radix sorting. Active-hour leases protect parser writes; late records trigger a complete rerun with early sealing disabled. Ordered output reuses completed-hour storage.

Endpoint matching uses a fingerprint and seven explicit length bits. This corrects the supplied hash's observed different-length merges. Distinct names of the same length can still collide; full-byte collision checks are absent under the user's score priority. Original endpoint bytes are retained for output. Universal exact endpoint matching is not claimed.

The pinned offline O3 Docker build links statically with AVX2/AES/VAES support and zero external crates. Available CPU affinity determines the worker count. Input/output follow the frozen `/data/requests.log` and `/out/summary.txt` paths; the output directory must exist. Numbers below 100,000,000 use an eight-digit conversion; larger uint32 values retain the original full-range path. Canonical count is 394 Rust lines plus nine Dockerfile lines.

The actual candidate Cargo executable passes all 18 reference datasets at 1/4/12 CPU affinities. AddressSanitizer passes all 18 at 1/12 workers with only fixture file paths substituted. Checks cover reordered input, endpoint lengths 1..125, uint32 limits, 1,677 endpoint byte/block variations and 116,135 individually checked latency groups. Full output SHA-256 is `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. These tests do not eliminate the fingerprint-collision limitation. Only Track A changes are included in this work.
