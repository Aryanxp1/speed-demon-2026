# Track C: Join — Lex

Dependency-free Rust join and aggregation engine targeting Linux x86-64 with AVX2 and PCLMULQDQ.

## Approach

The engine maps the input files and constructs a direct region-to-country lookup from sorted region IDs. Workers parse CSV using 64-byte SIMD delimiter scans and carry-less-multiply quote masks. Record partitions use the generated dataset's CRLF boundaries; region partitions align to LF.

A concurrent exact-byte name table assigns IDs, and per-country atomic bitmaps count distinct names. The fast table has 524,288 slots. Bounded probing and standard-library maps/sets handle capacity overflow. Long-name suffixes reference the immutable mapped input. Integer cents preserve exact amount sums and extrema.

The `22347ff` failure is repaired: trailing LF no longer hides the last region, amount extrema start at proper sentinels, and competing name insertions cannot skip a just-claimed matching slot. CPU feature requirements are explicit in the parser and Docker build.

## Validation and performance

Tiny, sample, full, seeds 42/999, and official handwritten CSV fixtures match reference bytes. Full SHA-256: `d8088b1cd42ba1338b1345b107b55db7b96ef1f5066f3ff64fd26a8912a0f536`.

Five local full runs measured .15/.06/.06/.06/.06 s with maximum RSS 226.03 MB. These cached WSL measurements include mapped pages. The repaired version has not been evaluated on the official board; no 72+ score is claimed. See [PERF.md](PERF.md) for diagnosis, hashes, raw measurements and verification limits. External crate dependencies remain zero.
