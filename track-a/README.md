# Track A: Ingest — Team Lex

Retained verified best: **85.847678 points, K=52.113899**, run `run-3a1919ec7fb8`, evaluated at `9e4b798`. Every gate passed. The active source and build are the exact f7e6bf8 version that produced that result. The supplied trackkk1 adaptation passed every gate but scored **85.607294, K=51.540175**, so it was reverted. Its source remains in commit `aa30116`. The 86+ target has not been verified.

Rust 2021, standard library only. Workers parse 1 MiB claims in 64 KiB SIMD batches, store four-byte rows with escaped uint32 latencies, and calculate exact nearest-rank percentiles with radix sorting. Active-hour reader leases guard sealing and reclamation. Late records trigger a complete rerun with early sealing disabled. Ordered output reuses completed-hour storage.

The original pinned O3/fat-LTO/static x86-64-v3/AES/VAES build uses zero external crates. Workers are capped at twelve. Use `--data` and `--out`, or `DATA_DIR`/`OUT_DIR`; `SD_DATA_DIR`, `SD_OUT_DIR` and `SD_THREADS` are supported. Defaults are `/data/requests.log` and `/out/summary.txt`. Canonical count is 417 Rust plus nine Dockerfile lines.

Endpoint matching retains the asymmetric fingerprint key and seven explicit length bits. Rare same-length collisions remain possible because full-byte collision checks are absent under the user's score priority. Original endpoint bytes are retained for output; universal collision-free matching is not claimed.

The retained Cargo executable passes all 18 reference datasets at 1/4/12 CPU affinities and AddressSanitizer at 1/12 workers. Full output SHA-256 is `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. PERF.md records the supplied source's provenance, focused adaptation, tests and official result. Only Track A was changed by this pass.
