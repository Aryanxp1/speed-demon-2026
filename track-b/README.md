# Track B: Index — Team Lex

Rust 2021, standard library only. Combines the existing Team Lex Rust input/output support with the fast matching and phased index from the user-provided `Downloads/thortb/thortb/track-b` source. Unnecessary source comments are removed; provenance and measurements remain in PERF.md.

The engine reads queries first, scans the corpus in batches and uses sparse postings or transposed 64-document bitmaps. A cache-line bucket contains seven hash tags and eight actions, allowing one vector comparison per token. Full buckets continue probing. Phrases compare neighboring token hashes at their rarest word. Boolean operators remain strictly left to right. Phase sizes adapt to dense inputs so document offsets remain within sixteen bits.

Matching follows the supplied source's probabilistic hash behavior: different words with the same stored fingerprint can be confused. Full-word collision verification from the previous merge is removed in response to the user's explicit priority on points and K. This is not a guarantee of exact matching for every possible input.

The pinned offline Docker build links statically with AVX2/AES/VAES support. Workers are capped at twelve. Use --data and --out, or DATA_DIR/OUT_DIR; SD_DATA and SD_OUT are also supported. Defaults are /data and /out.

Reference checks cover 25 datasets at 1/4/12 workers, and the same 25 pass AddressSanitizer. Cases include full/sample/tiny inputs, independent generated inputs, token lengths, long shared prefixes, dense documents, Unicode separators, case folding, alphanumeric keys and leading zeros. These checks do not eliminate the fingerprint collision limitation. The supplied source's reported 78.57 points is historical evidence, not a result claimed for this integration. Official results are recorded in PERF.md.

Verified official result: **77.986833 points, K=36.285796**, run `run-0e4ed240cb36`; all gates passed. This improves the previous team best by 3.491353 points.
