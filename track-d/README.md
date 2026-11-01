# Team Lex: Track D

Current candidate targets higher K through unique atomic claims, a smaller exact queue ring, and a packed raw-to-final vertex map. Each raw vertex has one map writer; final IDs use three bytes when possible and four otherwise. Final IDs are written directly, avoiding a region-code decode for each remapped edge. Proven-unreachable narrow searches avoid unnecessary u64 retries; overflow and large finite results still use the exact 64-bit solver. Worker phases use standard joined threads to reduce source size. AVX2 parsing and native twelve-worker seed-17 PGO remain.

Two-full-seed confirmation, 41 interleaved rounds each, gave median paired runtime ratios 0.9820 (seed 1, 26/41 wins) and 0.9822 (seed 17, 25/41 wins). Absolute medians were 92.686/93.597 ms for baseline/candidate on seed 1 and 93.433/90.518 ms on seed 17; local scheduling variation is significant. Sampled anonymous peaks fell from about 56,312 to 54,320 KiB; estimated formatted S including build is 617 versus 634. These are local proxies, not an official 75-point claim.

Optimized, ASan/UBSan and ThreadSanitizer builds pass public tiny/sample/full, 70 existing differential fixtures, 88 supplied differential cases, 60 u32 boundary cases, 60 stress cases, cross-group and large-u64 fixtures at 1/4/12 workers. A forced four-byte-map ASan/UBSan build passes the same suite. Additional checks cover 600-node paths at/above the u32 sentinel and a 70,005-node skewed star.

Previous retained best: **74.428628 points, K=30.801549**, scored at `789ebdb` with source `f3c7439`; the earlier first-place result was 74.373046 / K=30.722809. New official evaluation pending. Docker and organizer generator bytes remain unchanged. See PERF.md for evidence, provenance and rejected experiments.

Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS` (default up to twelve, override clamped 1-64). Output: `/out/distances.txt` by default. Zero external dependencies.
