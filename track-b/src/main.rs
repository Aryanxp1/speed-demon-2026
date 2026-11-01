use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering::*};
use std::sync::{Mutex, OnceLock};
use std::{arch::x86_64::*, fs::File, io::Write, os::fd::AsRawFd, ptr::null_mut, thread};

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn madvise(addr: *mut u8, len: usize, advice: i32) -> i32;
    fn prctl(option: i32, arg: usize, a: usize, b: usize, c: usize) -> i32;
    fn syscall(number: i64, ...) -> i64;
    fn memchr(text: *const u8, byte: i32, len: usize) -> *const u8;
}

const ANONYMOUS: i32 = 0x4022;

const HUGE_PAGE: usize = 1 << 21;
const CLAIM: usize = 1 << 18;

const PHASE_CLAIMS: usize = 200;

const HEADER: usize = 1;

const JOBS: usize = 16;
const TALLY_IDS: usize = 10;
const BATCH: usize = 1 << 14;
const BUCKETS: usize = 1 << 15;
const BUCKET_SLOTS: usize = 7;
const ID_BLOCK: usize = 32;
const MAX_TERMS: usize = BUCKETS * BUCKET_SLOTS + 64 * ID_BLOCK;
const AHEAD: usize = 12;
const HEAVY: u32 = 1 << 31;
const OVERFLOW: u32 = 1 << 30;
const ANCHOR: u32 = 1 << 29;
const MAX_HEAVY: usize = 4096;
const SHARDS: usize = 64;
const DOC_BITS: u32 = 18;
const CHUNK: usize = 64;
const QUERY_BATCH: usize = 64;

const WORKSPACE: usize = 32 << 20;

const MAX_WORDS: usize = 8;

#[repr(C, align(64))]
struct Bucket {
    tags: [u32; BUCKET_SLOTS + 1],
    actions: [u32; BUCKET_SLOTS + 1],
}

struct Buffers {
    starts: &'static mut [u32],
    ends: &'static mut [u32],
    lines: &'static mut [u32],
    hashes: &'static mut [u64],
    heavy_hits: &'static mut [u32],
    heavy_ends: &'static mut [u32],
    group: &'static mut [u64],
    list_hits: &'static mut [u64],
    anchor_hits: &'static mut [u64],
    counts: &'static mut [u32],
}

struct Term {
    first: u32,
    count: u32,
    phrases: u32,
    heavy: bool,
    in_bool: AtomicBool,
}

struct Phrase {
    term: u32,
    len: u32,
    pivot: u32,
    next: u32,
    words: [u64; MAX_WORDS],
}

#[derive(Clone, Copy, PartialEq)]
enum Op {
    And,
    Or,
    Not,
}

#[derive(Clone, Copy)]
enum Set {
    List(&'static [u16]),
    Bits(&'static [u64]),
}

#[derive(Default)]
struct Scratch {
    lists: [Vec<u16>; 2],
    bits: Vec<u64>,
    marks: Vec<u64>,
}

struct Tally {
    count: u32,
    sum: u64,
    first: [u32; TALLY_IDS],
}

impl Tally {
    fn add_list(&mut self, base: u32, list: &[u16]) {
        let sum = list.iter().map(|&doc| doc as u64).sum();
        let ids = list.iter().map(|&doc| doc as u32);
        self.add(base, list.len() as u64, sum, ids);
    }

    fn add(&mut self, base: u32, count: u64, sum: u64, ids: impl Iterator<Item = u32>) {
        let room = (self.count as usize).min(TALLY_IDS);
        for (slot, id) in self.first[room..].iter_mut().zip(ids) {
            *slot = base + id;
        }
        self.count += count as u32;
        self.sum += sum + base as u64 * count;
    }

    fn add_bits(&mut self, base: u32, bits: &[u64]) {
        let (count, sum) = unsafe {
            let table =
                |row: [i8; 16]| _mm256_broadcastsi128_si256(_mm_loadu_si128(row.as_ptr().cast()));
            let pick = |table, index| _mm256_shuffle_epi8(table, index);
            let ones = table([0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4]);
            let places = table([0, 0, 1, 1, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6]);
            let weights = _mm256_set1_epi64x(0x3830_2820_1810_0800);
            let (low, zero) = (_mm256_set1_epi8(15), _mm256_setzero_si256());
            let (mut counts, mut sums, mut offsets, mut starts) = (zero, zero, zero, zero);
            let mut first = _mm256_setr_epi64x(0, 64, 128, 192);
            for block in bits.chunks_exact(4) {
                let word = _mm256_loadu_si256(block.as_ptr().cast());
                let lo = _mm256_and_si256(word, low);
                let hi = _mm256_and_si256(_mm256_srli_epi16::<4>(word), low);
                let high_ones = pick(ones, hi);
                let per_byte = _mm256_add_epi8(pick(ones, lo), high_ones);
                let low_places = _mm256_add_epi8(pick(places, lo), pick(places, hi));

                let positions = _mm256_add_epi8(low_places, _mm256_slli_epi16::<2>(high_ones));
                let per_word = _mm256_sad_epu8(per_byte, zero);
                counts = _mm256_add_epi64(counts, per_word);
                sums = _mm256_add_epi64(sums, _mm256_sad_epu8(positions, zero));
                starts = _mm256_add_epi64(starts, _mm256_mul_epu32(per_word, first));
                let weighted = _mm256_maddubs_epi16(per_byte, weights);
                offsets =
                    _mm256_add_epi32(offsets, _mm256_madd_epi16(weighted, _mm256_set1_epi16(1)));
                first = _mm256_add_epi64(first, _mm256_set1_epi64x(256));
            }

            let total = |v: __m256i| {
                let lanes: [u32; 8] = std::mem::transmute(v);
                lanes.iter().map(|&lane| lane as u64).sum::<u64>()
            };
            (total(counts), total(sums) + total(starts) + total(offsets))
        };
        let ids = bits
            .iter()
            .enumerate()
            .filter(|(_, &word)| word != 0)
            .flat_map(|(index, &word)| {
                (0..64)
                    .filter(move |bit| word >> bit & 1 == 1)
                    .map(move |bit| index as u32 * 64 + bit)
            });
        self.add(base, count, sum, ids);
    }

    fn write(&self, out: &mut Vec<u8>) {
        put_number(out, self.count as u64);
        put_number(out, self.sum);
        for &doc in &self.first[..(self.count as usize).min(TALLY_IDS)] {
            put_number(out, doc as u64);
        }
        *out.last_mut().unwrap() = b'\n';
    }
}

struct Run {
    text: &'static [u8],
    docs: usize,
    words_per_bitmap: usize,
    buckets: *mut Bucket,
    bucket_count: usize,
    bucket_shift: u32,
    max_terms: usize,
    phase_claims: usize,
    terms: *mut Term,
    term_count: AtomicUsize,
    queries: &'static [u8],
    shares: Vec<OnceLock<Parsed>>,
    phrases: *mut Phrase,
    sample: &'static [AtomicU32],

    setup: [AtomicUsize; 6],

    phase: Vec<[AtomicUsize; 4]>,
    phase_base: Vec<u32>,

    arrived: AtomicU32,
    round: AtomicU32,
    bitmaps: *mut u64,
    rows: *mut u32,
    claimed: Vec<AtomicUsize>,
    postings: *mut u16,
    parts: Vec<Mutex<Vec<u8>>>,
    workspace: *mut u8,
}
unsafe impl Sync for Run {}

unsafe fn reserve<T>(count: usize, huge: bool) -> *mut T {
    let bytes = count * size_of::<T>();
    let region = mmap(null_mut(), bytes + HUGE_PAGE, 3, ANONYMOUS, -1, 0);
    assert!(region as isize != -1);
    let aligned = region.add(region.align_offset(HUGE_PAGE));
    madvise(aligned, bytes, 15 - huge as i32);
    aligned.cast()
}

unsafe fn discard(at: *const u8, len: usize) {
    madvise(at as *mut u8, len, 4);
}

unsafe fn space<T>(count: usize) -> &'static mut [T] {
    std::slice::from_raw_parts_mut(reserve(count, false), count)
}

unsafe fn take<T>(next: &mut *mut u8, count: usize) -> &'static mut [T] {
    let at = *next;
    *next = at.add((count * size_of::<T>()).next_multiple_of(4096));
    std::slice::from_raw_parts_mut(at.cast(), count)
}

unsafe fn map_file(path: &str) -> &'static [u8] {
    let file = File::open(path).unwrap();
    let len = file.metadata().unwrap().len() as usize;

    let span = len.div_ceil(4096) * 4096 + 4096;
    let base = mmap(null_mut(), span, 1, ANONYMOUS, -1, 0);
    assert!(base as isize != -1);
    assert!(len == 0 || mmap(base, len, 1, 0x12, file.as_raw_fd(), 0) == base);
    std::slice::from_raw_parts(base, len)
}

const KEY: i64 = 0x9E37_79B9_7F4A_7C15_u64 as i64;

static MASKS: [[u8; 16]; 2] = [[255; 16], [0; 16]];

#[inline(always)]
unsafe fn hash_pair(a: *const u8, len_a: usize, b: *const u8, len_b: usize) -> (u64, u64) {
    let load = |at: *const u8| _mm_loadu_si128(at.cast());
    let mask_of = |len: usize| load(MASKS.as_flattened().as_ptr().add(16 - len));
    let pair = _mm256_set_m128i(load(b), load(a));
    let mask = _mm256_set_m128i(mask_of(len_b), mask_of(len_a));
    let key = _mm256_and_si256(_mm256_or_si256(pair, _mm256_set1_epi8(0x20)), mask);

    let mixed = _mm256_aesenc_epi128(_mm256_aesenc_epi128(key, _mm256_set1_epi64x(KEY)), key);
    let folded = _mm256_xor_si256(mixed, _mm256_srli_si256::<8>(mixed));
    let [low, high] = [
        _mm256_extract_epi64::<0>(folded),
        _mm256_extract_epi64::<2>(folded),
    ];
    (low as u64 | 1, high as u64 | 1)
}

#[inline(always)]
unsafe fn hash_of(token: *const u8, len: usize) -> u64 {
    if len > 32 {
        return 0;
    }
    if len > 16 {
        long_hash(token, len)
    } else {
        hash_pair(token, len, token, len).0
    }
}

#[inline(always)]
unsafe fn action_in(run: &Run, hash: u64) -> u32 {
    let bucket = run.buckets.add(bucket_of(hash, run.bucket_shift));
    let wanted = _mm256_set1_epi32(hash as i32);
    let same = _mm256_cmpeq_epi32(_mm256_load_si256(bucket.cast()), wanted);
    let found = _mm256_movemask_ps(_mm256_castsi256_ps(same)) as u32 | 1 << BUCKET_SLOTS;
    let action = (*bucket).actions[found.trailing_zeros() as usize];
    if action == OVERFLOW {
        run.action_past(hash)
    } else {
        action
    }
}

#[inline(always)]
fn bucket_of(hash: u64, shift: u32) -> usize {
    (hash >> shift) as usize
}

#[cold]
unsafe fn long_hash(token: *const u8, len: usize) -> u64 {
    (hash_of(token, 16).rotate_left(29) ^ hash_of(token.add(16), len - 16)) | 1
}

impl Run {
    #[cold]
    unsafe fn action_past(&self, hash: u64) -> u32 {
        let mut index = bucket_of(hash, self.bucket_shift);
        loop {
            let bucket = &*self.buckets.add(index);
            match bucket.tags[..BUCKET_SLOTS]
                .iter()
                .position(|&tag| tag == hash as u32)
            {
                Some(slot) => return bucket.actions[slot],
                None if bucket.actions[BUCKET_SLOTS] == 0 => return 0,
                None => index = (index + 1) % self.bucket_count,
            }
        }
    }

    #[cold]
    unsafe fn claim_chunk(&self, shard: usize, claim: usize) -> (*mut u32, *mut u32) {
        let offset = self.claimed[shard].fetch_add(CHUNK, Relaxed);
        let at = self.rows.add(shard * (self.text.len() + 4096) + offset);
        *at = claim as u32 + 1;
        (at.add(1), at.add(CHUNK))
    }

    unsafe fn index_claim<const SAMPLE: bool>(&self, claim: usize, b: &mut Buffers) -> usize {
        let (start, mut doc) = aligned_line(self.text, self.docs, claim * CLAIM);
        let end = aligned_line(self.text, self.docs, claim * CLAIM + CLAIM).0;
        if start >= end {
            return 0;
        }
        let (first_doc, text) = (doc, self.text.as_ptr());

        let mut cursors = [(null_mut(), null_mut()); SHARDS];
        let (mut token_count, mut end_count, mut line_count) = (0, 0, 0);

        let (mut at, mut carry, mut first_token) = (start, 0u64, true);

        let byte = |value: u8| _mm256_set1_epi8(value as i8);
        let (floor_lower, ceil_lower) = (byte(b'a' - 1), byte(b'z' + 1));
        let (floor_digit, ceil_digit) = (byte(b'0' - 1), byte(b'9' + 1));
        let (fold, newline) = (byte(0x20), byte(b'\n'));
        let between = |x, floor, ceil| {
            _mm256_and_si256(_mm256_cmpgt_epi8(x, floor), _mm256_cmpgt_epi8(ceil, x))
        };

        while at < end {
            let batch_end = (at + BATCH).min(end);
            while at < batch_end {
                let classify = |at: usize| {
                    let chunk = _mm256_loadu_si256(text.add(at).cast());
                    let lower = _mm256_or_si256(chunk, fold);
                    let letters = between(lower, floor_lower, ceil_lower);
                    let digits = between(chunk, floor_digit, ceil_digit);
                    (
                        _mm256_movemask_epi8(_mm256_or_si256(letters, digits)) as u32 as u64,
                        _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, newline)) as u32 as u64,
                    )
                };
                let (inside_low, breaks_low) = classify(at);
                let (inside_high, breaks_high) = classify(at + 32);
                let mut inside = inside_low | inside_high << 32;
                let mut breaks = breaks_low | breaks_high << 32;
                if at + 64 > end {
                    let keep = (1u64 << (end - at)).wrapping_sub(1);
                    inside &= keep;
                    breaks &= keep;
                }
                let shifted = inside << 1 | carry;
                carry = inside >> 63;
                let mut opened = inside & !shifted;
                let mut closed = !inside & shifted;

                while breaks != 0 {
                    let before = opened & ((1u64 << breaks.trailing_zeros()) - 1);
                    b.lines[line_count] = (token_count + before.count_ones() as usize) as u32;
                    line_count += 1;
                    breaks &= breaks - 1;
                }

                let (found, ended) = (opened.count_ones() as usize, closed.count_ones() as usize);
                let first_in = |bits: u64| (at + bits.trailing_zeros() as usize) as u32;
                for extra in 0..12 {
                    *b.starts.get_unchecked_mut(token_count + extra) = first_in(opened);
                    *b.ends.get_unchecked_mut(end_count + extra) = first_in(closed);
                    opened &= opened.wrapping_sub(1);
                    closed &= closed.wrapping_sub(1);
                }
                for extra in 12..found {
                    b.starts[token_count + extra] = first_in(opened);
                    opened &= opened - 1;
                }
                for extra in 12..ended {
                    b.ends[end_count + extra] = first_in(closed);
                    closed &= closed - 1;
                }
                token_count += found;
                end_count += ended;
                at += 64;
            }

            let ready = b.lines[line_count.saturating_sub(1)] as usize;
            let mut token = 0;
            if first_token && ready > 0 {
                first_token = false;
                token = 1;
                b.hashes[0] = 0;
            }
            let (starts, ends, hashes) =
                (b.starts.as_ptr(), b.ends.as_ptr(), b.hashes.as_mut_ptr());
            let span = |i: usize| (*starts.add(i) as usize, *ends.add(i) as usize);
            let mut i = token;
            while i + 1 < ready {
                let (from_a, to_a) = span(i);
                let (from_b, to_b) = span(i + 1);
                let (len_a, len_b) = (to_a - from_a, to_b - from_b);
                let (a, b) = (text.add(from_a), text.add(from_b));
                (*hashes.add(i), *hashes.add(i + 1)) = if (len_a | len_b) > 16 {
                    (hash_of(a, len_a), hash_of(b, len_b))
                } else {
                    hash_pair(a, len_a, b, len_b)
                };
                i += 2;
            }
            if i < ready {
                let (from, to) = span(i);
                *hashes.add(i) = hash_of(text.add(from), to - from);
            }
            let (mut heavy_count, mut list_count, mut anchor_count) = (0, 0, 0);
            let batch_doc = doc;

            let buckets = self.buckets;
            let (heavy_hits, list_hits) = (b.heavy_hits.as_mut_ptr(), b.list_hits.as_mut_ptr());
            for line in 0..line_count {
                let to = b.lines[line] as usize;
                while token < to {
                    let ahead = bucket_of(*hashes.add(token + AHEAD), self.bucket_shift);
                    _mm_prefetch::<_MM_HINT_T0>(buckets.add(ahead).cast());
                    let action = action_in(self, *hashes.add(token));

                    *heavy_hits.add(heavy_count) = action;
                    heavy_count += (action >> 31) as usize;
                    *list_hits.add(list_count) = (action as u64) << 32 | doc as u64;

                    list_count += (action.wrapping_sub(1) < HEAVY - 1) as usize;

                    if action & ANCHOR != 0 {
                        *b.anchor_hits.get_unchecked_mut(anchor_count) =
                            (action as u64) << 40 | (doc as u64) << 20 | token as u64;
                        anchor_count += 1;
                    }
                    token += 1;
                }
                *b.heavy_ends.get_unchecked_mut(line) = heavy_count as u32;

                if line + 1 < line_count {
                    doc += 1;
                    b.hashes[to] = 0;
                    token = to + 1;
                }
            }
            for (i, &hit) in b.anchor_hits[..anchor_count].iter().enumerate() {
                let later = |ahead: usize| (b.anchor_hits[i + ahead] >> 40) as usize;
                _mm_prefetch::<_MM_HINT_T0>(
                    self.terms.wrapping_add(later(8).wrapping_sub(1)).cast(),
                );
                if let Some(term) = later(4).checked_sub(1) {
                    let first = (*self.terms.add(term)).phrases as usize;
                    _mm_prefetch::<_MM_HINT_T0>(
                        self.phrases.wrapping_add(first.wrapping_sub(1)).cast(),
                    );
                }
                let token = hit as usize & ((1 << 20) - 1);
                let mut next = (*self.terms.add((hit >> 40) as usize - 1)).phrases;
                while next != 0 {
                    let phrase = &*self.phrases.add(next as usize - 1);
                    next = phrase.next;
                    let (len, pivot) = (phrase.len as usize, phrase.pivot as usize);
                    if token >= pivot
                        && token - pivot + len <= ready
                        && b.hashes[token - pivot..token - pivot + len] == phrase.words[..len]
                    {
                        b.list_hits[list_count] =
                            (phrase.term as u64 + 1) << 32 | (hit >> 20 & ((1 << 20) - 1));
                        list_count += 1;
                    }
                }
            }
            if !SAMPLE {
                let (words, mut from) = (self.heavy().div_ceil(64), 0);
                for line in 0..line_count {
                    let to = b.heavy_ends[line] as usize;
                    let rel = (batch_doc as usize + line
                        - self.phase_base[claim / self.phase_claims] as usize)
                        % 64;
                    let row = &mut b.group[rel * words..(rel + 1) * words];
                    for &action in &b.heavy_hits[from..to] {
                        let id = (action & !HEAVY) as usize;
                        row[id / 64] |= 1 << (id % 64);
                    }
                    from = to;
                    if rel == 63 {
                        self.flush_group(b.group, claim, batch_doc as usize + line);
                    }
                }
            }
            for &hit in &b.list_hits[..list_count] {
                let term = ((hit >> 32) as u32 & !ANCHOR) - 1;
                if SAMPLE {
                    b.counts[term as usize] += 1;
                    continue;
                }
                let shard = term as usize & (SHARDS - 1);
                let cursor = cursors.get_unchecked_mut(shard);
                if cursor.0 == cursor.1 {
                    *cursor = self.claim_chunk(shard, claim);
                }
                *cursor.0 = ((term >> SHARDS.trailing_zeros()) + 1) << DOC_BITS | hit as u32;
                cursor.0 = cursor.0.add(1);
            }

            b.lines[0] = 0;
            line_count = line_count.min(1);
            b.starts.copy_within(ready..token_count, 0);
            token_count -= ready;
            b.ends.copy_within(ready..end_count, 0);
            end_count -= ready;
        }
        if !SAMPLE {
            self.flush_group(b.group, claim, doc as usize);
        }

        doc - first_doc + 1
    }

    fn heavy(&self) -> usize {
        self.setup[3].load(Relaxed).min(MAX_HEAVY)
    }

    unsafe fn flush_group(&self, group: &mut [u64], claim: usize, doc: usize) {
        let phase = claim / self.phase_claims;
        let rel = doc - self.phase_base[phase] as usize;
        let (heavy, out) = (self.heavy(), self.bitmap(0, phase).add(rel / 64));
        let words = heavy.div_ceil(64);
        for k in 0..words {
            let mut m: [u64; 64] =
                std::array::from_fn(|j| std::mem::take(&mut group[j * words + k]));
            for width in [32usize, 16, 8, 4, 2, 1] {
                let mask = u64::MAX / ((1 << width) + 1);
                for block in (0..64).step_by(2 * width) {
                    for r in block..block + width {
                        let t = ((m[r] >> width) ^ m[r + width]) & mask;
                        m[r + width] ^= t;
                        m[r] ^= t << width;
                    }
                }
            }
            for (t, &bits) in m.iter().enumerate().take(heavy - k * 64) {
                *out.add((k * 64 + t) * self.words_per_bitmap) |= bits;
            }
        }
    }

    fn barrier(&self) {
        let round = self.round.load(Acquire);
        if self.arrived.fetch_add(1, AcqRel) as usize + 1 == self.shares.len() {
            self.arrived.store(0, Relaxed);
            self.round.fetch_add(1, Release);

            unsafe { syscall(202, self.round.as_ptr(), 129, i32::MAX) };
            return;
        }

        for _ in 0..3000 {
            if self.round.load(Acquire) != round {
                return;
            }
            std::hint::spin_loop();
        }
        while self.round.load(Acquire) == round {
            unsafe { syscall(202, self.round.as_ptr(), 128, round, 0) };
        }
    }

    fn each_job(&self, counter: &AtomicUsize, count: usize, job: impl FnMut(usize)) {
        (0..)
            .map(|_| counter.fetch_add(1, Relaxed))
            .take_while(|&next| next < count)
            .for_each(job);
    }

    unsafe fn plan_phrases(&self, slice: usize) {
        let count = self.setup[5].load(Relaxed);
        for index in count * slice / JOBS..count * (slice + 1) / JOBS {
            let phrase = &mut *self.phrases.add(index);

            let terms = phrase.words[..phrase.len as usize].iter();
            let terms = terms.map(|&hash| action_in(self, hash) as usize - 1);
            let (pivot, term) = (terms.enumerate())
                .min_by_key(|&(_, term)| self.sample[term].load(Relaxed))
                .unwrap();
            let anchor = &mut (*self.terms.add(term)).phrases;
            phrase.pivot = pivot as u32;
            phrase.next = AtomicU32::from_ptr(anchor).swap(index as u32 + 1, Relaxed);
        }
    }

    unsafe fn plan_buckets(&self, slice: usize) {
        let docs = self.setup[4].load(Relaxed);
        for index in self.bucket_count * slice / JOBS..self.bucket_count * (slice + 1) / JOBS {
            let bucket = &mut *self.buckets.add(index);
            for action in &mut bucket.actions[..BUCKET_SLOTS] {
                if *action == 0 {
                    continue;
                }
                let id = *action as usize - 1;
                let term = &mut *self.terms.add(id);
                if term.phrases != 0 {
                    *action |= ANCHOR;
                } else if !term.in_bool.load(Relaxed) {
                    *action = 0;
                } else if self.sample[id].load(Relaxed) as usize * 32 >= docs {
                    let heavy = self.setup[3].fetch_add(1, Relaxed);
                    if heavy < MAX_HEAVY {
                        *action = HEAVY | heavy as u32;
                        (term.heavy, term.first) = (true, heavy as u32);
                    }
                }
            }
        }
    }

    unsafe fn build_shard(&self, shard: usize, phase: usize) {
        let base = self.phase_base[phase];
        let region = self.rows.add(shard * (self.text.len() + 4096));
        let claimed = self.claimed[shard].load(Relaxed);
        let mut chunks: Vec<(u32, u32)> = (0..claimed)
            .step_by(CHUNK)
            .map(|at| (*region.add(at), at as u32))
            .collect();
        chunks.sort_unstable();
        let term_count = self.term_count.load(Relaxed);
        let terms = term_count.div_ceil(SHARDS) + 1;

        for id in (shard..term_count).step_by(SHARDS) {
            (*self.terms.add(id)).count = 0;
        }
        let mut cursors = vec![0u32; terms];
        for_rows(region, &chunks, |low, _| {
            *cursors.get_unchecked_mut(low) += 1
        });

        let lists = cursors.iter().filter(|&&size| size > 0).count();
        let total = cursors.iter().sum::<u32>() as usize + lists * HEADER;
        let mut at = self.phase[phase][3].fetch_add(total, Relaxed) as u32;
        for cursor in cursors.iter_mut().filter(|cursor| **cursor > 0) {
            let size = *cursor;
            *cursor = at + HEADER as u32;
            *self.postings.add(*cursor as usize - 1) = u16::MAX;
            at += size + HEADER as u32;
        }
        let firsts = cursors.clone();
        for_rows(region, &chunks, |low, doc| {
            let cursor = cursors.get_unchecked_mut(low);
            let slot = self.postings.add(*cursor as usize);
            *slot = (doc - base) as u16;
            *cursor += (*slot.sub(1) != *slot) as u32;
        });
        for low in (0..terms).filter(|&low| firsts[low] > 0) {
            let info = &mut *self.terms.add(low * SHARDS + shard);
            info.first = firsts[low] - HEADER as u32;
            info.count = cursors[low] - firsts[low];
        }
        std::ptr::write_bytes(region, 0, claimed);
        self.claimed[shard].store(0, Relaxed);

        let heavy = self.heavy();
        let (from, to) = (heavy * shard / SHARDS, heavy * (shard + 1) / SHARDS);
        let first = self.bitmap(from as u32, phase + 1);
        std::ptr::write_bytes(first, 0, (to - from) * self.words_per_bitmap);
    }

    unsafe fn bitmap(&self, index: u32, phase: usize) -> *mut u64 {
        let set = (phase & 1) * MAX_HEAVY + index as usize;
        self.bitmaps.add(set * self.words_per_bitmap)
    }

    unsafe fn set_of(&self, term: u32, phase: usize) -> Set {
        let info = &*self.terms.add(term as usize);
        if info.heavy {
            let bits = self.bitmap(info.first, phase);
            return Set::Bits(std::slice::from_raw_parts(bits, self.words_per_bitmap));
        }
        let docs = self.postings.add(info.first as usize + HEADER);
        Set::List(std::slice::from_raw_parts(docs, info.count as usize))
    }

    unsafe fn answer_bool(&self, steps: &[(Op, u32)], phase: usize, scratch: &mut Scratch) -> Set {
        let mut current = self.set_of(steps[0].1, phase);
        for &(op, term) in &steps[1..] {
            if op != Op::Or && matches!(current, Set::List([])) {
                continue;
            }
            current = apply(op, current, self.set_of(term, phase), scratch);
        }
        current
    }

    unsafe fn answer(&self, (share, first, batch): Unit, phase: usize, scratch: &mut Scratch) {
        let base = self.phase_base[phase];
        let end = (first + QUERY_BATCH).min(share.queries.len());
        let tallies = share.tallies.as_ptr() as *mut Tally;
        let steps_of = |at: usize| &share.steps[share.queries[at].clone()];

        let prefetch = |at: usize, lists: bool| {
            for &(_, term) in (at < end).then(|| steps_of(at)).unwrap_or_default() {
                let info = &*self.terms.add(term as usize);
                if !lists {
                    _mm_prefetch::<_MM_HINT_T0>((info as *const Term).cast());
                } else if !info.heavy {
                    let list = self.postings.add(info.first as usize);

                    for line in 0..(info.count as usize).div_ceil(32).min(6) {
                        _mm_prefetch::<_MM_HINT_T0>(list.add(line * 32).cast());
                    }
                }
            }
        };
        prefetch(first, false);
        prefetch(first + 1, false);
        for index in first..end {
            prefetch(index + 2, false);
            prefetch(index + 1, true);
            let steps = steps_of(index);
            let tally = &mut *tallies.add(index);
            match self.answer_bool(steps, phase, scratch) {
                Set::List(list) => tally.add_list(base, list),
                Set::Bits(bits) => tally.add_bits(base, bits),
            }
        }
        if phase + 1 == self.phase_base.len() - 1 {
            let mut out = Vec::with_capacity(QUERY_BATCH * 64);
            for index in first..end {
                (*tallies.add(index)).write(&mut out);
            }
            *self.parts[batch].lock().unwrap() = out;
        }
    }

    unsafe fn parse_share(&self, k: usize) -> Parsed {
        let (size, shares) = (self.queries.len(), self.shares.len());
        let start = |k: usize| match k * size / shares {
            0 => 0,
            at => self.queries[at - 1..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(size, |n| at + n),
        };
        let (mut parsed, mut ids, mut op) = (Parsed::default(), 0..0, Op::And);
        for line in self.queries[start(k)..start(k + 1)].split(|&c| c == b'\n') {
            if line.is_empty() {
                continue;
            }
            let phrase = line[0] == b'P';

            let body = &line[if phrase { 8 } else { 5 }..line.len() - phrase as usize];
            let (first, mut words, mut count) = (parsed.steps.len(), [0; MAX_WORDS], 0);
            for word in body.split(|&c| c == b' ') {
                match word {
                    b"AND" => op = Op::And,
                    b"OR" => op = Op::Or,
                    b"NOT" => op = Op::Not,
                    _ => {
                        let hash = hash_of(word.as_ptr(), word.len());
                        let term = intern(self, &mut ids, hash);
                        if phrase {
                            words[count] = hash;
                            count += 1;
                        } else {
                            (*self.terms.add(term as usize))
                                .in_bool
                                .store(true, Relaxed);
                            parsed.steps.push((op, term));
                        }
                    }
                }
            }
            if phrase {
                let term = new_id(self, &mut ids);
                parsed.steps.push((Op::And, term));

                let slot = &mut *self.phrases.add(self.setup[5].fetch_add(1, Relaxed));
                (slot.term, slot.len, slot.words) = (term, count as u32, words);
            }
            parsed.queries.push(first..parsed.steps.len());
        }
        parsed.tallies = space(parsed.queries.len());
        parsed
    }

    unsafe fn work(&self, id: usize) {
        let tokens = (1 << 19) + 32;
        let mut next = self.workspace.add(id * WORKSPACE);
        let mut buffers = Buffers {
            starts: take(&mut next, tokens),
            ends: take(&mut next, tokens),
            lines: take(&mut next, BATCH / 2 + 64),
            hashes: take(&mut next, tokens + AHEAD),
            heavy_hits: take(&mut next, tokens),
            heavy_ends: take(&mut next, BATCH / 2 + 64),
            group: take(&mut next, 64 * MAX_HEAVY / 64),
            list_hits: take(&mut next, tokens),
            anchor_hits: take(&mut next, tokens),
            counts: take(&mut next, self.max_terms),
        };
        self.shares[id].set(self.parse_share(id)).ok();
        self.barrier();

        self.each_job(&self.setup[0], 12, |claim| {
            self.setup[4].fetch_add(self.index_claim::<true>(claim, &mut buffers), Relaxed);
        });
        let terms = self.term_count.load(Relaxed);
        for (total, &part) in self.sample.iter().zip(&buffers.counts[..terms]) {
            total.fetch_add(part, Relaxed);
        }

        discard(buffers.counts.as_ptr().cast(), self.max_terms * 4);
        self.barrier();
        self.each_job(&self.setup[1], JOBS, |slice| self.plan_phrases(slice));
        self.barrier();
        self.each_job(&self.setup[2], JOBS, |part| self.plan_buckets(part));
        self.barrier();

        let mut units = Vec::new();
        for share in self.shares.iter().map(|share| share.get().unwrap()) {
            for first in (0..share.queries.len()).step_by(QUERY_BATCH) {
                units.push((share, first, units.len()));
            }
        }
        let mut scratch = Scratch {
            bits: vec![0; self.words_per_bitmap],
            marks: vec![0; self.words_per_bitmap],
            ..Default::default()
        };
        let phases = self.phase_base.len() - 1;

        for stage in 0..=phases {
            if stage < phases {
                self.each_job(&self.phase[stage][0], self.phase_claims, |claim| {
                    self.index_claim::<false>(stage * self.phase_claims + claim, &mut buffers);
                });
            }
            if stage > 0 {
                self.each_job(&self.phase[stage - 1][2], units.len(), |batch| {
                    self.answer(units[batch], stage - 1, &mut scratch)
                });
            }
            if stage == phases {
                break;
            }
            self.barrier();
            if stage + 1 == phases {
                let page =
                    |n: usize| (self.text.len() * n / self.shares.len()).div_ceil(4096) * 4096;
                discard(self.text.as_ptr().add(page(id)), page(id + 1) - page(id));
            }

            assert!(self.phase_base[stage + 1] - self.phase_base[stage] < u16::MAX as u32);
            self.each_job(&self.phase[stage][1], SHARDS, |shard| {
                self.build_shard(shard, stage)
            });
            self.barrier();
        }
    }
}

#[inline(always)]
unsafe fn for_rows(region: *const u32, chunks: &[(u32, u32)], mut visit: impl FnMut(usize, u32)) {
    for &(_, at) in chunks {
        for i in 1..CHUNK {
            let row = *region.add(at as usize + i);
            if row == 0 {
                break;
            }
            visit((row >> DOC_BITS) as usize - 1, row & ((1 << DOC_BITS) - 1));
        }
    }
}

unsafe fn id_of(line: *const u8) -> usize {
    let digits = (0..).map(|i| *line.add(i)).take_while(|&c| c != b'\t');
    digits.fold(0, |id, digit| id * 10 + (digit - b'0') as usize)
}

unsafe fn aligned_line(text: &[u8], docs: usize, at: usize) -> (usize, usize) {
    let (base, len) = (text.as_ptr(), text.len());
    let past_line =
        |from: usize| memchr(base.add(from), b'\n' as i32, len - from) as usize - base as usize + 1;
    let mut line = at;
    if at > 0 && at < len && *base.add(at - 1) != b'\n' {
        line = past_line(at);
    }
    while line < len {
        let id = id_of(base.add(line));
        if id % 64 == 0 {
            return (line, id);
        }
        line = past_line(line);
    }
    (len, docs)
}

unsafe fn filter_by(a: &[u16], b: &[u16], keep: bool, out: &mut Vec<u16>, marks: &mut [u64]) {
    out.reserve(a.len().max(b.len()));
    let target = out.as_mut_ptr();
    let mut found = 0;

    if a.len() * 16 <= b.len() {
        for &doc in a {
            let present = b.binary_search(&doc).is_ok();
            *target.add(found) = doc;
            found += (present == keep) as usize;
        }
    } else {
        let (marked, tested) = if keep { (a, b) } else { (b, a) };
        for &doc in marked {
            *marks.get_unchecked_mut(doc as usize / 64) |= 1 << (doc % 64);
        }
        for &doc in tested {
            let present = *marks.get_unchecked(doc as usize / 64) >> (doc % 64) & 1;
            *target.add(found) = doc;
            found += (present == keep as u64) as usize;
        }
        for &doc in marked {
            *marks.get_unchecked_mut(doc as usize / 64) = 0;
        }
    }
    out.set_len(found);
}

fn union(a: &[u16], b: &[u16], out: &mut Vec<u16>) {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let (x, y) = (a[i], b[j]);
        out.push(x.min(y));
        i += (x <= y) as usize;
        j += (y <= x) as usize;
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
}

unsafe fn apply(op: Op, current: Set, term: Set, scratch: &mut Scratch) -> Set {
    let swap = match (op, current, term) {
        (Op::And, Set::Bits(_), Set::List(_)) => true,
        (Op::And, Set::List(a), Set::List(b)) => b.len() < a.len(),
        _ => false,
    };
    let (a, b) = [(current, term), (term, current)][swap as usize];
    scratch.lists.swap(0, 1);
    let out = &mut scratch.lists[0];
    out.clear();
    let bits = &mut scratch.bits;
    let (words, keep, marks) = (bits.len(), op == Op::And, &mut scratch.marks);
    match (op, a, b) {
        (Op::And | Op::Not, Set::List(a), Set::List(b)) => filter_by(a, b, keep, out, marks),
        (Op::And | Op::Not, Set::List(a), Set::Bits(b)) => {
            let held = |doc: u16| *b.get_unchecked(doc as usize / 64) >> (doc % 64) & 1 == 1;
            out.extend(a.iter().filter(|&&doc| held(doc) == keep));
        }

        (Op::Or, Set::List(a), Set::List(b)) if a.len() + b.len() < words => union(a, b, out),
        _ => {
            let mark = |bits: &mut [u64], docs: &[u16], clear: bool| {
                for &doc in docs {
                    let word = bits.get_unchecked_mut(doc as usize / 64);
                    let bit = 1 << (doc % 64);
                    *word = if clear { *word & !bit } else { *word | bit };
                }
            };
            match a {
                Set::Bits(source) if source.as_ptr() != bits.as_ptr() => {
                    bits.copy_from_slice(source)
                }
                Set::Bits(_) => {}
                Set::List(docs) => {
                    bits.fill(0);
                    mark(bits, docs, false);
                }
            }
            match b {
                Set::List(docs) => mark(bits, docs, op == Op::Not),
                Set::Bits(other) => {
                    for (word, &other) in bits.iter_mut().zip(other) {
                        *word = match op {
                            Op::And => *word & other,
                            Op::Or => *word | other,
                            Op::Not => *word & !other,
                        };
                    }
                }
            }
            return Set::Bits(std::slice::from_raw_parts(bits.as_ptr(), words));
        }
    }
    Set::List(std::slice::from_raw_parts(out.as_ptr(), out.len()))
}

fn put_number(out: &mut Vec<u8>, mut value: u64) {
    let from = out.len();
    while out.len() == from || value > 0 {
        out.push(b'0' + (value % 10) as u8);
        value /= 10;
    }
    out[from..].reverse();
    out.push(b' ');
}

unsafe fn intern(run: &Run, ids: &mut std::ops::Range<u32>, hash: u64) -> u32 {
    let mut index = bucket_of(hash, run.bucket_shift);
    loop {
        let bucket = run.buckets.add(index);
        for slot in 0..BUCKET_SLOTS {
            let tag = AtomicU32::from_ptr(std::ptr::addr_of_mut!((*bucket).tags[slot]));
            let action = AtomicU32::from_ptr(std::ptr::addr_of_mut!((*bucket).actions[slot]));
            let mut seen = tag.load(Acquire);
            if seen == 0 {
                seen = match tag.compare_exchange(0, hash as u32, AcqRel, Acquire) {
                    Ok(_) => {
                        let term = new_id(run, ids);
                        action.store(term + 1, Release);
                        return term;
                    }
                    Err(other) => other,
                };
            }
            if seen == hash as u32 {
                while action.load(Acquire) == 0 {
                    std::hint::spin_loop();
                }
                return action.load(Acquire) - 1;
            }
        }
        AtomicU32::from_ptr(std::ptr::addr_of_mut!((*bucket).actions[BUCKET_SLOTS]))
            .store(OVERFLOW, Relaxed);
        index = (index + 1) % run.bucket_count;
    }
}

unsafe fn new_id(run: &Run, ids: &mut std::ops::Range<u32>) -> u32 {
    ids.next().unwrap_or_else(|| {
        let first = run.term_count.fetch_add(ID_BLOCK, Relaxed) as u32;
        assert!(first as usize + ID_BLOCK <= run.max_terms);
        *ids = first + 1..first + ID_BLOCK as u32;
        first
    })
}

type Unit<'a> = (&'a Parsed, usize, usize);

#[derive(Default)]
struct Parsed {
    queries: Vec<std::ops::Range<usize>>,
    steps: Vec<(Op, u32)>,
    tallies: &'static mut [Tally],
}

fn spawn_tree<'scope, 'env>(scope: &'scope thread::Scope<'scope, 'env>, run: &'env Run, id: usize) {
    for child in (2 * id + 1..2 * id + 3).filter(|&c| c < run.shares.len()) {
        scope.spawn(move || spawn_tree(scope, run, child));
    }
    unsafe { run.work(id) };
}

fn main() {
    let threads = thread::available_parallelism()
        .map_or(12, |n| n.get())
        .clamp(1, 12);
    let data_dir = directory("--data", &["DATA_DIR", "SD_DATA"], "/data");
    let out_dir = directory("--out", &["OUT_DIR", "SD_OUT"], "/out");
    std::fs::create_dir_all(&out_dir).unwrap();
    unsafe {
        let [text, queries] =
            ["docs.txt", "queries.txt"].map(|name| map_file(&format!("{data_dir}/{name}")));
        let occurrences = queries
            .windows(2)
            .filter(|p| p[0] == b' ' && p[1] & 32 != 0)
            .count();
        let bucket_count = BUCKETS.max((occurrences / BUCKET_SLOTS + 1).next_power_of_two());
        let max_terms = MAX_TERMS.max(
            occurrences + queries.iter().filter(|&&c| c == b'\n').count() + threads * ID_BLOCK,
        );

        let last_line = text.rsplit(|&c| c == b'\n').nth(1);
        let docs = last_line.map_or(0, |line| id_of(line.as_ptr()) + 1);
        assert!(docs < 1 << DOC_BITS);

        let mut phase_claims = PHASE_CLAIMS;
        let phase_base: Vec<u32> = loop {
            let phases = text.len().div_ceil(phase_claims * CLAIM).max(1);
            let base: Vec<u32> = (0..=phases)
                .map(|p| aligned_line(text, docs, p * phase_claims * CLAIM).1 as u32)
                .collect();
            if base.windows(2).all(|w| w[1] - w[0] < u16::MAX as u32) {
                break base;
            }
            assert!(phase_claims > 1);
            phase_claims = phase_claims.div_ceil(2);
        };
        let phases = phase_base.len() - 1;
        let widest = phase_base.windows(2).fold(0, |m, w| m.max(w[1] - w[0]));

        let words_per_bitmap = (widest as usize / 64 + 2).next_multiple_of(4);

        let batches = queries.len() / (7 * QUERY_BATCH) + threads;
        let run = Run {
            text,
            docs,
            words_per_bitmap,
            buckets: reserve(bucket_count, true),
            bucket_count,
            bucket_shift: 64 - bucket_count.ilog2(),
            max_terms,
            phase_claims,
            terms: reserve(max_terms, true),
            term_count: AtomicUsize::new(0),
            queries,
            shares: (0..threads).map(|_| OnceLock::new()).collect(),
            phrases: reserve(max_terms, false),
            sample: space(max_terms),
            setup: [0; 6].map(AtomicUsize::new),
            phase: (0..phases).map(|_| Default::default()).collect(),
            phase_base,
            arrived: Default::default(),
            round: Default::default(),
            bitmaps: reserve(2 * MAX_HEAVY * words_per_bitmap, true),

            rows: reserve(SHARDS * (text.len() + 4096), false),
            claimed: (0..SHARDS).map(|_| AtomicUsize::new(0)).collect(),
            postings: reserve(text.len() + HUGE_PAGE, true),
            parts: (0..batches).map(|_| Mutex::default()).collect(),
            workspace: reserve(threads * WORKSPACE, false),
        };

        run.buckets.cast::<u8>().write_volatile(0);
        prctl(41, 1, 0, 0, 0);

        thread::scope(|s| spawn_tree(s, &run, 0));

        let mut out = File::create(format!("{out_dir}/results.txt")).unwrap();
        for part in &run.parts {
            out.write_all(&part.lock().unwrap()).unwrap();
        }
    }
}

fn directory(flag: &str, variables: &[&str], default: &str) -> String {
    let args: Vec<String> = std::env::args().collect();
    variables
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|s| !s.is_empty()))
        .or_else(|| {
            args.windows(2)
                .rev()
                .find(|a| a[0] == flag)
                .map(|a| a[1].clone())
        })
        .unwrap_or_else(|| default.to_owned())
}
