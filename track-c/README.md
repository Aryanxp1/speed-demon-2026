# Track C: Join - Team Lex

Best confirmed result: **80.470017 points**, **K=40.681817**, run `run-fe6bb38f59d2`, implementation `d752aea`. All official gates passed; runtime 28.118 ms and measured memory 8,732,672 bytes. This improves on the incoming 79.288196-point best. The 80-point target is achieved; the 82-point stretch target remains unmet. See [PERF.md](PERF.md) for results and rejected experiments.

One C++20 solver, no external runtime dependencies, built statically with pinned GCC16. The unchanged organizer generator provides offline full seed-17 training for the PGO build; data and profiles are absent from the runtime image. Both inputs are mapped with zero padding for wide reads. Transparent huge pages are disabled to keep small tables compact.

## Pipeline

1. Workers flag country codes in the region file. A barrier assigns compact indices in output order. A second pass builds a byte-sized direct region lookup. Dense shared tables are pre-faulted with the named Linux MADV_POPULATE_WRITE advice.
2. Split records after CRLFs, which the generator guarantees never occur inside quoted fields. Workers claim pieces dynamically. Quote scanning handles doubled quotes, embedded LF, mixed terminators, BOMs and empty fields. SIMD multiply-adds decode IDs, region IDs and money into exact integers.
3. Pipeline region lookup, name interning and bitmap marking two records ahead. The shared name table compares groups of four 64-bit fingerprints loaded atomically; CAS assigns slots. Fingerprints include the entire raw name and its length. They retain the incoming design's theoretical 64-bit hash-collision limitation; they are not collision-free byte comparisons.
4. Atomic bitmap marks identify first sightings directly, so each worker updates its private distinct count without a final bitmap scan. Numeric totals remain private until merging.
5. The last finisher in each group of four pieces releases the completed mapping interior. Atomic completion publication prevents premature unmapping, and retained boundary pages protect neighbouring wide loads. Merge totals and format country-sorted output with integer arithmetic.

## Verification

The selected PGO executable passed 35 datasets at 1, 4 and 12 workers. The same core solver passed optimized, AddressSanitizer/UndefinedBehaviorSanitizer and ThreadSanitizer checks on those datasets before the build-only PGO follow-up. Cases include two full-scale seeds, all public scales, additional generated seeds, 24 independent Python-reference fixtures and three page-boundary fixtures with roughly 4-KiB names. Coverage includes names differing after byte 24, trailing spaces, eight-digit IDs, unknown regions, money bounds and hostile CSV fields. Full seed-1 output SHA-256 is `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536`.

Local timings are WSL comparisons. The official result above confirms container build and correctness/timeout/sandbox gates. Only Track C changed in this sprint.
