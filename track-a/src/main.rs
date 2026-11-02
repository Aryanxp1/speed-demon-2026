use std::arch::x86_64::*;
use std::ptr::{copy_nonoverlapping, read_unaligned as read, write_unaligned};
use std::sync::{atomic::Ordering::*, atomic::*, Arc, Mutex, OnceLock};
use std::{fs::File, os::fd::AsRawFd, os::unix::fs::FileExt, thread};

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;

    fn madvise(addr: *mut u8, len: usize, advice: i32) -> i32;
    fn prctl(option: i32, ...) -> i32;
}

const CLAIM: usize = 1 << 20;
const BLOCK: usize = 1 << 16;
const MIN_LINE: usize = 40;
const HOURS: usize = 72;
const CHUNK: usize = 256;
const SLOTS: usize = 1 << 13;
const ID_BITS: u32 = 11;
const ID_MASK: u64 = (1 << ID_BITS) - 1;
const SPARE_AT: usize = 1 << 18;
const ZEROS: u64 = 0x3030_3030_3030_3030;

const LATENCY_BITS: u32 = 20;
const ESCAPE: u32 = (1 << LATENCY_BITS) - 1;
const DIGIT_BITS: u32 = 10;
const DIGITS: usize = 1 << DIGIT_BITS;

fn spaces_in(word: u64) -> u64 {
    let z = word ^ 0x2020_2020_2020_2020;
    z.wrapping_sub(0x0101_0101_0101_0101) & !z & 0x8080_8080_8080_8080
}

#[inline(always)]
unsafe fn name_hash(name: *const u8, len: usize) -> u64 {
    const WINDOW: [[u8; 32]; 3] = [[0; 32], [255; 32], [0; 32]];
    let load = |at: *const u8| _mm256_loadu_si256(at.cast());
    let (window, n) = (WINDOW.as_ptr().cast::<u8>(), len.min(32));
    let head = _mm256_and_si256(load(name), load(window.add(64 - n)));
    let tail = _mm256_and_si256(load(name.add(len).sub(32)), load(window.add(n)));
    let mut mixed = _mm256_aesenc_epi128(head, tail);
    if len > 64 {
        mixed = _mm256_aesenc_epi128(mixed, load(name.add(32)));
        mixed = _mm256_aesenc_epi128(mixed, load(name.add(len - 64)));
    }

    let key: __m256i = std::mem::transmute([KEY_LOW, KEY_HIGH]);
    let mixed = _mm256_aesenc_epi128(_mm256_aesenc_epi128(mixed, key), mixed);
    let [a, b]: [u64; 2] = std::mem::transmute(_mm_xor_si128(
        _mm256_castsi256_si128(mixed),
        _mm256_extracti128_si256(mixed, 1),
    ));
    (a ^ b) & !(127u64 << ID_BITS) | (len as u64) << ID_BITS
}

const SEALED: usize = 1 << (usize::BITS - 1);
const KEY_LOW: u128 = 0x61c8864680b583ea94f82b3c6ef372fe;
const KEY_HIGH: u128 = 0x9e3779b97f4a7c15f372fe94f82b3c6e;

struct Run {
    input: *mut u8,
    len: usize,
    next_claim: AtomicUsize,

    slots: [AtomicU64; SLOTS],
    names: Mutex<Vec<&'static [u8]>>,
    active: [AtomicUsize; HOURS],

    order: Mutex<Arc<Vec<(&'static [u8], usize)>>>,
    rows: *mut u32,
    stride: usize,
    claimed: [AtomicUsize; HOURS],

    parsed: Vec<AtomicU32>,

    early: bool,
    dirty: AtomicBool,
    next_hour: AtomicUsize,
    texts: [OnceLock<&'static [u8]>; HOURS],
    output: File,
    placed: Mutex<(usize, u64)>,
}
unsafe impl Sync for Run {}

impl Run {
    #[inline(always)]
    unsafe fn lookup(&self, name: *const u8, len: usize) -> usize {
        let hash = name_hash(name, len);
        let mut i = (hash >> (64 - SLOTS.trailing_zeros())) as usize;
        loop {
            let slot = self.slots[i].load(Acquire);
            if slot >> ID_BITS == hash >> ID_BITS {
                return (slot & ID_MASK) as usize - 1;
            } else if slot == 0 {
                let mut names = self.names.lock().unwrap();
                if self.slots[i].load(Acquire) == 0 {
                    let id = names.len();
                    assert!(id < ID_MASK as usize);
                    names.push(std::slice::from_raw_parts(name, len).to_vec().leak());
                    self.slots[i].store(hash >> ID_BITS << ID_BITS | (id + 1) as u64, Release);
                    return id;
                }
                continue;
            }
            i = (i + 1) & (SLOTS - 1);
        }
    }

    unsafe fn parse(&self) {
        let input = self.input;

        let mut cursors = [(std::ptr::null_mut::<u32>(), std::ptr::null_mut()); HOURS];
        let (mut hour, mut cursor) = (0, (std::ptr::null_mut(), std::ptr::null_mut()));
        let mut ends = [0u32; BLOCK / MIN_LINE + 1];
        loop {
            self.advance();
            let claim = self.next_claim.fetch_add(CLAIM, Relaxed);
            if claim >= self.len {
                return;
            }

            let mut line = input.add(claim);
            while claim > 0 && *line.sub(1) != b'\n' {
                line = line.sub(1);
            }

            let (mut key, mut low, mut high) = (u64::MAX, HOURS, 0);
            let mut owned = [false; HOURS];
            'blocks: for start in (claim..(claim + CLAIM).min(self.len)).step_by(BLOCK) {
                let newline = _mm256_set1_epi8(b'\n' as i8);
                let (mut count, mut at) = (0, start);
                while at < (start + BLOCK).min(self.len) {
                    let hits = _mm256_cmpeq_epi8(_mm256_load_si256(input.add(at).cast()), newline);
                    let mask = _mm256_movemask_epi8(hits) as u32;
                    *ends.get_unchecked_mut(count) = (at + mask.trailing_zeros() as usize) as u32;
                    count += (mask != 0) as usize;
                    at += 32;
                }
                for &end in &ends[..count] {
                    let end = input.add(end as usize);
                    _mm_prefetch(end.wrapping_add(BLOCK).cast(), _MM_HINT_T0);

                    let clock: u64 = read(line.add(8).cast());
                    if clock << 24 != key {
                        cursors[hour] = cursor;
                        key = clock << 24;
                        let day = (clock >> 8 & 15) + 1 - (clock >> 1 & 1);
                        hour =
                            (day * 24 + (clock >> 24 & 15) * 10 + (clock >> 32 & 15) - 18) as usize;
                        cursor = cursors[hour];
                        (low, high) = (low.min(hour), high.max(hour));
                        if !owned[hour] {
                            if self.active[hour]
                                .try_update(AcqRel, Acquire, |n| (n < SEALED).then_some(n + 1))
                                .is_err()
                            {
                                self.dirty.store(true, Relaxed);
                                break 'blocks;
                            }
                            owned[hour] = true;
                        }
                    }

                    let method = spaces_in(read(line.add(28).cast())).trailing_zeros() / 8;
                    let name = line.add(29 + method as usize);

                    let last_eight = read(end.sub(8).cast());
                    let spaces = spaces_in(last_eight);
                    let (latency, digits) = if spaces != 0 {
                        let digits = spaces.leading_zeros() as usize / 8;
                        let v = (last_eight ^ ZEROS) >> (64 - 8 * digits) << (64 - 8 * digits);
                        let v = (v & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561) >> 8;
                        let v = (v & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601) >> 16;
                        let v = (v & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001) >> 32;
                        (v as u32, digits)
                    } else {
                        let digits = (1..).find(|&n| *end.sub(n + 1) == b' ').unwrap();
                        let text = std::slice::from_raw_parts(end.sub(digits), digits);
                        (std::str::from_utf8_unchecked(text).parse().unwrap(), digits)
                    };

                    let status = end.sub(digits + 4);
                    let len = status.sub(1).offset_from(name) as usize;
                    let id = self.lookup(name, len) as u32;
                    let is_error = (*status == b'5') as u32;

                    if cursor.0 >= cursor.1 {
                        let at = self.claimed[hour].fetch_add(CHUNK, Relaxed) + hour * self.stride;
                        cursor = (self.rows.add(at), self.rows.add(at + CHUNK - 1));
                    }
                    *cursor.0 = ((id + 1) << 1 | is_error) << LATENCY_BITS | latency.min(ESCAPE);
                    cursor.0 = cursor.0.add(1);
                    if latency >= ESCAPE {
                        *cursor.0 = latency;
                        cursor.0 = cursor.0.add(1);
                    }
                    line = end.add(1);
                }
            }

            for (state, &held) in self.active.iter().zip(&owned) {
                if held {
                    state.fetch_sub(1, Release);
                }
            }
            madvise(input.add(claim), CLAIM.min(self.len - claim), 4);
            self.parsed[claim / CLAIM].store((1 << 16 | low << 8 | high) as u32, Release);
        }
    }

    fn advance(&self) {
        let (mut top, mut sealed) = (0usize, HOURS);
        for claim in &self.parsed {
            let info = claim.load(Acquire) as usize;
            if info == 0 {
                sealed = top.saturating_sub(1);
                break;
            }
            self.dirty.fetch_or((info >> 8 & 255) + 1 < top, Relaxed);
            top = top.max((info & 255) * self.early as usize);
        }
        let hour = self.next_hour.load(Relaxed);
        if hour >= sealed {
            return;
        }
        let Ok(_) = self.active[hour].compare_exchange(0, SEALED, AcqRel, Acquire) else {
            return;
        };
        self.next_hour.store(hour + 1, Relaxed);

        let order = {
            let names = self.names.lock().unwrap();
            let mut order = self.order.lock().unwrap();
            if order.len() != names.len() {
                let mut sorted: Vec<_> = names.iter().copied().zip(0..).collect();
                sorted.sort_unstable();
                *order = Arc::new(sorted);
            }
            order.clone()
        };
        let part = unsafe { self.summarise(hour, &order) };
        self.texts[hour].set(part).unwrap();

        self.write_ready();
    }

    unsafe fn summarise(&self, hour: usize, order: &[(&[u8], usize)]) -> &'static [u8] {
        let rows = self.rows.add(hour * self.stride);
        let claimed = self.claimed[hour].load(Relaxed);

        let spare = rows.add(claimed.max(SPARE_AT).next_multiple_of(1024));

        let (mut starts, mut errors) = (vec![0u32; 1 << ID_BITS], vec![0u32; 1 << ID_BITS]);
        let mut long = Vec::new();

        let (mut sizes, mut count) = ([[0u32; DIGITS]; 2], 0);
        for chunk in (0..claimed).step_by(CHUNK) {
            let mut at = chunk;
            while at < chunk + CHUNK && *rows.add(at) != 0 {
                let row = *rows.add(at);
                at += 1;
                let group = (row >> (LATENCY_BITS + 1)) as usize;
                *starts.get_unchecked_mut(group) += 1;
                *errors.get_unchecked_mut(group) += row >> LATENCY_BITS & 1;
                if row & ESCAPE == ESCAPE {
                    long.push((*rows.add(at), row));
                    at += 1;
                } else {
                    *rows.add(count) = row;
                    count += 1;
                    sizes[0][row as usize & (DIGITS - 1)] += 1;
                    sizes[1][(row >> DIGIT_BITS) as usize & (DIGITS - 1)] += 1;
                }
            }
        }

        let [low, high] = &mut sizes;
        for (from, to, shift, hist) in [(rows, spare, 0, low), (spare, rows, DIGIT_BITS, high)] {
            let mut total = 0;
            for size in hist.iter_mut() {
                (total, *size) = (total + *size, total);
            }
            for i in 0..count {
                let row = *from.add(i);
                let slot = hist.get_unchecked_mut((row >> shift) as usize & (DIGITS - 1));
                *to.add(*slot as usize) = row;
                *slot += 1;
            }
        }
        long.sort_unstable();

        let mut total = 0;
        for start in starts.iter_mut() {
            (total, *start) = (total + *start, total);
        }
        let mut deal = |row: u32, latency: u32| {
            let start = starts.get_unchecked_mut((row >> (LATENCY_BITS + 1)) as usize);
            *spare.add(*start as usize) = latency;

            *start += 1;
        };
        for i in 0..count {
            deal(*rows.add(i), *rows.add(i) & ESCAPE);
        }
        long.iter().for_each(|&(latency, row)| deal(row, latency));

        let date = ["10-30", "10-31", "11-01", "11-02"][(hour + 18) / 24];
        let label = format!("2026-{date}T{:02}:00:00Z ", (hour + 18) % 24);
        let (text, mut w) = (rows.cast::<u8>(), rows.cast::<u8>());
        for &(name, id) in order {
            let span = starts[id] as usize..starts[id + 1] as usize;
            if span.is_empty() {
                continue;
            }
            let (n, bad) = (span.len() as u32, errors[id + 1]);
            let sorted = std::slice::from_raw_parts(spare.add(span.start), span.len());
            let at_rank = |p: usize| sorted[(p * sorted.len() + 99) / 100 - 1];
            copy_nonoverlapping(label.as_ptr(), w, 21);
            copy_nonoverlapping(name.as_ptr(), w.add(21), name.len());
            w = w.add(21 + name.len());
            for number in [n, at_rank(50), at_rank(95), at_rank(99), bad] {
                w = put_number(w, number);
            }
            w = put_number(w, (bad as u64 * 1_000_000 / n as u64) as u32);
            *w = b'\n';
            w = w.add(1);
        }

        madvise(spare.cast(), (count + long.len()) * 4, 4);
        std::slice::from_raw_parts(text, w.offset_from(text) as usize)
    }

    fn write_ready(&self) {
        let mut ready = Vec::new();
        {
            let mut placed = self.placed.lock().unwrap();
            let (next, offset) = &mut *placed;
            while let Some(text) = self.texts.get(*next).and_then(|text| text.get()) {
                ready.push((*next, *offset, *text));
                (*next, *offset) = (*next + 1, *offset + text.len() as u64);
            }
        }
        for (hour, offset, text) in ready {
            self.output.write_all_at(text, offset).unwrap();

            let rows = unsafe { self.rows.add(hour * self.stride) };
            unsafe { madvise(rows.cast(), self.stride * 4, 4) };
        }
    }

    fn work(&self) {
        unsafe { self.parse() };

        while self.next_hour.load(Relaxed) < HOURS {
            self.advance();
            thread::yield_now();
        }
    }
}

fn eight_digits(value: u32) -> u64 {
    let merged = (value / 10000) as u64 | ((value % 10000) as u64) << 32;
    let hundreds = (merged.wrapping_mul(10486) >> 20) & 0x0000_007F_0000_007F;
    let pairs = hundreds | (merged - 100 * hundreds) << 16;
    let tens = (pairs.wrapping_mul(103) >> 10) & 0x000F_000F_000F_000F;
    tens | (pairs - 10 * tens) << 8
}

unsafe fn put_number(w: *mut u8, value: u32) -> *mut u8 {
    if value >= 100_000_000 {
        return put_large(w, value);
    }
    *w = b' ';
    let digits = eight_digits(value);
    let skip = (digits | 1 << 63).trailing_zeros() as usize / 8;
    write_unaligned(w.add(1).cast(), (digits + ZEROS) >> (8 * skip));
    w.add(9 - skip)
}

#[cold]
unsafe fn put_large(w: *mut u8, value: u32) -> *mut u8 {
    *w = b' ';
    let digits = eight_digits(value / 100_000_000) as u128
        | (eight_digits(value % 100_000_000) as u128) << 64;

    let skip = (digits | 1 << 127).trailing_zeros() as usize / 8;
    let zeros = ZEROS as u128 * ((1 << 64) + 1);
    write_unaligned(w.add(1).cast(), (digits + zeros) >> (8 * skip));
    w.add(17 - skip)
}

fn main() {
    let _ = solve(true) && solve(false);
}

fn solve(early: bool) -> bool {
    let file = File::open("/data/requests.log").unwrap();
    let len = file.metadata().unwrap().len() as usize;
    let threads = thread::available_parallelism().map_or(12, |n| n.get());

    let (input, rows, stride) = unsafe {
        prctl(41, 1usize, 0usize, 0usize, 0usize);
        let map = |at, len, prot, flags, fd| mmap(at, len, prot, flags, fd, 0);
        let null = std::ptr::null_mut();
        let input = map(null, len.div_ceil(4096) * 4096 + 4096, 1, 0x4022, -1);

        assert!(input as isize != -1);
        assert!(len == 0 || map(input, len, 1, 0x12, file.as_raw_fd()) == input);

        let stride =
            (SPARE_AT + (len / MIN_LINE + 1) * 4 + threads * CHUNK * 2).next_multiple_of(1 << 19);

        let rows = map(null, stride * HOURS * 4 + (2 << 20), 3, 0x4022, -1);
        assert!(rows as isize != -1);
        let rows = rows.map_addr(|a| a.next_multiple_of(2 << 20));
        (input, rows.cast(), stride)
    };

    let run = Run {
        input,
        len,
        next_claim: AtomicUsize::new(0),
        slots: [const { AtomicU64::new(0) }; SLOTS],
        names: Mutex::new(Vec::new()),
        active: [const { AtomicUsize::new(0) }; HOURS],
        rows,
        stride,
        claimed: [const { AtomicUsize::new(0) }; HOURS],
        order: Mutex::default(),
        parsed: (0..len.div_ceil(CLAIM)).map(|_| 0.into()).collect(),
        early,
        dirty: AtomicBool::new(false),
        next_hour: AtomicUsize::new(0),
        texts: [const { OnceLock::new() }; HOURS],
        output: File::create("/out/summary.txt").unwrap(),
        placed: Mutex::new((0, 0)),
    };
    thread::scope(|s| {
        for _ in 1..threads {
            s.spawn(|| run.work());
        }
        run.work();
    });
    run.dirty.load(Relaxed)
}
