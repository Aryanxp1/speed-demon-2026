# Track B: Index — Team Lex

Rust 2021, standard library only. Combines the earlier Team Lex Rust exact inline-key and input/output behavior with the user-provided Thor phased index. Unnecessary source comments are removed; provenance and measurements remain in PERF.md.

The engine reads queries first, scans the corpus in batches and uses sparse postings or transposed 64-document bitmaps. A cache-line bucket contains three exact short keys, hash filters and actions. Longer words use exact normalized-byte checks, including collision and overflow probing. Phrase hashes are a fast filter followed by exact words. Boolean operators remain strictly left to right. Phase sizes adapt to dense inputs so document offsets remain within sixteen bits.

The pinned offline Docker build links statically with AVX2/AES/VAES support. Workers are capped at twelve. Use --data and --out, or DATA_DIR/OUT_DIR; SD_DATA and SD_OUT are also supported. Defaults are /data and /out.

Reference and AddressSanitizer checks cover 24 datasets plus independent alphanumeric and leading-zero cases. Forced dictionary and phrase hash collisions also pass. The Thor folder's reported 78.57 points is historical evidence for its source, not a result claimed for this merge. Official results are recorded in PERF.md.
