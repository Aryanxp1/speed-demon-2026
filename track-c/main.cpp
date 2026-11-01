#include <bits/stdc++.h>
#include <fcntl.h>
#include <immintrin.h>
#include <sched.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <unistd.h>

using u8 = uint8_t;
using u32 = uint32_t;
using u64 = uint64_t;
using i64 = int64_t;

typedef u64 __attribute__((aligned(1))) u64_u;
#define load8(p) (*(const u64_u *)(p))
#define mask64(v) _mm256_movemask_pd(_mm256_castsi256_pd(v))
#define code3(c) (((c)[0] - 'A') * 1024 + ((c)[1] - 'A') * 32 + ((c)[2] - 'A'))
#define zeroed(T, n) (T *)mmap(0, (n) * sizeof(T), 3, 0x22, -1, 0)
constexpr u64 M1 = 0x0101010101010101ull, M8 = 0x8080808080808080ull;

inline void pop_share(void *p, size_t n, int t, int nt) {
  size_t pg = (n + 4095) >> 12, f = pg * t / nt, to = pg * (t + 1) / nt;
  to > f ? (void)madvise((char *)p + (f << 12), (to - f) << 12,
                         MADV_POPULATE_WRITE)
         : (void)0;
}
const char *map_file(const char *path, size_t &size) {
  int fd = open(path, O_RDONLY);
  (fd < 0) ? _exit(1) : (void)0;
  size = lseek(fd, 0, SEEK_END);
  auto *p = (char *)mmap(0, size + 4096, 1, 0x22, -1, 0);
  (size && mmap(p, size, 1, 0x12, fd, 0) == (void *)-1) ? _exit(1) : (void)0;
  return close(fd), p;
}

inline u64 decimal8(u64 v) {
  __m128i pairs =
      _mm_maddubs_epi16(_mm_cvtsi64_si128(v), _mm_set1_epi16(0x010A));
  __m128i quads = _mm_madd_epi16(pairs, _mm_set1_epi32(0x00010064));
  return u32(_mm_cvtsi128_si32(
      _mm_madd_epi16(_mm_packs_epi32(quads, _mm_setzero_si128()),
                     _mm_set1_epi32(0x00012710))));
}

inline u64 money(const char *s, const char *e) {
  u64 w = load8(e - 8);
  u64 v = (w << 8 & 0xFFFFFFFFFF00 | w & 0xFFFF000000000000) &
          0x0F0F0F0F0F0F0F0F & ~0ull << 8 * (9 - (e - s));
  return decimal8(v);
}

inline std::pair<const char *, u64> parse_int_comma(const char *p) {
  u64 raw = load8(p), x = raw ^ 0x2C2C2C2C2C2C2C2Cull;
  u64 mask = (x - M1) & ~x & M8;
  u32 len = mask ? __builtin_ctzll(mask) >> 3 : 8;
  u64 v = (raw << ((8 - len) << 3)) & 0x0F0F0F0F0F0F0F0Full;
  return {p + len, decimal8(v)};
}

inline const char *find_quote(const char *p) {
  __m256i q = _mm256_set1_epi8('"');
  for (;; p += 32)
    if (u32 m = _mm256_movemask_epi8(
            _mm256_cmpeq_epi8(_mm256_loadu_si256((const __m256i *)p), q)))
      return p + __builtin_ctz(m);
}

struct Text {
  const char *at;
  u32 raw, len;
};
inline Text text_fast(const char *&p) {
  const char *s = p;
  if (__builtin_expect(*s != '"', 0)) {
    const char *c = (const char *)rawmemchr(s, ',');
    return p = c + 1, Text{s, u32(c - s), u32(c - s)};
  }
  const char *q = find_quote(s + 1);
  if (__builtin_expect(q[1] != '"', 1))
    return p = q + 2, Text{s + 1, u32(q - s - 1), u32(q - s - 1)};
  u32 pairs = 1;
  for (q += 2; (q = find_quote(q))[1] == '"'; q += 2)
    ++pairs;
  return p = q + 2, Text{s + 1, u32(q - s - 1), u32(q - s - 1 - pairs)};
}

struct NameTable {
  static constexpr u32 BITS = 17, SLOTS = 1 << BITS;
  static __attribute__((always_inline)) inline u64 key(const char *s, u32 len) {
    static constexpr u64 K[4] = {~0ull, ~0ull, 0, 0};
    __m128i h128 =
        _mm_and_si128(_mm_loadu_si128((const __m128i *)s),
                      _mm_loadu_si128((const __m128i *)((const char *)K + 16 -
                                                        std::min(len, 16u))));
    u64 h = (u64(_mm_cvtsi128_si64(h128)) * 0x9E3779B97F4A7C15ull ^ len) +
            u64(_mm_extract_epi64(h128, 1)) * 0xBF58476D1CE4E5B9ull;
    if (__builtin_expect(len > 16, 0)) {
      u64 tail = 0;
      memcpy(&tail, s + 16, std::min(len - 16, 8u));
      h = (h ^ tail) * 0x94D049BB133111EBull;
      for (u32 off = 24; off < len; off += 8) {
        tail = 0;
        memcpy(&tail, s + off, std::min(len - off, 8u));
        h = (h ^ tail) * 0x94D049BB133111EBull;
      }
    }
    h = (h ^ h >> 31) * 0xD6E8FEB86659FD93ull;
    return (h ^ h >> 32) + (h == 0);
  }
  void prefetch(u64 k) const {
    _mm_prefetch((const char *)(&slots_[k >> (64 - BITS)]), _MM_HINT_T0);
  }
  __attribute__((always_inline)) inline u32 id(u64 k) {
    __m256i tgt = _mm256_set1_epi64x(k);
    for (u32 i = (k >> (64 - BITS)) & ~3u;; i = (i + 4) & (SLOTS - 1)) {
      __m256i v =
          _mm256_set_epi64x(slots_[i + 3].load(std::memory_order_relaxed),
                            slots_[i + 2].load(std::memory_order_relaxed),
                            slots_[i + 1].load(std::memory_order_relaxed),
                            slots_[i].load(std::memory_order_relaxed));
      u32 hit = mask64(_mm256_cmpeq_epi64(v, tgt));
      if (__builtin_expect(hit != 0, 1))
        return i + __builtin_ctz(hit);
      for (u32 f = mask64(_mm256_cmpeq_epi64(v, _mm256_setzero_si256())); f;
           f &= f - 1) {
        u32 j = i + __builtin_ctz(f);
        u64 s = 0;
        if (slots_[j].compare_exchange_strong(s, k,
                                              std::memory_order_relaxed) ||
            s == k)
          return j;
      }
    }
  }
  std::atomic<u64> *slots_ = zeroed(std::atomic<u64>, SLOTS);
};

struct Totals {
  u64 count = 0, id_sum = 0, name_bytes = 0, note_bytes = 0, distinct = 0;
  i64 amount_sum = 0, amount_min = INT64_MAX, amount_max = INT64_MIN;
  void add(const Totals &t) {
    count += t.count, id_sum += t.id_sum, name_bytes += t.name_bytes,
        note_bytes += t.note_bytes, distinct += t.distinct,
        amount_sum += t.amount_sum,
        amount_min = std::min(amount_min, t.amount_min),
        amount_max = std::max(amount_max, t.amount_max);
  }
};

static constexpr u32 WORDS = NameTable::SLOTS / 64;

void parse_records(const char *p, const char *end, const char *file_end,
                   const u8 *country_of, size_t regions, NameTable &names,
                   std::atomic<u64> *seen_bits, Totals *totals) {
  constexpr u64 LAG = 2;
  u64 p_name[LAG];
  u32 p_reg[LAG], p_id[LAG], p_nb[LAG], p_tb[LAG], m_c[LAG], m_id[LAG];
  i64 p_cents[LAG];
  u64 n = 0, last = UINT64_MAX;

  auto advance = [&]() __attribute__((always_inline)) {
    u32 k = n % LAG;
    if (n >= 2 * LAG && n - 2 * LAG < last) {
      u32 c = m_c[k], id = m_id[k];
      u64 mask = 1ull << (id & 63);
      totals[c].distinct += !(seen_bits[c * WORDS + (id >> 6)].fetch_or(
                                  mask, std::memory_order_relaxed) &
                              mask);
    }
    if (n >= LAG && n - LAG < last) {
      u32 c = country_of[p_reg[k]], id = names.id(p_name[k]);
      Totals &t = totals[c];
      ++t.count, t.id_sum += p_id[k], t.amount_sum += p_cents[k],
          t.amount_min = std::min(t.amount_min, p_cents[k]),
          t.amount_max = std::max(t.amount_max, p_cents[k]),
          t.name_bytes += p_nb[k], t.note_bytes += p_tb[k], m_c[k] = c,
          m_id[k] = id,
          __builtin_prefetch(&seen_bits[c * WORDS + (id >> 6)], 1);
    }
  };

  __m128i lf_vec128 = _mm_set1_epi8('\n');
  while (p < end) {
    _mm_prefetch(p + 2048, _MM_HINT_T0);
    if (__builtin_expect(u8(*p) == 0xEF, 0))
      p += 3;
    auto [f1, id] = parse_int_comma(p);
    p = f1 + 1;
    Text name = text_fast(p), note = text_fast(p);
    auto [f4, region] = parse_int_comma(p);
    const char *amount = f4 + 1;
    __m128i v = _mm_loadu_si128((const __m128i *)amount);
    u32 lf = _mm_movemask_epi8(_mm_cmpeq_epi8(v, lf_vec128));
    const char *eol =
        __builtin_expect(lf != 0, 1) ? amount + __builtin_ctz(lf) : file_end;
    const char *amount_end = eol - (eol[-1] == '\r');
    bool neg = *amount == '-';
    i64 cents = money(amount + neg, amount_end);
    neg ? (cents = -cents) : 0;

    u32 r = std::min<size_t>(region, regions);
    _mm_prefetch((const char *)(country_of + r), _MM_HINT_T0), advance();
    auto nk = NameTable::key(name.at, name.raw);
    u32 slot = n++ % LAG;
    p_name[slot] = nk, p_reg[slot] = r, p_id[slot] = u32(id),
    p_nb[slot] = name.len, p_tb[slot] = note.len, p_cents[slot] = cents;
    names.prefetch(nk), p = eol + 1;
  }
  for (last = n; n < last + 2 * LAG; ++n)
    advance();
}

inline std::pair<u64, u32> region_line(const char *&p) {
  auto [comma, id] = parse_int_comma(p);
  p = comma + 5 + (comma[4] == '\r');
  return {id, code3(comma + 1)};
}

auto split(const char *d, size_t from, size_t sz, int n, const char *dl,
           size_t dl_sz) {
  std::vector<size_t> cut(n + 1, sz);
  cut[0] = from;
  for (int t = 1; t < n; ++t) {
    size_t c = std::max(cut[t - 1], from + (sz - from) * t / n);
    auto *hit = (char *)memmem(d + c, sz - c, dl, dl_sz);
    cut[t] = hit ? hit - d + dl_sz : sz;
  }
  return cut;
}

int main(int argc, char **argv) {
  prctl(PR_SET_THP_DISABLE, 1, 0, 0, 0);
  cpu_set_t cpus;
  sched_getaffinity(0, sizeof(cpus), &cpus);
  int nt = std::clamp(CPU_COUNT(&cpus), 1, 12);
  const char *reg_file = argc > 1 ? argv[1] : "/data/regions.csv",
             *rec_file = argc > 2 ? argv[2] : "/data/records.csv",
             *out_file = argc > 3 ? argv[3] : "/out/aggregates.txt";
  size_t rsize = 0, size = 0;
  const char *regions = map_file(reg_file, rsize),
             *records = map_file(rec_file, size);
  const char *last = (const char *)memrchr(regions, '\n', rsize - 1) + 1;
  size_t nreg = atoll(last) + 1;
  u8 *country_of = zeroed(u8, nreg + 1);
  size_t roff = strchr(regions, '\n') - regions + 1;
  auto rcut = split(regions, roff, rsize, nt, "\n", 1);
  std::vector<std::atomic<u8>> present(1 << 15);
  std::array<u8, 1 << 15> index_of{};
  std::vector<u32> codes;
  auto number_countries = [&]() noexcept {
    if (codes.empty())
      for (u32 k = 0; k < (1 << 15); ++k)
        if (present[k].load(std::memory_order_relaxed))
          codes.push_back(k), index_of[k] = codes.size();
  };
  std::barrier sync(nt, number_countries);
  size_t start = size ? (char *)memchr(records, '\n', size) - records + 1 : 0;
  const int pieces = 8 * nt;
  auto cut = split(records, start, size, pieces, "\r\n", 2);
  std::vector<std::array<Totals, 256>> totals(nt);
  NameTable names;
  std::atomic<u64> *seen_bits = zeroed(std::atomic<u64>, 256 * WORDS);
  std::atomic<int> claim{0};
  std::vector<std::atomic<u8>> released(pieces / 4);

  auto worker = [&](int t) {
    pop_share(country_of, nreg, t, nt);
    pop_share(names.slots_, NameTable::SLOTS * 8, t, nt);
    for (const char *p = regions + rcut[t]; p < regions + rcut[t + 1];)
      present[region_line(p).second].store(1, std::memory_order_relaxed);
    sync.arrive_and_wait();
    pop_share(seen_bits, (codes.size() + 1) * WORDS * 8, t, nt);
    for (const char *p = regions + rcut[t]; p < regions + rcut[t + 1];) {
      auto [id, c] = region_line(p);
      country_of[id] = index_of[c];
    }
    sync.arrive_and_wait();
    for (int k; (k = claim++) < pieces;) {
      parse_records(records + cut[k], records + cut[k + 1], records + size,
                    country_of, nreg, names, seen_bits, totals[t].data());
      // The last completed piece releases only the group interior; keep edge
      // pages mapped for wide loads by neighbouring live parsers.
      unsigned group = k / 4;
      u8 bit = 1u << (k % 4);
      if ((released[group].fetch_or(bit, std::memory_order_acq_rel) | bit) ==
          ((1u << 4) - 1)) {
        size_t first = (cut[group * 4] + 4096) & ~size_t(4095);
        size_t last = (cut[(group + 1) * 4] - 1) & ~size_t(4095);
        if (first < last)
          munmap((void *)(records + first), last - first);
      }
    }
  };

  std::vector<std::thread> pool;
  for (int t = 1; t < nt; ++t)
    pool.emplace_back(worker, t);
  worker(0);
  for (auto &th : pool)
    th.join();

  char buf[65536], *o = buf;
  auto put_u = [&](u64 v) { *o++ = ' ', o = std::to_chars(o, o + 32, v).ptr; };
  auto put_m = [&](i64 c) {
    *o++ = ' ', c < 0 ? (*o++ = '-', c = -c) : 0;
    o = std::to_chars(o, o + 32, u64(c) / 100).ptr;
    *o++ = '.', *o++ = '0' + u64(c) % 100 / 10, *o++ = '0' + u64(c) % 10;
  };
  for (u32 i = 1; i <= codes.size() + 1; ++i) {
    u32 c = i % (codes.size() + 1);
    Totals sum;
    for (int t = 0; t < nt; ++t)
      sum.add(totals[t][c]);
    if (sum.count == 0)
      continue;
    c == 0 ? (void)(memcpy(o, "_UNMATCHED", 10), o += 10)
           : (void)(*o++ = 'A' + (codes[c - 1] >> 10),
                    *o++ = 'A' + ((codes[c - 1] >> 5) & 31),
                    *o++ = 'A' + (codes[c - 1] & 31));
    put_u(sum.count), put_u(sum.id_sum), put_m(sum.amount_sum),
        put_m(sum.amount_min), put_m(sum.amount_max), put_u(sum.distinct),
        put_u(sum.name_bytes), put_u(sum.note_bytes), *o++ = '\n';
  }
  int fd = open(out_file, O_WRONLY | O_CREAT | O_TRUNC, 0644);
  (void)!write(fd, buf, o - buf), close(fd);
  _exit(0);
}
