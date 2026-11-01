#include <algorithm>
#include <array>
#include <atomic>
#include <barrier>
#include <charconv>
#include <cstring>
#include <fcntl.h>
#include <immintrin.h>
#include <sched.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <thread>
#include <unistd.h>
#include <vector>

using u8 = uint8_t;
using u32 = uint32_t;
using u64 = uint64_t;
using i64 = int64_t;

typedef u64 __attribute__((aligned(1))) u64_u;
#define load8(p) (*(const u64_u*)(p))
#define mask64(v) _mm256_movemask_pd(_mm256_castsi256_pd(v))
inline u32 code3(const char* c) { return ((c[0] - 'A') << 10) | ((c[1] - 'A') << 5) | (c[2] - 'A'); }
constexpr u64 M1 = 0x0101010101010101ull, M8 = 0x8080808080808080ull;

template <class T> inline T* zeroed(size_t n) {
  void* p = mmap(0, n * sizeof(T), 3, 0x22, -1, 0);
  return p == (void*)-1 ? (_exit(1), (T*)0) : (T*)p;
}
template <class T> inline void populate_share(T* p, size_t n, int t, int nt) {
  size_t pg = (n * sizeof(T) + 4095) >> 12, f = pg * t / nt, to = pg * (t + 1) / nt;
  (to > f) ? (void)madvise((char*)p + (f << 12), (to - f) << 12, 19) : (void)0;
}
const char* map_file(const char* path, size_t& size) {
  int fd = open(path, O_RDONLY);
  (fd < 0) ? _exit(1) : (void)0;
  size = lseek(fd, 0, SEEK_END);
  auto* p = (char*)mmap(0, size + 4096, 1, 0x22, -1, 0);
  (size && mmap(p, size, 1, 0x12, fd, 0) == (void*)-1) ? _exit(1) : (void)0;
  return close(fd), p;
}

inline u64 fold(u64 v, size_t n) {
  v &= 0x0F0F0F0F0F0F0F0F & ~u64(0) << 8 * (8 - n);
  v = (v * 10 + (v >> 8)) & 0x00FF00FF00FF00FF;
  v = (v * 100 + (v >> 16)) & 0x0000FFFF0000FFFF;
  return (v * 10000 + (v >> 32)) & 0xFFFFFFFF;
}
inline u64 money(const char* s, const char* e) {
  u64 w = load8(e - 8);
  return fold((w & 0xFFFFFFFFFF) << 8 | (w & 0xFFFF000000000000), e - s - 1);
}

inline std::pair<const char*, u64> parse_int_comma(const char* p) {
  u64 raw = load8(p);
  u64 x = raw ^ 0x2C2C2C2C2C2C2C2Cull;
  u32 ctz = __builtin_ctzll((x - M1) & ~x & M8), len = ctz >> 3;
  u64 v = (raw << ((8 - len) << 3)) & 0x0F0F0F0F0F0F0F0Full;
  v = (v * 10 + (v >> 8)) & 0x00FF00FF00FF00FF;
  v = (v * 100 + (v >> 16)) & 0x0000FFFF0000FFFF;
  return {p + len, (v * 10000 + (v >> 32)) & 0xFFFFFFFF};
}

inline const char* find_quote(const char* p) {
  __m256i q = _mm256_set1_epi8('"');
  for (;; p += 32) {
    u32 m = _mm256_movemask_epi8(_mm256_cmpeq_epi8(_mm256_loadu_si256((const __m256i*)p), q));
    if (m) return p + __builtin_ctz(m);
  }
}

struct Text { const char* at; u32 raw, len; };
inline Text text_fast(const char*& p) {
  const char* s = p;
  if (__builtin_expect(*s != '"', 0)) {
    const char* c = (const char*)rawmemchr(s, ',');
    return p = c + 1, Text{s, u32(c - s), u32(c - s)};
  }
  const char* q = find_quote(s + 1);
  if (__builtin_expect(q[1] != '"', 1)) return p = q + 2, Text{s + 1, u32(q - s - 1), u32(q - s - 1)};
  u32 pairs = 1;
  for (q += 2; (q = find_quote(q))[1] == '"'; q += 2) ++pairs;
  return p = q + 2, Text{s + 1, u32(q - s - 1), u32(q - s - 1 - pairs)};
}

class NameTable {
public:
  static constexpr u32 BITS = 17, SLOTS = 1 << BITS;
  static __attribute__((always_inline)) inline u64 key(const char* s, u32 len) {
    alignas(16) static constexpr char K[32] = {-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1};
    __m128i m = _mm_loadu_si128((const __m128i*)(K + 16 - std::min(len, 16u)));
    __m128i head = _mm_and_si128(_mm_loadu_si128((const __m128i*)s), m);
    u64 h = (u64(_mm_cvtsi128_si64(head)) * 0x9E3779B97F4A7C15ull ^ len) + u64(_mm_extract_epi64(head, 1)) * 0xBF58476D1CE4E5B9ull;
    if (__builtin_expect(len > 16, 0)) {
      u64 w = load8(s + 16) & ((1ull << (8 * (len - 16))) - 1);
      h = (h ^ w) * 0x94D049BB133111EBull;
    }
    h = (h ^ h >> 31) * 0xD6E8FEB86659FD93ull;
    return (h ^ h >> 32) + (h == 0);
  }
  void prefetch(u64 k) const { _mm_prefetch((const char*)(&slots_[k >> (64 - BITS)]), _MM_HINT_T0); }
  __attribute__((always_inline)) inline u32 id(u64 k) {
    __m256i tgt = _mm256_set1_epi64x(k);
    for (u32 i = (k >> (64 - BITS)) & ~7u;; i = (i + 8) & (SLOTS - 1)) {
      u32 hit = 0, free = 0;
      for (u32 j = 0; j < 8; j += 4) {
        __m256i v = _mm256_load_si256((const __m256i*)&slots_[i + j]);
        hit |= mask64(_mm256_cmpeq_epi64(v, tgt)) << j;
        free |= mask64(_mm256_cmpeq_epi64(v, _mm256_setzero_si256())) << j;
      }
      if (hit) return i + __builtin_ctz(hit);
      for (u32 f = free; f; f &= f - 1) {
        u32 j = i + __builtin_ctz(f); u64 s = 0;
        if (slots_[j].compare_exchange_strong(s, k, std::memory_order_relaxed) || s == k) return j;
      }
    }
  }

  void populate(int t, int n) const { populate_share(slots_, SLOTS, t, n); }
private:
  std::atomic<u64>* slots_ = zeroed<std::atomic<u64>>(SLOTS);
};

struct Totals {
  u64 count = 0, id_sum = 0, name_bytes = 0, note_bytes = 0, distinct = 0;
  i64 amount_sum = 0, amount_min = INT64_MAX, amount_max = INT64_MIN;
  void add(const Totals& t) {
    count += t.count, id_sum += t.id_sum, name_bytes += t.name_bytes, note_bytes += t.note_bytes,
    distinct += t.distinct, amount_sum += t.amount_sum,
    amount_min = std::min(amount_min, t.amount_min), amount_max = std::max(amount_max, t.amount_max);
  }
};

struct NamesSeen {
  static constexpr u32 WORDS = NameTable::SLOTS / 64;
  std::atomic<u64>* bits = zeroed<std::atomic<u64>>(256 * WORDS);
  void prefetch(u32 c, u32 id) { __builtin_prefetch(&bits[c * WORDS + (id >> 6)], 1); }
  void mark(u32 c, u32 id) { bits[c * WORDS + (id >> 6)].fetch_or(1ull << (id & 63), std::memory_order_relaxed); }
  u64 count(u32 c) const {
    u64 n = 0;
    for (u32 w = 0; w < WORDS; ++w) n += __builtin_popcountll(bits[c * WORDS + w]);
    return n;
  }
};

void parse_records(const char* p, const char* end, const char* file_end,
                   const u8* country_of, size_t regions, NameTable& names,
                   NamesSeen& seen, Totals* totals) {
  constexpr u64 LAG = 4;
  struct Pending { u64 name; u32 region, id, name_b, note_b; i64 cents; } pending[LAG];
  struct Mark { u32 country, id; } marks[LAG];
  u64 n = 0, last = UINT64_MAX;
  auto advance = [&]() __attribute__((always_inline)) {
    u32 k = n % LAG;
    (n >= 2 * LAG && n - 2 * LAG < last) ? seen.mark(marks[k].country, marks[k].id) : (void)0;
    if (n >= LAG && n - LAG < last) {
      const Pending& old = pending[k];
      u32 c = country_of[old.region];
      Totals& t = totals[c];
      ++t.count, t.id_sum += old.id, t.amount_sum += old.cents,
      t.amount_min = std::min(t.amount_min, old.cents), t.amount_max = std::max(t.amount_max, old.cents),
      t.name_bytes += old.name_b, t.note_bytes += old.note_b,
      marks[k] = {c, names.id(old.name)}, seen.prefetch(c, marks[k].id);
    }
  };

  __m128i lf_vec128 = _mm_set1_epi8('\n');
  while (p < end) {
    if (__builtin_expect(u8(*p) == 0xEF, 0)) p += 3;
    auto [f1, id] = parse_int_comma(p);
    p = f1 + 1;
    Text name = text_fast(p), note = text_fast(p);
    auto [f4, region] = parse_int_comma(p);
    const char* amount = f4 + 1;
    __m128i v = _mm_loadu_si128((const __m128i*)amount);
    u32 lf = _mm_movemask_epi8(_mm_cmpeq_epi8(v, lf_vec128));
    const char* eol = __builtin_expect(lf != 0, 1) ? amount + __builtin_ctz(lf) : file_end;
    const char* amount_end = eol - (eol[-1] == '\r');
    bool negative = *amount == '-';
    i64 cents = money(amount + negative, amount_end);
    negative ? (cents = -cents) : 0;

    u32 r = std::min<size_t>(region, regions);
    _mm_prefetch((const char*)(country_of + r), _MM_HINT_T0), advance();
    auto nk = NameTable::key(name.at, name.raw);
    pending[n++ % LAG] = {nk, r, u32(id), name.len, note.len, cents};
    names.prefetch(nk), p = eol + 1;
  }
  for (last = n; n < last + 2 * LAG; ++n) advance();
}

inline std::pair<u64, u32> region_line(const char*& p) {
  auto [comma, id] = parse_int_comma(p);
  p = comma + 4; while (*p++ != '\n');
  return {id, code3(comma + 1)};
}

auto split(const char* d, size_t from, size_t sz, int n, const char* dl, size_t dl_sz) {
  std::vector<size_t> cut(n + 1, sz);
  cut[0] = from;
  for (int t = 1; t < n; ++t) {
    size_t c = std::max(cut[t - 1], from + (sz - from) * t / n);
    auto* hit = (char*)memmem(d + c, sz - c, dl, dl_sz);
    cut[t] = hit ? hit - d + dl_sz : sz;
  }
  return cut;
}

int main(int argc, char** argv) {
  prctl(PR_SET_THP_DISABLE, 1, 0, 0, 0);
  cpu_set_t cpus;
  sched_getaffinity(0, sizeof(cpus), &cpus);
  int nt = std::clamp(CPU_COUNT(&cpus), 1, 12);
  const char* reg_file = argc > 1 ? argv[1] : "/data/regions.csv";
  const char* rec_file = argc > 2 ? argv[2] : "/data/records.csv";
  const char* out_file = argc > 3 ? argv[3] : "/out/aggregates.txt";

  size_t rsize, size;
  const char* regions = map_file(reg_file, rsize);
  const char* records = map_file(rec_file, size);

  const char* last = (const char*)memrchr(regions, '\n', rsize - 1) + 1;
  size_t nreg = atoll(last) + 1;
  u8* country_of = zeroed<u8>(nreg + 1);
  size_t roff = strchr(regions, '\n') - regions + 1;
  auto rcut = split(regions, roff, rsize, nt, "\n", 1);
  std::vector<std::atomic<u8>> present(1 << 15);
  std::array<u8, 1 << 15> index_of{};
  std::vector<u32> codes;
  auto number_countries = [&]() noexcept {
    if (!codes.empty()) return;
    for (u32 k = 0; k < present.size(); ++k)
      if (present[k]) codes.push_back(k), index_of[k] = codes.size();
    (codes.size() > 255) ? _exit(1) : (void)0;
  };
  std::barrier sync(nt, number_countries);

  size_t start = size ? (char*)memchr(records, '\n', size) - records + 1 : 0;
  const int pieces = 8 * nt;
  auto cut = split(records, start, size, pieces, "\r\n", 2);
  std::vector<std::array<Totals, 256>> totals(nt);
  NameTable names;
  NamesSeen seen;
  std::atomic<int> claim{0};

  auto worker = [&](int t) {
    populate_share(country_of, nreg, t, nt);
    names.populate(t, nt);
    for (const char* p = regions + rcut[t]; p < regions + rcut[t + 1];)
      present[region_line(p).second].store(1, std::memory_order_relaxed);
    sync.arrive_and_wait();

    populate_share(seen.bits, (codes.size() + 1) * NamesSeen::WORDS, t, nt);
    for (const char* p = regions + rcut[t]; p < regions + rcut[t + 1];) {
      auto [id, c] = region_line(p);
      country_of[id] = index_of[c];
    }
    sync.arrive_and_wait();

    for (int k; (k = claim++) < pieces;) {
      parse_records(records + cut[k], records + cut[k + 1], records + size,
                    country_of, nreg, names, seen, totals[t].data());
    }
    sync.arrive_and_wait();

    for (u32 c = t; c <= codes.size(); c += nt)
      totals[t][c].distinct = seen.count(c);
  };

  std::vector<std::thread> pool;
  for (int t = 1; t < nt; ++t) pool.emplace_back(worker, t);
  worker(0);
  for (auto& th : pool) th.join();

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
    for (const auto& th : totals) sum.add(th[c]);
    if (sum.count == 0) continue;
    c == 0 ? (void)(memcpy(o, "_UNMATCHED", 10), o += 10)
           : (void)(*o++ = 'A' + (codes[c - 1] >> 10),
                    *o++ = 'A' + ((codes[c - 1] >> 5) & 31),
                    *o++ = 'A' + (codes[c - 1] & 31));
    put_u(sum.count), put_u(sum.id_sum), put_m(sum.amount_sum),
    put_m(sum.amount_min), put_m(sum.amount_max), put_u(sum.distinct),
    put_u(sum.name_bytes), put_u(sum.note_bytes), *o++ = '\n';
  }
  int fd = open(out_file, O_WRONLY | O_CREAT | O_TRUNC, 0644);
  fd >= 0 ? ((void)!write(fd, buf, o - buf), (void)close(fd)) : (void)0;
  _exit(0);
}
