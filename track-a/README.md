# Track A: Ingest — Team Lex

Rust 2021, standard library only. Combines the existing Team Lex Rust streaming guards with the fast endpoint lookup from the user-provided `Downloads/thorta/thorta/track-a` source. Unnecessary source comments are removed; provenance and measurements remain in PERF.md.

Workers parse 1 MiB claims in 64 KiB SIMD batches, store compact four-byte rows with an escape for large latencies, and summarize completed hours using radix sorting. Hour storage is reused for output and released after writing. Active-claim guards prevent reclaiming an hour while a parser writes it. Late records trigger a complete rerun with early sealing disabled.

Endpoint matching uses a fingerprint and seven explicit length bits. This fixes the supplied hash's observed merges of different-length repeated-prefix names. Same-length fingerprint collisions remain possible; full-name collision checking is absent to follow the user's score priority. The original bytes are retained for output. Universal exact matching is not claimed.

The pinned offline Docker build links statically with AVX2/AES/VAES support. The executable follows the frozen contest interface: `/data/requests.log` in and `/out/summary.txt` out, with the output directory already mounted. Worker count follows available CPU affinity. Optional command-line paths and environment overrides have been removed to reduce source cost. The implementation has 388 nonblank canonical Rust lines, plus the unchanged nine-line Dockerfile.

The actual release executable passes 18 reference datasets at 1/4/12 CPU affinities. AddressSanitizer passes the same 18 at 1/12 affinities with fixture paths substituted. Checks include reordered input, endpoint lengths 1..125, uint32 boundaries, 1,677 endpoint byte/block variations and 116,135 individually checked latency groups. These checks do not remove the fingerprint-collision limitation.

Previous verified official best: **85.767155 points, K=51.921007**, run `run-d0163e91b459`, all gates passed, using the f7e6bf8 source and O3 build. The compact-interface trial scored 85.602492 points, K=51.528778. The current candidate retains O3 and batches input-page reclamation into 32 MiB pieces after parsing, with release/acquire publication and an integrated worker method. It preserves the parser guards and complete fallback. Its official result is pending; 86+ is not yet claimed. Detailed results and rejected experiments are in PERF.md.
