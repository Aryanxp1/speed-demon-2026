# Team Lex — Speed Demon 2026 final submission

Repository: [Aryanxp1/speed-demon-2026](https://github.com/Aryanxp1/speed-demon-2026). Submission branch: `speed-demon`.

This document records the official public full-dataset, seed-1 bests from the latest supplied **2 November 2026** leaderboard screenshot, including Track B's newer result at `534e399`. These are observed leaderboard results, not predictions for the private final evaluation. Individual runs can vary.

## Official results

**Overall: 82.597 points**, the geometric mean of A, C and B, as shown in the latest supplied leaderboard screenshot on 2 November 2026. Track D is the fourth, non-counting track in this snapshot.

- **Track A: 86.225029 points**, K=53.027431; T=0.018219 s; M=15,970,304 bytes; S=406; D=1. Best evaluated commit [`2ae1a6c`](https://github.com/Aryanxp1/speed-demon-2026/commit/2ae1a6c0b2bc3f11cafd9478982dacde0e218656); run `run-c3755766ac89`.
- **Track B: 79.073 points**, K=38.146211; T=0.087 s; M=42 MB; S=1148. Best evaluated commit [`534e399`](https://github.com/Aryanxp1/speed-demon-2026/commit/534e399). These are the latest screenshot's displayed values; its run ID and unrounded points/time/memory are not shown.
- **Track C: 82.648215 points**, K=44.974289; T=0.022840 s; M=8,830,976 bytes; S=323; D=1. Best evaluated commit [`2ae1a6c`](https://github.com/Aryanxp1/speed-demon-2026/commit/2ae1a6c0b2bc3f11cafd9478982dacde0e218656); run `run-d42f3999e566`.
- **Track D: 76.514177 points**, K=33.906546; T=0.083196 s; M=56,451,072 bytes; S=684; D=1. Best evaluated commit [`4588426`](https://github.com/Aryanxp1/speed-demon-2026/commit/45884261ae64e2c6184b534212cb466d730db336); run `run-f0ffbfc40e9a`.

All four listed best runs passed the official build, check, correctness, timeout and sandbox gates. `T` is elapsed time, `M` is charged peak memory, `S` is canonical source size and `D` is the dependency factor; every track reports dependency count zero and D=1. Local benchmark times and external authors' scores are not substituted for these measurements.

## Source selection and final packaging

- Track A: current source/build match the recorded best snapshot `2ae1a6c`.
- Track B: current source/build match the recorded best snapshot `421ac2a`.
- Track C: current source/build match the recorded best snapshot `2ae1a6c`.
- Track D: current source/build differ from the recorded best snapshot `4588426`.

The final selection should use the exact source and build recipe of each listed best commit, rather than checking out one historical commit for every track. Documentation-only commits do not change those implementations. This documentation task changes no solution, manifest, compiler flag or Dockerfile. Other chats are handling the remaining source restoration; the status above is explicitly tied to the recorded snapshot.

## Track A — Ingest

**Rust 2021.** Input `/data/requests.log`; output `/out/summary.txt`. Group access-log records by UTC hour and endpoint; emit count, nearest-rank p50/p95/p99 latency, 5xx count and integer error rate, ordered by hour and endpoint.

The selected engine maps the input, scans 1 MiB claims in 64 KiB SIMD batches and stores compact four-byte rows, with an escape for larger uint32 latencies. Endpoint interning keeps explicit length bits. Active-hour writer leases and exclusive sealing allow parsing and summarizing to overlap; a complete non-streaming rerun handles late/out-of-order records. Two radix passes produce exact percentile ranks. Common latencies use masked SIMD multiply-add decoding; common output numbers use an eight-digit formatter with the full uint32 path retained. The harness supplies `/out`; workers follow available affinity, capped at twelve.

The selected source passed 18 independent reference datasets at 1/4/12 CPU affinities and AddressSanitizer at 1/12, including reordered input, endpoint boundaries and uint32 extremes. Full-output SHA-256: `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`.

Details: [source](track-a/src/main.rs), [build](track-a/Dockerfile), [performance history](track-a/PERF.md).

## Track B — Index

**Rust 2021.** Inputs `/data/docs.txt` and `/data/queries.txt`; output `/out/results.txt`. Answer Boolean queries strictly left to right and phrase queries with count, sum of document IDs and ten smallest IDs.

The engine interns queried terms and scans the corpus in bounded phases. It uses sparse postings and transposed heavy-term bitmaps, eight-token AVX2 action compaction, and phrase checks around the rarest anchor. Twelve quarter-MiB sample jobs are spread over 1 MiB claims; term-owned reduction reads contiguous atomic counters. Density checks shrink phases and claims to keep u16 offsets valid. Scoped workers finish and file writes complete before early exit. Input/output overrides are supported through `--data`/`--out`, `DATA_DIR`/`OUT_DIR`, or `SD_DATA`/`SD_OUT`; workers are capped at twelve.

The retained version was rebuilt and passed all 27 reference datasets at 1/4/12 affinities. The identical source also passed AddressSanitizer at 1/12, including SIMD tails and the 250,000-document density fallback. The lower-scoring final-minute SLVS experiment was reverted. Full-output SHA-256: `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927`.

Details: [source](track-b/src/main.rs), [build](track-b/Dockerfile), [performance history](track-b/PERF.md).

## Track C — Join

**Rust 2021.** Inputs `/data/regions.csv` and `/data/records.csv`; output `/out/aggregates.txt`. Join records to country groups and emit counts, ID/amount sums, minimum/maximum amount, distinct names and decoded field lengths.

AVX2 structural masks and carry-less multiplication identify CSV delimiters outside quotes. Numeric fields are decoded in batches; region IDs are clamped in SIMD before prefetching and reading the region table. The selected version retains the original record-count loop and BOM handling. Tree workers collect private statistics and update shared atomic name/bitmap tables. Compact row heads and cold overflow tails preserve 256 groups and 131,072 ID slots, while hot storage fits in two huge pages. Checked private mappings support a missing final newline without modifying input files. Positional arguments can override the two input paths and output path.

The selected source passed 12 reference datasets at 1/4/12 affinities and AddressSanitizer at 1/12. Checks include quoted/multiline CSV, BOM/Unicode cases, negative amounts, 110,000 names across 255 countries, and missing/page-aligned final terminators. The serial table-setup experiment was reverted.

Details: [source](track-c/src/main.rs), [build](track-c/Dockerfile), [performance history](track-c/PERF.md).

## Track D — Traverse

**C++20.** Read graph/query files from `/data`; write `/out/distances.txt`. Runtime overrides: `SD_DATA`, `SD_OUT`, `SD_THREADS`.

The evaluated best uses compact raw edge rows, fused degree-two contraction and locality ordering, parallel graph construction and temporary-page recycling. Queries use a forward probe followed by exact bidirectional shortest-path search. Saturated narrow labels fall back to full 64-bit distances when needed; an overflow heap preserves distant queue entries. The pinned offline GCC build uses native targeting, LTO, static linking and PGO trained from the published generator at seed 17. The best snapshot is the original evaluated configuration, not an unevaluated bin-release or compiler trial.

The performance history records public tiny/sample/full checks, independent graph oracles, boundary/saturation and contraction fixtures, and ASan/UBSan/TSan checks. Full-output SHA-256: `a23dbdf0a3e49cc8901310a9ee1dc6a9a569a85297b6da75c2a7002cb2ed277e`.

Details: [source](track-d/d_traverse.cpp), [build](track-d/Dockerfile), [performance history](track-d/PERF.md).

## Reproduce a submission build

Build from each track's directory using its checked-in, digest-pinned Dockerfile. Rust builds use offline Cargo and no external crates. A/B use static x86-64-v3 builds with AES/VAES; C retains its Raptor Lake compiler target. D retains its visible build-time PGO stages. No compiler or build setting is changed by this document.

```sh
docker build -t lex-track-a ./track-a
docker build -t lex-track-b ./track-b
docker build -t lex-track-c ./track-c
docker build -t lex-track-d ./track-d
```

For an individual track, supply its matching dataset and writable output directory:

```sh
docker run --rm --network none --read-only --cpuset-cpus=0-11   -v /absolute/path/to/data:/data:ro   -v /absolute/path/to/output:/out   lex-track-a
```

Use the corresponding image for B/C/D. The official harness remains authoritative for resource limits, canonical source counting, correctness and final scoring. Local Docker execution is not claimed where the daemon was unavailable.

## Provenance and correctness limits

The Rust implementations combine the existing Team Lex engines with user-supplied optimization ideas; the per-track PERF files identify the supplied versions and distinguish their reported scores from Lex's official measurements. No claim of sole original authorship is made for borrowed ideas.

A/B/C retain the documented score-oriented fingerprint matching. Rare hash collisions can merge different keys; passing the reference suites does not establish collision-free matching for every possible input. Numeric rules, percentile ranks, Boolean evaluation order and graph-distance fallbacks remain as documented. Only official observed results are called verified; rejected and unmeasured trials remain labeled in the histories.
