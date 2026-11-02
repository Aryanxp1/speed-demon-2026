# Team Lex: Track D

Exact directed shortest paths, C++20, no external runtime dependencies.

The current candidate combines the user-supplied `Downloads/td/td/track-d` construction and saturated-label query engine with Team Lex's unique atomic vertex ownership, AVX2 input scanner, checked mappings, compact label directories and reusable twelve-worker phase pool.

Parsing builds four-slot raw rows with exact spill and heavy-weight tables. Contraction gathers locality-ordered regions. Equal final-ID slices fuse edge renaming with reverse-edge counting; cross-slice edges use owner mailboxes. Existing mapped pages are recycled into graph arrays. Every source vertex has exactly one owner.

Queries use 16-bit saturating labels for paths below 65535, and rerun saturated or uncertain results with full 64-bit labels. An exact 256-settle forward probe precedes balanced bidirectional search. A 64-bucket queue retains its exact far-distance heap. Zero weights, unreachable paths, duplicate edges and large distances remain supported.

The pinned offline Docker build and organizer seed-17 PGO generator are unchanged. Environment: `SD_DATA`, `SD_OUT`, `SD_THREADS`; output: `distances.txt`. Worker counts are bounded to twelve.

Prior official best: **75.016414 points / K=31.646688**, commit `c9fb522`, run `run-11935518a85b`. The current candidate awaits official evaluation; the source project's reported 75.839 is not a new Team Lex result.

Native PGO, ASan/UBSan and TSan pass public inputs, independent oracle cases, saturation/overflow boundaries and concurrency stress. Local PGO medians improved 22.3% and 16.7% on two full inputs, with higher sampled anonymous memory. See PERF.md for complete tradeoffs, provenance and validation. Scoring-box K decides promotion.
