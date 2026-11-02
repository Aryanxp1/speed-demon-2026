# Track A: Ingest — Team Lex

Retained verified official best: **85.767155 points, K=51.921007**, run `run-d0163e91b459`, evaluated commit `ba8f1cc`. All gates passed. The active source is the exact `f7e6bf8` Rust implementation and original pinned O3 build that produced that result. Later trials scored lower and have been reverted; the 86–87 target is not yet achieved. Full evidence is in PERF.md.

Rust 2021, standard library only. Combines Team Lex's streaming guards with the user-provided thorta endpoint lookup. Workers parse 1 MiB claims in 64 KiB SIMD batches, store four-byte rows with an escape for large uint32 latencies and compute exact nearest-rank percentiles with radix sorting. Active-hour leases protect parser writes; late records trigger a complete rerun with early sealing disabled. Ordered output reuses completed-hour storage.

Endpoint matching uses a fingerprint and seven explicit length bits. This corrects the supplied hash's observed different-length merges. Distinct names of the same length can still collide; full-byte collision checks are absent under the user's score priority. Original endpoint bytes are retained for output. Universal exact endpoint matching is not claimed.

The pinned offline O3 Docker build links statically with AVX2/AES/VAES support and zero external crates. Workers are capped at twelve. Use `--data` and `--out`, or `DATA_DIR`/`OUT_DIR`; `SD_DATA_DIR`, `SD_OUT_DIR` and `SD_THREADS` are supported. Defaults follow `/data/requests.log` and `/out/summary.txt`.

The restored production Cargo executable passes all 18 reference datasets at 1/4/12 CPU affinities. AddressSanitizer passes all 18 at 1/12 workers. Checks cover reordered input, endpoint lengths 1..125, uint32 limits, 1,677 endpoint byte/block variations and 116,135 individually checked latency groups. Full output SHA-256 is `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. These tests do not eliminate the fingerprint-collision limitation. Only Track A changes are included in this work.
