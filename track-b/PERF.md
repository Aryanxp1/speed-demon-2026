# Track B: Performance Log

Every step from the reference to your final version. Unofficial local measurements are clearly labeled.

| Step | Change | Run ID / Measurement | T (s) | M (MB) | Unofficial K | Speed Points |
|---|---|---|---|---|---|---|
| 0 | Official scoring box Python reference | `calibration-seed1.json` | 56.35 | 2,200 | 1.000 | 0.00 |
| 1 | Baseline Rust selective query inverted index + left-to-right boolean bitset evaluation + phrase candidate verification | Local seed-1 full | 49.85 | 545 | 1.182 | 3.63 |
| 2 | Sparse boolean optimization (direct bit ops for df < 256) + zero-heap stack-based phrase verification + ascending df candidate intersection | Local seed-1 full | 6.75 | 545 | 3.142 | 24.86 |
| 3 | Fast custom FNV-1a hasher (`FastHashMap`) replacing SipHash in query vocabulary lookups | Local seed-1 full | 4.82 | 545 | 3.708 | 28.46 |
| 4 | Multi-threaded parallel corpus ingest across line boundaries + parallel query evaluation (Current Submission) | Local seed-1 full | 3.07 | 532 | 4.457 | 32.45 |

### Verified Outputs
- **Seed 1 Tiny:** 100% byte-identical against Python reference.
- **Seed 1 Sample:** `eeeec1e9319da98a6f3c664c78f93e7078fd603c25c2bb3027174e81e3638a8f` (100% byte-identical).
- **Seed 1 Full:** `f86973c210228141bcb6f1f1b10188ad2a9b65f71e7cd7cf4590b38de4e80927` (100% byte-identical).
- **Holdout Seed 42 (Sample):** 100% byte-identical against Python reference.
- **Holdout Seed 999 (Sample):** 100% byte-identical against Python reference.
