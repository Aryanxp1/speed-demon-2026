# Track B: Index - Team Lex

This version adapts the user-supplied C++20 engine in `speed_demon_trackb_exp/tb2/track-b`. The previously active submission was Rust. Confirmed C++ result: **73.996964 points**, K=30.195296, run `run-b5cf222b8139` at `9dc2aee`. All gates passed; 0.160806 s and 41,443,328 measured bytes. This improves the incoming 73.103392-point best. The GCC16 comparison was slower and GCC15 is restored. A compact serial query-builder comparison is pending; 75 has not yet been reached. See [PERF.md](PERF.md).

The engine indexes only query terms, with exact short keys and full byte checks for CRC-key candidates. A corpus sample chooses hot bitmap rows, sparse per-block sets, phrase anchors and safe Boolean guards; sampling changes the amount of work, never the answers. Workers claim document ranges, evaluate Boolean operators strictly left to right and verify phrases against token-ID tapes. Per-query totals and the smallest ten IDs are merged exactly.

Dictionary prefetching is bounded to sixteen tokens ahead. The calling thread participates in every parallel phase. SIMD tokenization is ASCII-only; unknown and overlength tokens retain their positions. The program supports `--data`, `--out`, `SD_DATA` and `SD_OUT`, defaulting to `/data` and `/out`. It uses at most twelve available workers.

No external dependencies. Pinned GCC15 builds with native optimization and LTO; the pinned Debian runtime contains the executable. No PGO data, stored answers or input-specific lookup tables are used.

The core passed optimized, ASan/UBSan and TSan verification on 23 datasets at 1, 4 and 12 workers, including an actual CRC collision checked against independent reference results. The full published output hash remains unchanged. License attribution is retained in [LICENSE](LICENSE).
