# Track C: Join â€” Team Lex

Best confirmed result: **80.087186 points**, **K=39.970882**, run `run-b70a81fe63e9`, implementation `b4fd1dd`. All official gates passed; runtime 29.101 ms. A final PGO build follow-up is awaiting evaluation; 82 remains a stretch target. See [PERF.md](PERF.md) for official results and rejected experiments.

One C++20 solver, no external runtime dependencies, built statically with pinned GCC16. The unchanged organizer generator provides offline full seed-17 training for the PGO build; data/profiles are absent from the runtime image. Both inputs are mapped with zero padding for wide reads. Transparent huge pages are disabled to keep the small tables compact.

## Pipeline

1. Workers scan the region file to flag country codes. A barrier assigns compact country indices in output order. A second pass builds a byte-sized direct region lookup. Dense shared tables are pre-faulted with the named Linux MADV_POPULATE_WRITE advice.
2. Split records after CRLFs, which the generator guarantees never occur inside quoted fields. Workers claim pieces dynamically. Quote scanning handles doubled quotes, embedded LF, mixed terminators, BOMs and empty fields. SIMD multiply-adds decode IDs, region IDs and money into exact integers.
3. Pipeline region lookup, name interning and bitmap marking two records ahead. The shared name table compares groups of four 64-bit fingerprints loaded atomically, and CAS assigns slots. Fingerprints include the entire raw name and its length. They retain the incoming design's theoretical 64-bit hash-collision limitation; they are not collision-free byte comparisons.
4. Atomic bitmap marks identify first sightings directly, so each worker updates its private distinct count without a final bitmap scan. Numeric totals also remain private until merging.
5. The last finisher in each group of four pieces releases the completed mapping interior. Atomic completion publication prevents premature unmapping, and retained boundary pages protect neighbouring wide loads. Merge totals and format country-sorted output with integer arithmetic.

## Verification

Optimized, AddressSanitizer/UndefinedBehaviorSanitizer and ThreadSanitizer builds passed 35 datasets at 1, 4 and 12 workers. These include all public scales, another full-scale seed, additional generated seeds, 24 independent Python-reference fixtures and three page-boundary fixtures with roughly 4-KiB names. Cases include names differing after byte 24, long trailing spaces, eight-digit IDs, unknown regions, money bounds and hostile CSV fields. Full seed-1 output SHA-256 is `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536`.

Local timings are WSL comparisons, not scoring-host measurements. Official evaluation verifies the container build and correctness/timeout/sandbox gates. Only Track C changed in this sprint.
