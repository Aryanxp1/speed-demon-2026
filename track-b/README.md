# Track B: Index

The task, the input files and the exact output format are in the [spec](https://github.com/hackathon-raptors/speed-demon-2026/blob/main/specs/b-index.md).

## Approach

One Rust file, `src/main.rs`, written from scratch to be short (about 600 rustfmt lines against 968 for the C++ it replaced, which is in the git history up to commit 5c116c9), with no crates.

1. **Queries first.** `queries.txt` is parsed into a term table keyed by the token's 32 lowercased bytes (one AVX2 compare per probe, the top 32 hash bits in the slot so a mismatch rarely loads a key). Tokens not in the table are never indexed.
2. **Sample.** Each of 12 threads walks the first 1/32 of its line-aligned share of `docs.txt` and counts the documents holding each term. A BOOL term in over 1/32 of the sampled documents becomes a bitmap and every phrase's rarest term becomes its anchor. A frequency histogram then renumbers terms by class (bitmaps, lists, lists that anchor phrases, anchors only, phrase-only terms) and most frequent first within a class, so the scan reads a token's role off its id and the state of common terms shares a few cache lines.
3. **Scan.** Token bounds come from one AVX2 class mask (token bytes and LF) per 64-byte window; each token's slot is prefetched and looked up 8 tokens later. At every occurrence of an anchor the scan checks that anchor's phrases in place, reading the neighbouring tokens back and forward up to the document's TAB and LF while they are in cache; matches are kept as (query, document) pairs. The per-token handler is inlined. Tokens absent from the query dictionary return immediately. Bitmap terms take a dedicated fast path, gathering one word per 64 documents; repeated occurrences simply OR the same local bit. List terms are deduplicated per document and written as 16-bit gap-coded units to one of 32 buckets, in 256 KiB reserved buffers. Large document gaps use an escape unit.
4. **Lists.** Each term's postings lie in one stretch, thread by thread, as 16-bit offsets from each thread's first document, with the 12 segment lengths beside it; the stretches go bucket by bucket, so each thread's scatter of a bucket writes one small region.
5. **Reclaim scan state.** Sampling and classification buffers are released before scanning; the dictionary and phrase-anchor metadata are released once the scan workers join, before allocating final postings.
6. **Answers.** BOOL queries run strictly left to right on sorted lists (branch-free merges, or a binary search per document when one side is 16 times longer), switching to a bitmap once a bitmap term starts the query or is OR-ed in; count, sum and the ten smallest ids of a bitmap come from popcounts. Phrase queries read their matches. Queries are answered 64 at a time on 12 threads. Boolean and phrase results use the same allocation-free integer formatter.

## Dependencies

None. The Rust standard library, `std::arch` intrinsics, and `mmap` and `mallopt` declared by hand.

## With another 24 hours

The scan is still most of the run, and it is now bound by memory accesses (the term table and the per-term state), not by tokenizing: a faster tokenizer on its own cut 35% of tokenizing time single-threaded but almost nothing end to end. Peak memory is now the scan output (36 MB) plus the bitmaps (12 MB) and per-thread scan state; going further toward the 44 MB cap would need a scan output smaller than three bytes per posting. Lines could be trimmed.
