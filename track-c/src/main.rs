use std::sync::{atomic::Ordering::*, atomic::*, Barrier, Mutex, OnceLock};
use std::{arch::x86_64::*, fs::File, ops::Range, os::fd::AsRawFd, ptr::*, slice, thread};

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn madvise(addr: *mut u8, len: usize, advice: i32) -> i32;
    fn prctl(option: i32, ...) -> i32;
    fn _exit(status: i32) -> !;
}

const BIAS: i64 = 1 << 40;
const HOT_WORDS: usize = 1630;

static NEXT: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
static REGION_GROUP: [AtomicU8; 1_000_001] = [const { AtomicU8::new(0) }; 1_000_001];
static LETTERS: [OnceLock<u8>; 17576] = [const { OnceLock::new() }; 17576];
static TOTALS: Mutex<[[i64; 8]; 256]> = Mutex::new([[0; 8]; 256]);
static COUNTRIES: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[repr(C)]
struct Tables([[AtomicU64; 8]; 1 << 14], [AtomicU64; 524288]);

struct Run(&'static [u8], &'static [u8], &'static Tables, Barrier);

unsafe fn load(p: *const u8) -> __m256i {
    _mm256_loadu_si256(p.cast())
}

unsafe fn equal_mask(chunk: __m256i, byte: u8) -> u64 {
    _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, _mm256_set1_epi8(byte as i8))) as u32 as u64
}

unsafe fn digits_before(end: *const u8, len: usize) -> u64 {
    let s = 64 - 8 * len;
    read_unaligned::<u64>(end.sub(8).cast()) >> s << s
}

#[inline(always)]
unsafe fn decode_digits(words: &[u64], numbers: &mut [u32], count: usize, limit: u32) {
    for at in (0..count).step_by(4) {
        let digits = _mm256_and_si256(load(words.as_ptr().add(at).cast()), _mm256_set1_epi8(15));
        let pairs = _mm256_maddubs_epi16(digits, _mm256_set1_epi16(0x010A));
        let fours = _mm256_madd_epi16(pairs, _mm256_set1_epi32(0x0001_0064));
        let fours = _mm256_packus_epi32(fours, fours);
        let eights = _mm256_madd_epi16(fours, _mm256_set1_epi32(0x0001_2710));
        let packed = _mm256_castsi256_si128(_mm256_permute4x64_epi64::<0b1000>(eights));
        let packed = _mm_min_epu32(packed, _mm_set1_epi32(limit as i32));
        _mm_storeu_si128(numbers.as_mut_ptr().add(at).cast(), packed);
    }
}

unsafe fn name_hash(name: *const u8, len: usize) -> (u64, usize) {
    static MASKS: [[u8; 32]; 2] = [[0xFF; 32], [0; 32]];
    if len >= 32 {
        let (a, b) = (name_hash(name, 31), name_hash(name.add(31), len - 31));
        let hash = (a.0.rotate_left(23) ^ b.0).wrapping_mul(0xE703_7ED1_A0B4_28DB);
        return (hash | 1, a.1 + b.1);
    }
    let bytes = _mm256_and_si256(load(name), load(MASKS.as_ptr().cast::<u8>().add(32 - len)));
    let quotes = equal_mask(bytes, b'"').count_ones() as usize;
    let key = _mm_set_epi64x(0x1D8E_4E27_C47D_124F, len as i64 ^ 0x5899_65CC_7537_4CC3);
    let low = _mm256_castsi256_si128(bytes);
    let high = _mm256_extracti128_si256::<1>(bytes);
    let mixed = _mm_aesenc_si128(_mm_xor_si128(low, key), high);
    let mixed = _mm_aesenc_si128(_mm_aesenc_si128(mixed, key), key);
    (_mm_cvtsi128_si64(mixed) as u64 | 1, quotes)
}

fn take_block(c: &AtomicUsize, s: usize, len: usize) -> Option<usize> {
    Some(c.fetch_add(s, Relaxed)).filter(|&a| a < len)
}

#[inline(always)]
fn matrix_index(group: usize, word: usize) -> usize {
    if word < HOT_WORDS {
        group * HOT_WORDS + word
    } else {
        256 * HOT_WORDS + group * (2048 - HOT_WORDS) + word - HOT_WORDS
    }
}

impl Run {
    unsafe fn intern(&self, hash: u64, spare: &mut Range<usize>) -> usize {
        let index = (hash >> 50) as usize;
        let tag = hash & (1 << 47) - 1 | 1;
        let bucket = &self.2 .0[index];
        let wanted = _mm256_set1_epi64x(tag as i64);
        let [low, high] = [0, 1].map(|half| {
            let p = bucket.as_ptr().cast::<__m256i>().add(half);
            let slots = _mm256_srli_epi64::<17>(_mm256_load_si256(p));
            _mm256_movemask_pd(_mm256_castsi256_pd(_mm256_cmpeq_epi64(slots, wanted)))
        });
        match low | high << 4 {
            0 => self.insert(index, tag, spare),
            f => (bucket[f.trailing_zeros() as usize].load(Relaxed) & 0x1FFFF) as usize,
        }
    }

    #[cold]
    unsafe fn insert(&self, index: usize, tag: u64, spare: &mut Range<usize>) -> usize {
        if spare.start >= spare.end {
            let first = NEXT[3].fetch_add(32, Relaxed);
            assert!(first + 32 <= 1 << 17);
            *spare = first..first + 32;
        }
        let claimed = tag << 17 | spare.start as u64;
        let slots = self.2 .0.as_flattened();
        for slot in slots[index * 8..].iter().chain(slots) {
            let mut h = slot.load(Relaxed);
            if h == 0 {
                match slot.compare_exchange(0, claimed, Relaxed, Relaxed) {
                    Ok(_) => return spare.next().unwrap(),
                    Err(old) => h = old,
                }
            }
            if h >> 17 == tag {
                return (h & 0x1FFFF) as usize;
            }
        }
        unreachable!()
    }

    unsafe fn start<'a>(&'a self, scope: &'a thread::Scope<'a, '_>, mut descendants: usize) {
        while descendants > 0 {
            let share = descendants.div_ceil(2);
            scope.spawn(move || unsafe { self.start(scope, share - 1) });
            descendants -= share;
        }
        let page = NEXT[2].fetch_add(1, Relaxed) << 21;
        if page < 2 << 21 {
            (*(self.2 as *const Tables as *const AtomicU8).add(page)).store(0, Relaxed);
        }
        while let Some(claim) = take_block(&NEXT[0], 1 << 16, self.1.len()) {
            let rest = &self.1[claim..];
            let mut line = rest.iter().position(|&c| c == b'\n').unwrap() + 1;
            while line <= 1 << 16 && line < rest.len() {
                let row = &rest[line..];
                let at = row.iter().position(|&c| c == b',').unwrap();
                let id = row[..at].iter().fold(0, |n, &c| n * 10 + c as usize - 48);
                let code = &row[at + 1..][..3];
                let cell = &LETTERS[code.iter().fold(0, |n, &c| n * 26 + (c - b'A') as usize)];
                let group = cell.get_or_init(|| {
                    let mut countries = COUNTRIES.lock().unwrap();
                    countries.push(String::from_utf8_lossy(code).into_owned());
                    countries.len() as u8 - 1
                });
                REGION_GROUP[id].store(*group, Relaxed);
                line += at + 5;
            }
        }
        if self.3.wait().is_leader() {
            madvise(self.1.as_ptr() as *mut u8, self.1.len(), 4);
        }
        prctl(41, 1usize, 0usize, 0usize, 0usize);
        let (input, len) = (self.0.as_ptr() as *mut u8, self.0.len());
        let mut stats = [[0i64; 8]; 256];
        let mut pos = [0u32; 191 * 5];
        let mut quotes = [(0u64, 0u32); 1024];
        let [mut hashes, mut lengths] = [[0u64; 191]; 2];
        let (mut signs, mut spare) = ([0i64; 191], 0..0);
        let (mut digits, mut numbers) = ([[0u64; 191]; 3], [[0u32; 191]; 3]);
        let ones = _mm_set1_epi8(-1);

        while let Some(claim) = take_block(&NEXT[1], 1 << 20, len) {
            let end = (claim + (1 << 20)).min(len);
            let from = claim.saturating_sub(1);
            let begins = |p: &[u8]| p[0] == b'\n' && (p[1].is_ascii_digit() || p[1] == 0xEF);
            let origin = (self.0[from..].windows(2).position(begins)).map_or(len, |p| from + p + 1);
            let (mut record, mut at) = (origin, origin);
            let (mut inside, mut quote_total, mut filled) = (0u64, 0u32, 0usize);
            while record < end {
                for _ in 0..32 {
                    let low = load(input.add(at));
                    let high = load(input.add(at + 32));
                    let mask = |byte| equal_mask(low, byte) | equal_mask(high, byte) << 32;
                    let (quote, comma, newline) = (mask(b'"'), mask(b','), mask(b'\n'));
                    let quoted = _mm_clmulepi64_si128(_mm_cvtsi64_si128(quote as i64), ones, 0);
                    let inside_now = _mm_cvtsi128_si64(quoted) as u64 ^ inside;
                    inside = ((inside_now as i64) >> 63) as u64;
                    let mut bits = (comma | newline) & !inside_now;
                    quotes[(at - origin) / 64 % 1024] = (quote, quote_total);
                    quote_total += quote.count_ones();
                    let found = bits.count_ones() as usize;
                    for extra in 0..8 {
                        *pos.get_unchecked_mut(filled + extra) = at as u32 + bits.trailing_zeros();
                        bits &= bits.wrapping_sub(1);
                    }
                    let mut extra = 8;
                    while bits != 0 {
                        *pos.get_unchecked_mut(filled + extra) = at as u32 + bits.trailing_zeros();
                        bits &= bits - 1;
                        extra += 1;
                    }
                    filled += found;
                    at += 64;
                }

                let quotes_up_to = |position: usize| {
                    let (quote, before) = quotes[(position - origin) / 64 % 1024];
                    let below = _bzhi_u64(quote, ((position - origin) % 64) as u32);
                    before.wrapping_add(below.count_ones())
                };
                let field = |a| (*pos.as_ptr().add(a * 5).cast::<[u32; 5]>()).map(|p| p as usize);
                let mut count = 0;
                let mut start = record;
                while count < filled / 5 && start < end {
                    let [comma1, _, comma3, comma4, newline] = field(count);
                    let bom = (*input.add(start) == 0xEF) as usize * 3;
                    digits[0][count] = digits_before(input.add(comma1), comma1 - start - bom);
                    digits[2][count] = digits_before(input.add(comma4), comma4 - comma3 - 1);
                    let stop = newline - (*input.add(newline - 1) == b'\r') as usize;
                    let neg = (*input.add(comma4 + 1) == b'-') as usize;
                    let cents = digits_before(input.add(stop), stop - comma4 - 1 - neg);
                    digits[1][count] = (cents & 0xFF_FFFF_FFFF) << 8 | cents >> 48 << 48;
                    signs[count] = -(neg as i64);
                    start = newline + 1;
                    count += 1;
                }
                for (kind, limit) in [u32::MAX, u32::MAX, 1_000_000].into_iter().enumerate() {
                    decode_digits(&digits[kind], &mut numbers[kind], count, limit);
                }
                let mut rank = quotes_up_to(record);
                for at in 0..count {
                    let [comma1, comma2, comma3, ..] = field(at);
                    let raw_name = comma2 - comma1 - 1;
                    let sub2 = raw_name.saturating_sub(2);
                    let (hash, inner) = name_hash(input.add(comma1 + 2), sub2);
                    let name_len = sub2 - inner / 2;
                    let before = std::mem::replace(&mut rank, quotes_up_to(comma3));
                    let name_quotes = inner + 2 * (raw_name != 0) as usize;
                    let note_q = rank.wrapping_sub(before) as usize - name_quotes;
                    let note_len = comma3 - comma2 - 1 - note_q / 2 - (note_q != 0) as usize;
                    _mm_prefetch::<_MM_HINT_T0>(self.2 .0[(hash >> 50) as usize].as_ptr().cast());
                    _mm_prefetch::<_MM_HINT_T0>(
                        REGION_GROUP.as_ptr().add(numbers[2][at] as usize).cast(),
                    );
                    hashes[at] = hash;
                    lengths[at] = name_len as u64 | (note_len as u64) << 32;
                }
                record = start;
                for at in 0..count {
                    let group = REGION_GROUP
                        .get_unchecked(numbers[2][at] as usize)
                        .load(Relaxed) as usize;
                    let amount = (numbers[1][at] as i64 ^ signs[at]) - signs[at];
                    let s = &mut stats[group];
                    s[0] += 1;
                    s[1] += numbers[0][at] as i64;
                    s[2] += amount;
                    s[3] += lengths[at] as i64;
                    s[4] = s[4].max(BIAS + amount);
                    s[5] = s[5].max(BIAS - amount);
                    let name = self.intern(hashes[at], &mut spare);
                    let word = matrix_index(group, name / 64);
                    _mm_prefetch::<_MM_HINT_ET0>(self.2 .1.as_ptr().add(word).cast());
                    hashes[at] = (word << 6 | name % 64) as u64;
                }
                for &bit in &hashes[..count] {
                    let word = self.2 .1.get_unchecked((bit >> 6) as usize);
                    word.fetch_or(1 << (bit & 63), Relaxed);
                }

                pos.copy_within(count * 5..filled, 0);
                filled -= count * 5;
            }
            madvise(input.add(claim), end - claim, 4);
        }
        let mut totals = TOTALS.lock().unwrap();
        for (tot, p) in totals.iter_mut().zip(&stats) {
            for (i, v) in p.iter().enumerate() {
                tot[i] = if i < 4 { tot[i] + v } else { tot[i].max(*v) };
            }
        }
    }
}

unsafe fn map(path: &str) -> &'static [u8] {
    let f = File::open(path).unwrap();
    let (fd, mut len) = (f.as_raw_fd(), f.metadata().unwrap().len() as usize);
    let size = len.next_multiple_of(4096) + 4096;
    let base = mmap(null_mut(), size, 3, 0x4022, -1, 0);
    assert!(base as isize != -1);
    if len > 0 {
        assert!(mmap(base, len, 3, 0x12, fd, 0) == base);
    }
    if len > 0 && *base.add(len - 1) != b'\n' {
        *base.add(len) = b'\n';
        len += 1;
    }
    slice::from_raw_parts(base, len)
}

unsafe fn zeroed<T>() -> &'static T {
    let r = mmap(null_mut(), size_of::<T>() + (1 << 21), 3, 0x4022, -1, 0);
    let a = r.add(r.align_offset(1 << 21));
    madvise(a, size_of::<T>(), 14);
    &*a.cast()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let reg = a.get(1).map_or("/data/regions.csv", |s| s);
    let rec = a.get(2).map_or("/data/records.csv", |s| s);
    let out = a.get(3).map_or("/out/aggregates.txt", |s| s);
    let n = thread::available_parallelism().map_or(12, |n| n.get());
    COUNTRIES.lock().unwrap().push("_UNMATCHED".into());
    let run = unsafe { Run(map(rec), map(reg), zeroed(), Barrier::new(n)) };
    thread::scope(|s| unsafe { run.start(s, n - 1) });

    let totals = TOTALS.lock().unwrap();
    let mut keys: Vec<_> = COUNTRIES.lock().unwrap().iter().cloned().zip(0..).collect();
    keys.sort_unstable();
    let mut out_str = String::with_capacity(32768);
    for (key, group) in keys {
        let [count, id_sum, amount, lengths, hi, lo, ..] = totals[group];
        let words = NEXT[3].load(Relaxed).div_ceil(64);
        let first = matrix_index(group, 0);
        let split = words.min(HOT_WORDS);
        let spill = if words > split {
            matrix_index(group, split)
        } else {
            0
        };
        let distinct: u32 = [(first, split), (spill, words - split)]
            .into_iter()
            .flat_map(|(at, len)| run.2 .1[at..at + len].iter())
            .map(|w| w.load(Relaxed).count_ones())
            .sum();
        let [sum, min, max] =
            [amount, BIAS - lo, hi - BIAS].map(|c| format!("{:.2}", c as f64 / 100.0));
        let (nb, tb) = (lengths as u32, lengths >> 32);
        if count > 0 {
            out_str += &format!("{key} {count} {id_sum} {sum} {min} {max} {distinct} {nb} {tb}\n");
        }
    }
    std::fs::write(out, out_str).unwrap();
    unsafe { _exit(0) }
}
