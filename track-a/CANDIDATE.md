# Track A candidate — November 1, 2026

Historical validation of the November 1 baseline. The November 2 optimization and its official evaluation are recorded in [PERF.md](PERF.md); the current percentile implementation and offline profile-guided build are described in [README.md](README.md).

Status: activated in `main.cpp` after the user confirmed that Antigravity is assigned only to Track C. The static build was repeated after activation and the full seed-1 output hash matched. Earlier concurrent restoration was observed, but its cause is unknown.

Based on the user-supplied `C:/Users/aryan/Downloads/sd-ta-main/sd-ta-main/track-a/main.cpp`. Uses bounded streaming runs with an exact rerun on late records, private endpoint dictionary mirrors, four interleaved parse streams, AVX2 multi-rank selection, bitonic leaves, and integer formatting. Adaptations: disable transparent huge pages for the scoring environment; clamp workers to 1..12; fix inline and unused-parameter warnings. No dependencies added. Existing Dockerfile can compile the candidate when activated.

## Verification

- GCC Linux WSL release and static builds with -Wall -Wextra -Werror: clean.
- Byte-identical Python reference comparisons: full-seed1, sample-seed1, sample-seed42, tiny-seed1, tiny-seed999.
- Full seed-1 SHA-256: `495ca09682b9673ef836a506496177680bb4a3af54255174befddb1f83481cd3`. The older root briefing's full hash is incorrect.
- Supplied tests: dense blocks, synthetic multi-seed data, endpoint lengths 1..125 and shared prefixes, latency digit boundaries and uint32 extremes, group sizes and duplicate/ordered/adversarial selection patterns, reversed/sorted/shuffled sample inputs for seeds 4 and 5: passed.
- ASan + UBSan on sizes and endpoint-length differential tests: passed. TSan was not run locally.
- 21 alternating full-input runs on WSL, 12 workers: original median 17.922 ms, adapted candidate 14.886 ms, supplied version 15.003 ms. Every timed output hash checked.
- Separate 21 alternating runs with tmpfs output: original 18.121 ms, candidate 15.244 ms, supplied version 15.555 ms. Every timed output hash checked.
- Four-stream parsing transplanted alone: no clear gain. SIMD selection transplanted alone: about 6% faster; private cache added to that: no clear additional gain. Those partial variants were not selected.

These are local wall times including process launch, not official board measurements. Docker daemon was unavailable. No official 84+ score is claimed: formatted source size, cgroup peak memory, and the scoring CPU can change the ranking. The supplied log reports its race-free source at K=49.769039, but that is historical source evidence, not a measurement of this adaptation. No push performed. No Track B/C/D files edited.
