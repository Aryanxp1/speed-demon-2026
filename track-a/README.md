# Track A: Ingest — Team Lex

Verified standing best remains **85.847678 points, K=52.113899**, run `run-3a1919ec7fb8`. The SIMD adaptation passed every gate but scored 85.719189 and 85.415870. Its smaller fixed-interface comparison is now awaiting official evaluation; 86+ is not claimed. PERF.md records provenance and evidence.

The candidate adapts eight-byte SIMD space detection and common-latency multiply-add decoding into our Rust engine, plus common-eight-digit decimal output. Full uint32 input/output paths remain. Existing active-hour writer leases, exclusive sealing, arbitrary-order rerun, corrected endpoint-length fingerprints, radix percentiles and ordered output are retained. The supplied unguarded reclamation and original hash are not substituted.

Rust 2021, standard library only, unchanged pinned offline O3/fat-LTO/static AVX2/AES/VAES build. Workers follow available CPU affinity, capped at twelve. Input: `/data/requests.log`; output: `/out/summary.txt`. The harness supplies the output directory. Optional flag and environment overrides are omitted in this comparison.

All 18 independent reference datasets pass at 1/4/12 workers and CPU affinities; AddressSanitizer passes all 18 at 1/12 workers. Coverage includes reordered records, endpoint identities, decimal boundaries and full uint32 values. Full SHA-256: `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. Candidate source: 397 Rust plus nine Dockerfile lines. Rare same-length fingerprint collisions remain possible; universal exact matching is not claimed.
