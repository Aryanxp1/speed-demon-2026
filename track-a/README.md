# Track A: Ingest — Team Lex

Rust 2021, standard library only. Uses Team Lex's streaming guards with the fast endpoint lookup inspired by the user-supplied thorta source. Workers parse 1 MiB claims in 64 KiB SIMD batches, reserve 128 compact row slots at a time and compute exact nearest-rank percentiles with radix sorting and escaped uint32 storage. Active-hour leases protect parser writes; late records trigger a complete rerun with early sealing disabled. Ordered output reuses and then releases completed-hour storage.

Endpoint matching uses a fingerprint and seven explicit length bits, fixing the supplied hash's observed different-length merges. Distinct names of the same length can still collide. Full-byte collision checks are absent under the user's score priority; original endpoint bytes are retained for output. Universal exact endpoint matching is not claimed.

The pinned offline O3 Docker build has zero external crates. The current candidate removes rejected PGO training, uses twelve workers to match the scorer and reduces the canonical source to 380 Rust lines plus nine Dockerfile lines. It reads `/data/requests.log` and writes `/out/summary.txt`, with the output directory already present. Optional command-line and environment overrides are absent.

The actual release executable passes 18 reference datasets at 1/4/12 CPU affinities; AddressSanitizer passes the same 18 at 1/12 affinities with fixture paths substituted. These checks use twelve workers even on constrained affinity. Coverage includes arbitrary record ordering, endpoint lengths 1..125, uint32 limits, 1,677 endpoint byte/block variations and 116,135 individually checked latency groups. Full output SHA-256 remains `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`.

Standing verified official best: **85.767155 points, K=51.921007**, run `run-d0163e91b459`, f7e6bf8 source evaluated with commit ba8f1cc. The compact-128 candidate's official result is pending; 86+ has not yet been verified. Rejected experiments and all official run IDs are in PERF.md. Only Track A changes are included.
