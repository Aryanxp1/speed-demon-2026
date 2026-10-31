// Speed Demon 2026, Track C (Join).
#include <algorithm>
#include <array>
#include <atomic>
#include <charconv>
#include <cstring>
#include <fcntl.h>
#include <immintrin.h>
#include <mutex>
#include <string_view>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <thread>
#include <unistd.h>
#include <vector>

using u8 = uint8_t;
using u32 = uint32_t;
using u64 = uint64_t;
using i64 = int64_t;

constexpr u64 M1 = 0x0101010101010101ull, M8 = 0x8080808080808080ull;
inline u32 code3(const char *c) { return ((c[0] - 'A') << 10) | ((c[1] - 'A') << 5) | (c[2] - 'A'); }
inline u32 mask64(__m256i v) { return _mm256_movemask_pd(_mm256_castsi256_pd(v)); }


typedef u64 __attribute__((aligned(1))) u64_u;
inline u64 load8(const char* p) { return *(const u64_u*)p; }

template <class T>
inline T* zeroed(size_t n) {
    return (T *)mmap(0, n * sizeof(T), 3, 0x22, -1, 0);
}

template <class T>
inline void populate_share(T* p, size_t n, int t, int nt) {
    size_t pg = (n * sizeof(T) + 4095) >> 12, f = pg * t / nt, to = pg * (t + 1) / nt;
    if (to > f) madvise((char *)p + (f << 12), (to - f) << 12, 19);
}

const char *map_file(const char *path, size_t &size) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) _exit(1);
    size = lseek(fd, 0, SEEK_END);
    auto *p = (char *)mmap(0, size + 4096, 1, 0x22, -1, 0);
    if (size && mmap(p, size, 1, 0x12, fd, 0) == (void *)-1) _exit(1);
    return close(fd), p;
}

inline u32 match32(const char* p, char c) {
    __m256i v = _mm256_loadu_si256((const __m256i*)p);
    return _mm256_movemask_epi8(_mm256_cmpeq_epi8(v, _mm256_set1_epi8(c)));
}

inline u64 fold(u64 v, size_t n) {
    v &= 0x0F0F0F0F0F0F0F0F & ~u64(0) << 8 * (8 - n);
    v = (v * 10 + (v >> 8)) & 0x00FF00FF00FF00FF;
    v = (v * 100 + (v >> 16)) & 0x0000FFFF0000FFFF;
    return (v * 10000 + (v >> 32)) & 0xFFFFFFFF;
}

inline u64 digits(const char* s, const char* e) { return fold(load8(e - 8), e - s); }

inline u64 money(const char* s, const char* e) {
    u64 w = load8(e - 8);
    return fold((w & 0xFFFFFFFFFF) << 8 | (w & 0xFFFF000000000000), e - s - 1);
}

inline const char* find(const char* p, char c) {
    for (;; p += 64)
        if (u64 m = match32(p, c) | u64(match32(p + 32, c)) << 32)
            return p + __builtin_ctzll(m);
}

inline const char* comma_after(const char* p) {
    u64 x = load8(p) ^ 0x2C2C2C2C2C2C2C2Cull;
    return p + __builtin_ctzll((x - M1) & ~x & M8) / 8;
}

struct Text { const char* at; u32 raw, length; };

inline Text text(const char*& p) {
    const char* s = p;
    if (*s != '"') {
        p = find(s, ',') + 1;
        return {s, u32(p - 1 - s), u32(p - 1 - s)};
    }
    u32 pairs = 0;
    const char* q = s + 1;
    for (; (q = find(q, '"'))[1] == '"'; q += 2) ++pairs;
    p = q + 2;
    return {s + 1, u32(q - s - 1), u32(q - s - 1 - pairs)};
}

class NameTable {
public:
    static constexpr u32 BITS = 17, SLOTS = 1 << BITS;

    static u64 key(const char* s, u32 len) {
        static constexpr char K[32] = {-1, -1, -1, -1, -1, -1, -1, -1,
                                       -1, -1, -1, -1, -1, -1, -1, -1};
        __m128i m = _mm_loadu_si128((const __m128i *)(K + 16 - std::min(len, 16u)));
        __m128i head = _mm_and_si128(_mm_loadu_si128((const __m128i *)s), m);
        u64 h = (u64(_mm_cvtsi128_si64(head)) * 0x9E3779B97F4A7C15ull ^ len)
              + u64(_mm_extract_epi64(head, 1)) * 0xBF58476D1CE4E5B9ull;
        for (u32 i = 16; i < len; i += 8) {
            u64 w = load8(s + i);
            if (len - i < 8) w &= (u64(1) << 8 * (len - i)) - 1;
            h = (h ^ w) * 0x94D049BB133111EBull;
        }
        h = (h ^ h >> 31) * 0xD6E8FEB86659FD93ull;
        h ^= h >> 32;
        return h + (h == 0);
    }

    void prefetch(u64 k) const {
        _mm_prefetch((const char*)(&slots_[k >> (64 - BITS)]), _MM_HINT_T0);
    }

    u32 id(u64 k) {
        for (u32 i = k >> (64 - BITS) & ~3u;; i = (i + 4) & (SLOTS - 1)) {
            __m256i v = _mm256_load_si256((const __m256i*)&slots_[i]);
            u32 hit = mask64(_mm256_cmpeq_epi64(v, _mm256_set1_epi64x(k)));
            if (hit) return i + __builtin_ctz(hit);
            u32 free = mask64(_mm256_cmpeq_epi64(v, _mm256_setzero_si256()));
            for (; free; free &= free - 1) {
                u32 j = i + __builtin_ctz(free);
                u64 s = 0;
                if (slots_[j].compare_exchange_strong(s, k) || s == k) return j;
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
        count += t.count, id_sum += t.id_sum, name_bytes += t.name_bytes;
        note_bytes += t.note_bytes, distinct += t.distinct, amount_sum += t.amount_sum;
        amount_min = std::min(amount_min, t.amount_min);
        amount_max = std::max(amount_max, t.amount_max);
    }
};

struct NamesSeen {
    static constexpr u32 WORDS = NameTable::SLOTS / 64;
    std::atomic<u64>* bits = zeroed<std::atomic<u64>>(256 * WORDS);

    std::atomic<u64>& word(u32 c, u32 name) { return bits[c * WORDS + (name >> 6)]; }
    void prefetch(u32 c, u32 name) { __builtin_prefetch(&word(c, name), 1); }
    void mark(u32 c, u32 name) { word(c, name).fetch_or(1ull << (name & 63), std::memory_order_relaxed); }
    u64 count(u32 c) const {
        u64 n = 0;
        for (u32 w = 0; w < WORDS; ++w) n += __builtin_popcountll(bits[c * WORDS + w]);
        return n;
    }
};

void parse_records(const char* p, const char* end, const char* file_end,
                   const u8* country_of, size_t regions, NameTable& names,
                   NamesSeen& seen, Totals* totals) {
    constexpr u64 LAG = 16;
    struct Pending {
        u64 name;
        u32 region, id, name_bytes, note_bytes;
        i64 cents;
    } pending[LAG];
    struct Mark { u32 country, id; } marks[LAG];
    u64 n = 0, last = UINT64_MAX;
    auto advance = [&]() __attribute__((always_inline)) {
        u32 k = n % LAG;
        if (n >= 2 * LAG && n - 2 * LAG < last) seen.mark(marks[k].country, marks[k].id);
        if (n >= LAG && n - LAG < last) {
            const Pending& old = pending[k];
            u32 c = old.region < regions ? country_of[old.region] : 0;
            Totals& t = totals[c];
            ++t.count, t.id_sum += old.id, t.amount_sum += old.cents;
            t.amount_min = std::min(t.amount_min, old.cents);
            t.amount_max = std::max(t.amount_max, old.cents);
            t.name_bytes += old.name_bytes, t.note_bytes += old.note_bytes;
            marks[k] = {c, names.id(old.name)};
            seen.prefetch(c, marks[k].id);
        }
    };

    while (p < end) {
        if (u8(*p) == 0xEF) p += 3;
        const char* f1 = comma_after(p);
        u64 id = digits(p, f1);
        p = f1 + 1;
        Text name = text(p), note = text(p);
        const char* f4 = comma_after(p);
        u64 region = digits(p, f4);
        const char* amount = f4 + 1;
        u32 lf = match32(amount, '\n');
        const char* eol = lf ? amount + __builtin_ctz(lf) : file_end;
        const char* amount_end = eol - (eol[-1] == '\r');
        bool negative = *amount == '-';
        i64 cents = money(amount + negative, amount_end);
        if (negative) cents = -cents;

        u32 r = region < regions ? u32(region) : u32(regions);
        _mm_prefetch((const char*)(country_of + r), _MM_HINT_T0);
        advance();
        auto nk = NameTable::key(name.at, name.raw);
        pending[n++ % LAG] = {nk, r, u32(id), name.length, note.length, cents};
        names.prefetch(pending[(n - 1) % LAG].name);
        p = eol + 1;
    }
    for (last = n; n < last + 2 * LAG; ++n) advance();
}

inline std::pair<u64, u32> region_line(const char*& p) {
    u64 x = load8(p) ^ 0x2C2C2C2C2C2C2C2C;
    const char* c = p + __builtin_ctzll((x - 0x0101010101010101) & ~x & 0x8080808080808080) / 8;
    auto id = digits(p, c); p = c + 4; while (*p++ != '\n');
    return {id, code3(c + 1)};
}

struct Phase {
    const int pieces;
    std::atomic<int> claimed{0}, done{0};
    Phase(int p) : pieces(p) {}
    template <class F> void run(F&& f) {
        for (int k; (k = claimed++) < pieces;)
            if (f(k), ++done == pieces) done.notify_all();
        for (int d; (d = done) < pieces;) done.wait(d);
    }
};

auto split(const char* d, size_t from, size_t sz, int n, std::string_view dl) {
    std::vector<size_t> cut(n + 1, sz);
    cut[0] = from;
    for (int t = 1; t < n; ++t) {
        size_t c = std::max(cut[t - 1], from + (sz - from) * t / n);
        auto* hit = (char*)memmem(d + c, sz - c, dl.data(), dl.size());
        cut[t] = hit ? hit - d + dl.size() : sz;
    }
    return cut;
}

int main(int argc, char** argv) {
    prctl(PR_SET_THP_DISABLE, 1, 0, 0, 0);
    int nt = std::clamp((int)std::thread::hardware_concurrency(), 1, 12);
    const char* reg_file = argc > 1 ? argv[1] : "/data/regions.csv";
    const char* rec_file = argc > 2 ? argv[2] : "/data/records.csv";
    const char* out_file = argc > 3 ? argv[3] : "/out/aggregates.txt";

    size_t rsize, size;
    const char* regions = map_file(reg_file, rsize);
    const char* records = map_file(rec_file, size);

    const char* last = (const char*)memrchr(regions, '\n', rsize - 1) + 1;
    size_t nreg = atoll(last) + 1;
    u8* country_of = zeroed<u8>(nreg);
    size_t roff = strchr(regions, '\n') - regions + 1;
    auto rcut = split(regions, roff, rsize, nt, "\n");
    std::vector<std::atomic<u8>> present(1 << 15);
    std::array<u8, 1 << 15> index_of{};
    std::vector<u32> codes;
    std::once_flag numbered;
    auto number_countries = [&] {
        for (u32 k = 0; k < present.size(); ++k)
            if (present[k]) codes.push_back(k), index_of[k] = codes.size();
        if (codes.size() > 255) _exit(1);
    };

    size_t start = size ? (char*)memchr(records, '\n', size) - records + 1 : 0;
    const int pieces = 4 * nt;
    std::vector<size_t> cut = split(records, start, size, pieces, "\r\n");
    std::vector<std::array<Totals, 256>> totals(nt);
    NameTable names;
    NamesSeen seen;

    Phase flag{nt}, fill{nt}, parse{pieces}, count{256};
    auto worker = [&](int t) {
        flag.run([&](int k) {
            populate_share(country_of, nreg, k, nt);
            names.populate(k, nt);
            for (const char* p = regions + rcut[k]; p < regions + rcut[k + 1];)
                present[region_line(p).second].store(1, std::memory_order_relaxed);
        });
        std::call_once(numbered, number_countries);
        fill.run([&](int k) {
            populate_share(seen.bits, (codes.size() + 1) * NamesSeen::WORDS, k, nt);
            for (const char* p = regions + rcut[k]; p < regions + rcut[k + 1];) {
                auto [id, c] = region_line(p);
                country_of[id] = index_of[c];
            }
        });
        parse.run([&](int k) {
            parse_records(records + cut[k], records + cut[k + 1], records + size,
                          country_of, nreg, names, seen, totals[t].data());
        });
        count.run([&](int c) {
            if (c <= int(codes.size())) totals[t][c].distinct = seen.count(c);
        });
    };
    std::vector<std::thread> pool;
    for (int t = 1; t < nt; ++t) pool.emplace_back(worker, t);
    worker(0);
    for (auto& th : pool) th.join();

    char buf[65536];
    char* o = buf;
    auto put_num = [&](u64 v) { *o++ = ' ', o = std::to_chars(o, o + 32, v).ptr; };
    auto put_cents = [&](i64 c) {
        *o++ = ' ';
        if (c < 0) *o++ = '-', c = -c;
        o = std::to_chars(o, o + 32, u64(c) / 100).ptr;
        *o++ = '.', *o++ = '0' + (u64(c) % 100 / 10), *o++ = '0' + (u64(c) % 10);
    };

    for (u32 i = 1; i <= codes.size() + 1; ++i) {
        u32 c = i % (codes.size() + 1);
        Totals sum;
        for (const auto& th : totals) sum.add(th[c]);
        if (sum.count == 0) continue;
        if (c == 0) {
            memcpy(o, "_UNMATCHED", 10), o += 10;
        } else {
            u32 k = codes[c - 1];
            *o++ = 'A' + (k >> 10), *o++ = 'A' + ((k >> 5) & 31), *o++ = 'A' + (k & 31);
        }
        put_num(sum.count), put_num(sum.id_sum);
        put_cents(sum.amount_sum), put_cents(sum.amount_min), put_cents(sum.amount_max);
        put_num(sum.distinct), put_num(sum.name_bytes), put_num(sum.note_bytes);
        *o++ = '\n';
    }
    int fd = open(out_file, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) (void)!write(fd, buf, o - buf), close(fd);
    _exit(0);
}
