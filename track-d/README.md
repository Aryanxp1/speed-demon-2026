# Team Lex: Track D

Exact directed shortest paths, C++17, zero runtime dependencies.

Adapted from the user-supplied `Downloads/td/td/track-d` solver: four-slot raw rows with exact spills, fused contraction/locality ordering, equal final-ID slices, fused edge remapping/reverse counting and mapped-page recycling. Team Lex retains unique atomic vertex claims, checked allocations/mappings and process-wide huge-page disabling.

Queries use 16-bit saturated labels for paths below 65535, an exact 256-settle forward probe, bidirectional search and a 2048-bucket queue with seven-bit keys. Saturated or uncertain results rerun with full 64-bit labels. General edge decoding remains available alongside the common 11-bit specialization. Vertex packing is checked. Zero weights, unreachable paths, duplicate edges and large distances remain supported.

The two-stage pinned GCC15/Debian13 build now matches the supplied source project's generic C++17 static build, without PGO. Phase threads are joined. Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS` (up to twelve); output: `distances.txt`.

Historical Team Lex best: **75.016414 points / K=31.646688**, source `c9fb522`, run `run-11935518a85b`. The initial adaptation scored 74.897713; this build-parity trial awaits official evaluation. The supplied project's reported 75.839 is provenance, not a new Team Lex measurement.

Native release, ASan/UBSan and TSan pass public inputs, independent oracle cases, saturation/overflow boundaries and concurrency stress. See PERF.md for full results and rejected trials. No ranking or higher official score is claimed before evaluation.
