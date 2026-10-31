use std::arch::x86_64::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Barrier, Mutex};
use std::thread;

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
    fn prctl(option: i32, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32;
}

const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FAILED: *mut u8 = !0 as *mut u8;

unsafe fn zeroed<T>(n: usize) -> *mut T {
    let bytes = n * std::mem::size_of::<T>();
    let p = mmap(
        std::ptr::null_mut(),
        bytes,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if p == MAP_FAILED {
        std::process::exit(1);
    }
    p as *mut T
}

unsafe fn map_file(file: &File) -> (*const u8, usize) {
    let size = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
    if size == 0 {
        return (std::ptr::null(), 0);
    }
    let fd = file.as_raw_fd();
    // Map with spare 4096-byte padding so 64-byte vector loads past end are safe
    let p = mmap(
        std::ptr::null_mut(),
        size + 4096,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if p == MAP_FAILED {
        std::process::exit(1);
    }
    // Overlay file contents
    if mmap(
        p,
        size,
        PROT_READ,
        MAP_PRIVATE | 0x10, /* MAP_FIXED */
        fd,
        0,
    ) == MAP_FAILED
    {
        std::process::exit(1);
    }
    (p as *const u8, size)
}

#[inline(always)]
unsafe fn load8(p: *const u8) -> u64 {
    (p as *const u64).read_unaligned()
}

#[inline(always)]
unsafe fn match32(p: *const u8, c: u8) -> u32 {
    let v = _mm256_loadu_si256(p as *const __m256i);
    let cmp = _mm256_cmpeq_epi8(v, _mm256_set1_epi8(c as i8));
    _mm256_movemask_epi8(cmp) as u32
}

#[inline(always)]
unsafe fn match64(p: *const u8, c: u8) -> u64 {
    (match32(p, c) as u64) | ((match32(p.add(32), c) as u64) << 32)
}

#[inline(always)]
unsafe fn digits(s: *const u8, e: *const u8) -> u64 {
    let len = e as usize - s as usize;
    let shift = 8 * (8 - len);
    let mask = !0u64 << shift;
    let mut v = load8(e.sub(8)) & 0x0F0F0F0F0F0F0F0F & mask;
    v = (v.wrapping_mul(10) + (v >> 8)) & 0x00FF00FF00FF00FF;
    v = (v.wrapping_mul(100) + (v >> 16)) & 0x0000FFFF0000FFFF;
    (v.wrapping_mul(10000) + (v >> 32)) & 0xFFFFFFFF
}

struct Separator {
    at: *const u8,
    quotes: u64,
}

struct Structure {
    block: *const u8,
    end: *const u8,
    bits: u64,
    carry: u64,
    q: u64,
    before: u64,
}

impl Structure {
    #[inline(always)]
    fn new(p: *const u8, end: *const u8) -> Self {
        Self {
            block: p.wrapping_sub(64),
            end,
            bits: 0,
            carry: 0,
            q: 0,
            before: 0,
        }
    }

    #[target_feature(enable = "avx2,pclmulqdq")]
    unsafe fn next(&mut self) -> Separator {
        while self.bits == 0 {
            self.block = self.block.wrapping_add(64);
            if self.block >= self.end {
                return Separator {
                    at: self.end,
                    quotes: self.before + self.q.count_ones() as u64,
                };
            }
            self.before += self.q.count_ones() as u64;
            self.q = match64(self.block, b'"');
            let q_128 = _mm_cvtsi64_si128(self.q as i64);
            let ones = _mm_set1_epi8(-1);
            let x = _mm_clmulepi64_si128(q_128, ones, 0);
            let inside = (_mm_cvtsi128_si64(x) as u64) ^ self.carry;
            self.carry = (inside as i64 >> 63) as u64;
            self.bits = (match64(self.block, b',') | match64(self.block, b'\n')) & !inside;
        }
        let i = self.bits.trailing_zeros();
        self.bits &= self.bits - 1;
        let q_mask = (1u64 << i).wrapping_sub(1);
        Separator {
            at: self.block.add(i as usize),
            quotes: self.before + (self.q & q_mask).count_ones() as u64,
        }
    }
}

const SLOTS: usize = 1 << 17;
const IDS: usize = 1 << 17;
const HEAD: usize = 24;
const BUSY: u16 = 0xFFFF;

#[derive(Copy, Clone)]
struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

impl<T> SendPtr<T> {
    #[inline(always)]
    fn as_ptr(&self) -> *const T {
        self.0 as *const T
    }
    #[inline(always)]
    fn as_mut_ptr(&self) -> *mut T {
        self.0
    }
}

#[repr(C, align(32))]
#[derive(Copy, Clone)]
struct Slot {
    id: u32,
    len: u32,
    head: [u8; HEAD],
}

#[derive(Copy, Clone)]
struct Probe {
    slot: usize,
    tag: u16,
}

struct NameTable {
    tags: *mut AtomicU16,
    slots: *mut Slot,
    overflow: *mut *const u8,
    spill: Mutex<HashMap<Vec<u8>, u32>>,
    next_id: AtomicU32,
}

unsafe impl Send for NameTable {}
unsafe impl Sync for NameTable {}

impl NameTable {
    fn new() -> Self {
        unsafe {
            Self {
                tags: zeroed::<AtomicU16>(SLOTS),
                slots: zeroed::<Slot>(SLOTS),
                overflow: zeroed::<*const u8>(SLOTS),
                spill: Mutex::new(HashMap::new()),
                next_id: AtomicU32::new(0),
            }
        }
    }

    #[inline(always)]
    unsafe fn probe(&self, s: *const u8, len: u32) -> Probe {
        let mut h = len as u64;
        let mut i = 0u32;
        while i < len {
            let mut w = load8(s.add(i as usize));
            let rem = len - i;
            if rem < 8 {
                w &= (1u64 << (8 * rem)) - 1;
            }
            h = (h ^ w).wrapping_mul(0x9E3779B97F4A7C15);
            i += 8;
        }
        let mut p = Probe {
            slot: ((h >> 20) as usize) % SLOTS,
            tag: (1 + ((h >> 48) as u16) % (BUSY - 1)),
        };
        for _ in 0..SLOTS {
            let t = (*self.tags.add(p.slot)).load(Ordering::Relaxed);
            if t == 0 || t == p.tag || t == BUSY {
                break;
            }
            p.slot = (p.slot + 1) % SLOTS;
        }
        _mm_prefetch(self.slots.add(p.slot) as *const i8, _MM_HINT_T0);
        p
    }

    #[inline(always)]
    unsafe fn id(&self, s: *const u8, len: u32, mut p: Probe) -> u32 {
        for _ in 0..SLOTS {
            let tag_ref = &*self.tags.add(p.slot);
            let mut t = tag_ref.load(Ordering::Acquire);
            if t == 0 {
                match tag_ref.compare_exchange(0, BUSY, Ordering::Acquire, Ordering::Acquire) {
                    Ok(_) => {
                        let slot = &mut *self.slots.add(p.slot);
                        slot.id = self.next_id.fetch_add(1, Ordering::Relaxed);
                        slot.len = len;
                        std::ptr::copy_nonoverlapping(
                            s,
                            slot.head.as_mut_ptr(),
                            (len as usize).min(HEAD),
                        );
                        if len as usize > HEAD {
                            // Input mapping stays alive until all name lookups finish.
                            *self.overflow.add(p.slot) = s.add(HEAD);
                        }
                        tag_ref.store(p.tag, Ordering::Release);
                        return slot.id;
                    }
                    // A competing writer owns this slot. Inspect its published
                    // name before advancing, otherwise the same name gets two IDs.
                    Err(observed) => t = observed,
                }
            }
            while t == BUSY {
                t = tag_ref.load(Ordering::Acquire);
            }
            let slot = &*self.slots.add(p.slot);
            if t == p.tag && slot.len == len && self.same_head(slot, s, len) {
                if (len as usize) <= HEAD
                    || libc_memcmp(*self.overflow.add(p.slot), s.add(HEAD), len as usize - HEAD)
                {
                    return slot.id;
                }
            }
            p.slot = (p.slot + 1) % SLOTS;
        }
        // Only used once every fast slot is occupied. Exact bytes and the
        // mutex ensure bounded probing and deterministic deduplication.
        let mut spill = self.spill.lock().unwrap();
        *spill
            .entry(std::slice::from_raw_parts(s, len as usize).to_vec())
            .or_insert_with(|| self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    #[inline(always)]
    unsafe fn same_head(&self, slot: &Slot, s: *const u8, len: u32) -> bool {
        let a = _mm256_load_si256(slot as *const Slot as *const __m256i);
        let b = _mm256_loadu_si256(s.sub(8) as *const __m256i);
        let equal = _mm256_movemask_epi8(_mm256_cmpeq_epi8(a, b)) as u32;
        let head_len = (len as usize).min(HEAD);
        let want = (((1u64 << head_len) - 1) << 8) as u32;
        (equal & want) == want
    }
}

#[inline(always)]
unsafe fn libc_memcmp(a: *const u8, b: *const u8, n: usize) -> bool {
    let mut i = 0;
    while i < n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
        i += 1;
    }
    true
}

const WORDS: usize = IDS / 64;

struct NamesSeen {
    bits: *mut AtomicU64,
    spill: Mutex<HashSet<(u32, u32)>>,
}

unsafe impl Send for NamesSeen {}
unsafe impl Sync for NamesSeen {}

impl NamesSeen {
    fn new() -> Self {
        unsafe {
            Self {
                bits: zeroed::<AtomicU64>(256 * WORDS),
                spill: Mutex::new(HashSet::new()),
            }
        }
    }

    #[inline(always)]
    unsafe fn word(&self, c: u32, name: u32) -> &AtomicU64 {
        &*self.bits.add(c as usize * WORDS + (name as usize / 64))
    }

    #[inline(always)]
    unsafe fn prefetch(&self, c: u32, name: u32) {
        if name as usize >= IDS {
            return;
        }
        _mm_prefetch(
            self.bits.add(c as usize * WORDS + (name as usize / 64)) as *const i8,
            _MM_HINT_T0,
        );
    }

    #[inline(always)]
    unsafe fn mark(&self, c: u32, name: u32) {
        if name as usize >= IDS {
            self.spill.lock().unwrap().insert((c, name));
            return;
        }
        let bit = 1u64 << (name % 64);
        let w = self.word(c, name);
        if (w.load(Ordering::Relaxed) & bit) == 0 {
            w.fetch_or(bit, Ordering::Relaxed);
        }
    }

    #[inline(always)]
    unsafe fn count(&self, c: u32) -> u64 {
        let mut n = 0u64;
        for w in 0..WORDS {
            n += (*self.bits.add(c as usize * WORDS + w))
                .load(Ordering::Relaxed)
                .count_ones() as u64;
        }
        n + self
            .spill
            .lock()
            .unwrap()
            .iter()
            .filter(|&&(country, _)| country == c)
            .count() as u64
    }
}

#[derive(Copy, Clone)]
struct Totals {
    count: u64,
    id_sum: u64,
    name_bytes: u64,
    note_bytes: u64,
    distinct: u64,
    amount_sum: i64,
    amount_min: i64,
    amount_max: i64,
}

impl Default for Totals {
    fn default() -> Self {
        Self {
            count: 0,
            id_sum: 0,
            name_bytes: 0,
            note_bytes: 0,
            distinct: 0,
            amount_sum: 0,
            amount_min: i64::MAX,
            amount_max: i64::MIN,
        }
    }
}

// regions.csv is sorted by ID. Ignore the final LF before locating its last row.
fn region_count(bytes: &[u8]) -> usize {
    let last = bytes
        .rsplit(|&b| b == b'\n')
        .find(|line| !line.is_empty())
        .unwrap_or(&[]);
    last.iter()
        .take_while(|b| b.is_ascii_digit())
        .fold(0usize, |v, &b| v * 10 + (b - b'0') as usize)
        + 1
}

#[inline(always)]
fn text_length(s: *const u8, e: *const u8, q: u32) -> u64 {
    let raw = e as usize - s as usize;
    if q > 0 {
        (raw - (q as usize / 2 + 1)) as u64
    } else {
        raw as u64
    }
}

const LAG: usize = 16;

#[derive(Copy, Clone)]
struct Pending {
    name: *const u8,
    len: u32,
    country: u32,
    probe: Probe,
}

impl Default for Pending {
    fn default() -> Self {
        Self {
            name: std::ptr::null(),
            len: 0,
            country: 0,
            probe: Probe { slot: 0, tag: 0 },
        }
    }
}

#[derive(Copy, Clone, Default)]
struct Mark {
    country: u32,
    id: u32,
}

unsafe fn parse_records(
    mut p: *const u8,
    end: *const u8,
    file_end: *const u8,
    country_of: *const u8,
    regions: usize,
    names: &NameTable,
    seen: &NamesSeen,
    totals: &mut [Totals; 256],
) {
    let mut pending = [Pending::default(); LAG];
    let mut marks = [Mark::default(); LAG];
    let mut n = 0u64;
    let mut last = u64::MAX;

    let advance =
        |n_val: u64, pending_arr: &[Pending; LAG], marks_arr: &mut [Mark; LAG], last_val: u64| {
            let k = (n_val as usize) % LAG;
            if n_val >= 2 * (LAG as u64) && n_val - 2 * (LAG as u64) < last_val {
                seen.mark(marks_arr[k].country, marks_arr[k].id);
            }
            if n_val >= (LAG as u64) && n_val - (LAG as u64) < last_val {
                let old = &pending_arr[k];
                marks_arr[k] = Mark {
                    country: old.country,
                    id: names.id(
                        old.name,
                        old.len,
                        Probe {
                            slot: old.probe.slot,
                            tag: old.probe.tag,
                        },
                    ),
                };
                seen.prefetch(marks_arr[k].country, marks_arr[k].id);
            }
        };

    let mut sep = Structure::new(p, file_end);
    while p < end {
        let f1 = sep.next();
        let f2 = sep.next();
        let f3 = sep.next();
        let f4_at = sep.next().at;
        let eol_at = sep.next().at;

        if *p == 0xEF {
            p = p.add(3);
        }
        let id = digits(p, f1.at);
        let region = digits(f3.at.add(1), f4_at) as usize;
        let amount = f4_at.add(1);
        let amount_end = if *eol_at.sub(1) == b'\r' {
            eol_at.sub(1)
        } else {
            eol_at
        };
        let negative = *amount == b'-';
        let amt_digits_start = if negative { amount.add(1) } else { amount };
        let mut cents = (digits(amt_digits_start, amount_end.sub(3)) as i64) * 100
            + (digits(amount_end.sub(2), amount_end) as i64);
        if negative {
            cents = -cents;
        }

        let c = if region < regions {
            *country_of.add(region) as usize
        } else {
            0
        };
        let t = &mut totals[c];
        t.count += 1;
        t.id_sum += id;
        t.amount_sum += cents;
        t.amount_min = t.amount_min.min(cents);
        t.amount_max = t.amount_max.max(cents);
        let name_quotes = (f2.quotes - f1.quotes) as u32;
        t.name_bytes += text_length(f1.at.add(1), f2.at, name_quotes);
        t.note_bytes += text_length(f2.at.add(1), f3.at, (f3.quotes - f2.quotes) as u32);
        let quoted = if name_quotes != 0 { 1 } else { 0 };

        advance(n, &pending, &mut marks, last);

        let k = (n as usize) % LAG;
        let name_ptr = f1.at.add(1 + quoted);
        let name_len = (f2.at as usize - f1.at as usize - 1 - 2 * quoted) as u32;
        let probe = names.probe(name_ptr, name_len);
        pending[k] = Pending {
            name: name_ptr,
            len: name_len,
            country: c as u32,
            probe,
        };

        p = eol_at.add(1);
        n += 1;
    }
    last = n;
    while n < last + 2 * (LAG as u64) {
        advance(n, &pending, &mut marks, last);
        n += 1;
    }
}

#[inline(always)]
unsafe fn country_code(p: *const u8) -> u32 {
    ((*p as u32 - 'A' as u32) << 10)
        | ((*p.add(1) as u32 - 'A' as u32) << 5)
        | (*p.add(2) as u32 - 'A' as u32)
}

fn split_delim(data: *const u8, from: usize, size: usize, n: usize, delim: &[u8]) -> Vec<usize> {
    let mut cut = vec![size; n + 1];
    cut[0] = from;
    for t in 1..n {
        let approx = cut[t - 1].max(from + (size - from) * t / n);
        let mut hit = None;
        let mut p = approx;
        while p + delim.len() <= size {
            unsafe {
                if *data.add(p) == delim[0] && (delim.len() == 1 || *data.add(p + 1) == delim[1]) {
                    hit = Some(p);
                    break;
                }
            }
            p += 1;
        }
        cut[t] = if let Some(h) = hit {
            h + delim.len()
        } else {
            size
        };
    }
    cut
}

fn put_money(out: &mut String, cents: i64) {
    let a = if cents < 0 {
        (-cents) as u64
    } else {
        cents as u64
    };
    if cents < 0 {
        out.push_str(" -");
    } else {
        out.push(' ');
    }
    out.push_str(&(a / 100).to_string());
    out.push('.');
    out.push((b'0' + ((a % 100) / 10) as u8) as char);
    out.push((b'0' + (a % 10) as u8) as char);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !std::is_x86_feature_detected!("avx2") || !std::is_x86_feature_detected!("pclmulqdq") {
        return Err("Track C requires AVX2 and PCLMULQDQ".into());
    }
    unsafe {
        prctl(41, 1, 0, 0, 0); // Linux PR_SET_THP_DISABLE
    }

    let mut data_dir = PathBuf::from("/data");
    let mut out_dir = PathBuf::from("/out");

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--data" && i + 1 < args.len() {
            data_dir = PathBuf::from(&args[i + 1]);
            i += 2;
        } else if args[i] == "--out" && i + 1 < args.len() {
            out_dir = PathBuf::from(&args[i + 1]);
            i += 2;
        } else {
            i += 1;
        }
    }
    if let Ok(env_data) = std::env::var("DATA_DIR") {
        if !env_data.is_empty() {
            data_dir = PathBuf::from(env_data);
        }
    }
    if let Ok(env_out) = std::env::var("OUT_DIR") {
        if !env_out.is_empty() {
            out_dir = PathBuf::from(env_out);
        }
    }

    let records_path = data_dir.join("records.csv");
    let regions_path = data_dir.join("regions.csv");
    let out_path = out_dir.join("aggregates.txt");

    std::fs::create_dir_all(&out_dir).ok();

    if !records_path.exists() || !regions_path.exists() {
        File::create(&out_path)?;
        return Ok(());
    }

    let regions_file = File::open(&regions_path)?;
    let records_file = File::open(&records_path)?;

    let (regions, rsize) = unsafe { map_file(&regions_file) };
    let (records, size) = unsafe { map_file(&records_file) };

    let nthreads = 12;

    let nregions = region_count(unsafe { std::slice::from_raw_parts(regions, rsize) });

    let country_of = unsafe { zeroed::<u8>(nregions) };

    // Header end
    let r_header_end = unsafe {
        let mut p = 0;
        while p < rsize && *regions.add(p) != b'\n' {
            p += 1;
        }
        (p + 1).min(rsize)
    };
    let rcut = split_delim(regions, r_header_end, rsize, nthreads, b"\n");

    let present: Vec<AtomicU8> = (0..(1 << 15)).map(|_| AtomicU8::new(0)).collect();
    let index_of = [0u8; 1 << 15];
    let codes = Vec::new();

    // Barrier for country index computation
    let barrier1 = Barrier::new(nthreads);
    let barrier2 = Barrier::new(nthreads);
    let barrier3 = Barrier::new(nthreads);

    let rec_header_end = unsafe {
        let mut p = 0;
        while p < size && *records.add(p) != b'\n' {
            p += 1;
        }
        (p + 1).min(size)
    };
    let cut = split_delim(records, rec_header_end, size, nthreads, b"\r\n");

    let mut totals = vec![[Totals::default(); 256]; nthreads];
    let names = NameTable::new();
    let seen = NamesSeen::new();

    let present_ref = &present;
    let codes_box = std::sync::Mutex::new(codes);
    let index_of_box = std::sync::Mutex::new(index_of);

    let regions_send = SendPtr(regions as *mut u8);
    let records_send = SendPtr(records as *mut u8);
    let country_of_send = SendPtr(country_of);

    let rcut_ref = &rcut;
    let cut_ref = &cut;

    thread::scope(|s| {
        for (t, totals_t) in totals.iter_mut().enumerate() {
            let b1 = &barrier1;
            let b2 = &barrier2;
            let b3 = &barrier3;
            let codes_m = &codes_box;
            let index_of_m = &index_of_box;
            let names_ref = &names;
            let seen_ref = &seen;
            let r_ptr = regions_send;
            let rec_ptr = records_send;
            let c_of_ptr = country_of_send;

            s.spawn(move || unsafe {
                let from = r_ptr.as_ptr().add(rcut_ref[t]);
                let to = r_ptr.as_ptr().add(rcut_ref[t + 1]);
                let mut p = from;
                while p < to {
                    let mut comma = p;
                    while comma < to && *comma != b',' {
                        comma = comma.add(1);
                    }
                    if comma < to {
                        present_ref[country_code(comma.add(1)) as usize]
                            .store(1, Ordering::Relaxed);
                    }
                    while p < to && *p != b'\n' {
                        p = p.add(1);
                    }
                    if p < to {
                        p = p.add(1);
                    }
                }

                b1.wait();

                if t == 0 {
                    let mut c_vec = codes_m.lock().unwrap();
                    let mut idx_arr = index_of_m.lock().unwrap();
                    for k in 0..present_ref.len() {
                        if present_ref[k].load(Ordering::Relaxed) != 0 {
                            c_vec.push(k as u32);
                            idx_arr[k] = c_vec.len() as u8;
                        }
                    }
                }

                b2.wait();

                // Build direct country_of table for thread's slice
                let idx_arr = index_of_m.lock().unwrap();
                let mut q = from;
                while q < to {
                    let mut comma = q;
                    while comma < to && *comma != b',' {
                        comma = comma.add(1);
                    }
                    if comma < to {
                        let reg_ptr = q;
                        let reg_id = digits(reg_ptr, comma) as usize;
                        if reg_id < nregions {
                            *c_of_ptr.as_mut_ptr().add(reg_id) =
                                idx_arr[country_code(comma.add(1)) as usize];
                        }
                    }
                    while q < to && *q != b'\n' {
                        q = q.add(1);
                    }
                    if q < to {
                        q = q.add(1);
                    }
                }
                drop(idx_arr);

                b3.wait();

                // Parse records in parallel
                parse_records(
                    rec_ptr.as_ptr().add(cut_ref[t]),
                    rec_ptr.as_ptr().add(cut_ref[t + 1]),
                    rec_ptr.as_ptr().add(size),
                    c_of_ptr.as_ptr(),
                    nregions,
                    names_ref,
                    seen_ref,
                    totals_t,
                );
            });
        }
    });

    let codes = codes_box.into_inner().unwrap();
    let num_c = codes.len();

    // Count distinct per country
    for c in 0..=num_c {
        let d = unsafe { seen.count(c as u32) };
        totals[0][c].distinct = d;
    }

    // Unmap raw inputs
    unsafe {
        munmap(regions as *mut u8, rsize + 4096);
        munmap(records as *mut u8, size + 4096);
    }

    // Merge totals across threads
    let mut sum_totals = [Totals::default(); 256];
    for t in 0..nthreads {
        for c in 0..=num_c {
            let src = &totals[t][c];
            let dst = &mut sum_totals[c];
            dst.count += src.count;
            dst.id_sum += src.id_sum;
            dst.name_bytes += src.name_bytes;
            dst.note_bytes += src.note_bytes;
            dst.amount_sum += src.amount_sum;
            dst.amount_min = dst.amount_min.min(src.amount_min);
            dst.amount_max = dst.amount_max.max(src.amount_max);
        }
    }

    // Output formatted lines
    let mut out = String::with_capacity(32768);
    for idx in 0..num_c {
        let c = idx + 1;
        let t = &sum_totals[c];
        if t.count == 0 {
            continue;
        }
        let code_val = codes[idx];
        out.push((b'A' + ((code_val >> 10) & 31) as u8) as char);
        out.push((b'A' + ((code_val >> 5) & 31) as u8) as char);
        out.push((b'A' + (code_val & 31) as u8) as char);
        out.push(' ');
        out.push_str(&t.count.to_string());
        out.push(' ');
        out.push_str(&t.id_sum.to_string());
        put_money(&mut out, t.amount_sum);
        put_money(&mut out, t.amount_min);
        put_money(&mut out, t.amount_max);
        out.push(' ');
        out.push_str(&totals[0][c].distinct.to_string());
        out.push(' ');
        out.push_str(&t.name_bytes.to_string());
        out.push(' ');
        out.push_str(&t.note_bytes.to_string());
        out.push('\n');
    }

    // Unmatched
    let un_t = &sum_totals[0];
    if un_t.count > 0 {
        out.push_str("_UNMATCHED ");
        out.push_str(&un_t.count.to_string());
        out.push(' ');
        out.push_str(&un_t.id_sum.to_string());
        put_money(&mut out, un_t.amount_sum);
        put_money(&mut out, un_t.amount_min);
        put_money(&mut out, un_t.amount_max);
        out.push(' ');
        out.push_str(&totals[0][0].distinct.to_string());
        out.push(' ');
        out.push_str(&un_t.name_bytes.to_string());
        out.push(' ');
        out.push_str(&un_t.note_bytes.to_string());
        out.push('\n');
    }

    let mut out_file = File::create(&out_path)?;
    out_file.write_all(out.as_bytes())?;
    out_file.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_region_and_chunk_boundaries() {
        for ending in ["", "\n"] {
            let text = format!("region_id,country\n1,AAA\n1234567,BBB{ending}");
            assert_eq!(region_count(text.as_bytes()), 1234568);
            for threads in 1..32 {
                let from = text.find('\n').unwrap() + 1;
                let cuts = split_delim(text.as_ptr(), from, text.len(), threads, b"\n");
                assert_eq!(cuts[0], from);
                assert_eq!(*cuts.last().unwrap(), text.len());
                for pair in cuts.windows(2) {
                    assert!(pair[0] <= pair[1]);
                    if pair[0] < text.len() {
                        assert_eq!(text.as_bytes()[pair[0] - 1], b'\n');
                    }
                }
            }
        }
        assert_eq!(Totals::default().amount_min, i64::MAX);
        assert_eq!(Totals::default().amount_max, i64::MIN);
    }

    #[test]
    fn concurrent_names_have_one_id_even_after_failed_claims() {
        let table = NameTable::new();
        let barrier = Barrier::new(12);
        let data: Vec<Vec<u8>> = (0..256)
            .map(|i| {
                let mut bytes = vec![0; 8];
                bytes
                    .extend_from_slice(format!("name-{i:04}-with-a-long-shared-prefix").as_bytes());
                bytes.extend_from_slice(&[0; 64]);
                bytes
            })
            .collect();
        thread::scope(|scope| {
            let handles: Vec<_> = (0..12)
                .map(|_| {
                    scope.spawn(|| {
                        data.iter()
                            .map(|bytes| unsafe {
                                let p = bytes.as_ptr().add(8);
                                let len = (bytes.len() - 72) as u32;
                                let probe = table.probe(p, len);
                                barrier.wait();
                                table.id(p, len, probe)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            let ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            for other in &ids[1..] {
                assert_eq!(ids[0], *other);
            }
        });
        assert_eq!(table.next_id.load(Ordering::Relaxed), 256);
    }

    #[test]
    fn full_fast_table_spills_without_losing_distinct_names() {
        let table = NameTable::new();
        let seen = NamesSeen::new();
        let bytes = [b'x'; 80];
        unsafe {
            for slot in 0..SLOTS {
                (*table.tags.add(slot)).store(1, Ordering::Relaxed);
            }
            table.next_id.store(IDS as u32, Ordering::Relaxed);
            let a = table.id(bytes.as_ptr().add(8), 8, Probe { slot: 0, tag: 2 });
            let b = table.id(bytes.as_ptr().add(8), 8, Probe { slot: 0, tag: 2 });
            assert_eq!(a, b);
            seen.mark(3, a);
            seen.mark(3, b);
            assert_eq!(seen.count(3), 1);
            assert_eq!(seen.count(4), 0);
        }
    }
}
