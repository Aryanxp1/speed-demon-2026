# Track A: Ingest — Team Lex

Verified standing best: **85.847678 points, K=52.113899**, run `run-3a1919ec7fb8`, evaluated at `9e4b798`, using the f7e6bf8 source and original pinned build. The focused trackkk1 adaptation is awaiting official evaluation. The supplied version's reported 86.17 is not a score claimed for this adaptation.

Rust 2021, standard library only. The adaptation combines our parser and active-hour write guards with the supplied shared sealing-progress approach. Workers parse 1 MiB claims in 64 KiB SIMD batches and store compact four-byte rows with escaped large uint32 latencies. Exact nearest-rank percentiles use radix sorting. Late records trigger a complete rerun with early sealing disabled. Ordered output reuses and releases completed-hour storage.

Input is `/data/requests.log`, output `/out/summary.txt`; the output directory must exist. Available CPU affinity sets the worker count, capped at twelve. Optional path and worker-environment overrides are omitted. The original pinned O3/fat-LTO/static x86-64-v3/AES/VAES build remains, with zero external crates. Canonical count is 395 Rust lines plus nine Dockerfile lines.

Endpoint matching retains our asymmetric fingerprint key and seven explicit length bits. Rare same-length collisions remain possible because full-byte collision checks are absent under the user's score priority. Original endpoint bytes are preserved for output; universal collision-free matching is not claimed.

The actual release executable passes all 18 reference datasets at 1/4/12 CPU affinities. AddressSanitizer passes all 18 at 1/12 affinities with only fixture directory arguments substituted. Full output SHA-256 is `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. PERF.md records provenance, tests, official results and rejected trials. Only Track A is changed by this pass.
