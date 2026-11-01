# Track B: Index

The task, the input files and the exact output format are in the [spec](https://github.com/hackathon-raptors/speed-demon-2026/blob/main/specs/b-index.md).

## Approach

One Rust source file, with no external crates. The current implementation adapts the Track B snapshot supplied by the user from `C:/Users/aryan/OneDrive/Videos/aryann/aryan`; its diagnostic second answer pass has been removed.

1. **Queries first.** Only queried terms are indexed. A CRC hash of the first 16 lowercased bytes selects a slot; all 32 bytes are compared to establish equality, including hash collisions. Dictionary sizing leaves an empty slot even for tiny query files.
2. **Sample and plan.** Twelve workers sample their line-aligned corpus partitions. Common Boolean terms become bitmaps; each phrase uses its rarest term as an anchor. Terms are renumbered by role and frequency.
3. **Buffer document tokens.** AVX2 identifies token boundaries and an eight-token lookup pipeline overlaps dictionary misses. Each document becomes a reusable token-ID buffer. Long or unqueried tokens retain a sentinel position, so phrases cannot bridge them. Per-document bookkeeping deduplicates list terms, gathers bitmap bits, and verifies phrase anchors against exact token-ID slices.
4. **Compact postings.** List postings use gap-coded 16-bit units in 32 streams with 64 KiB reserved chunks, then scatter into contiguous per-term segments. Planning state is released before scanning; dictionaries, phrase-anchor metadata and scan counts are released before final posting allocation.
5. **Reuse workers.** One persistent pool serves sampling, scanning, scattering and answers. Completion barriers keep scoped jobs alive until every worker finishes; completed jobs are cleared. Panics abort before borrowed job data can unwind.
6. **Answer queries.** Boolean operators run strictly left to right. Large list operations switch to bitmaps; AVX2 computes bitmap counts and ID sums. Bitmap rows are padded to whole vectors. Answers are built in parallel chunks and written through a buffered writer, avoiding a second full output copy.

## Dependencies

None. The Rust standard library, `std::arch` intrinsics, and `mmap` and `mallopt` declared by hand.

## Validation and measurements

Official best: **73.103392 points**, run `run-237d62c0cf7c`, implementation `2c9f85b`; all gates passed. See [PERF.md](PERF.md) for experiments and official board results. Local comparisons validate exact bytes against the Python reference; local launch-inclusive timings and sampled anonymous RSS are distinct from the board's execution time and cgroup memory measurements.
