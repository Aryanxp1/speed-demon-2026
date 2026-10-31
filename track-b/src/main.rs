use std::arch::x86_64::*;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn prctl(option: i32, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32;
    fn _exit(status: i32) -> !;
}

const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FIXED: i32 = 0x10;
const MAP_FAILED: *mut u8 = !0 as *mut u8;

const MAX_TERM: usize = 32;
const NONE: u32 = 0xFFFFFFFF;
const NTHREADS: usize = 12;

struct LowerTable {
    t: [u8; 256],
}

impl LowerTable {
    const fn new() -> Self {
        let mut t = [0u8; 256];
        let mut c = b'0';
        while c <= b'9' {
            t[c as usize] = c;
            c += 1;
        }
        let mut c = b'a';
        while c <= b'z' {
            t[c as usize] = c;
            c += 1;
        }
        let mut c = b'A';
        while c <= b'Z' {
            t[c as usize] = c + 32;
            c += 1;
        }
        Self { t }
    }
}

static LOWER: LowerTable = LowerTable::new();

struct SendPtr<T>(*const T);
impl<T> Clone for SendPtr<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        SendPtr(self.0)
    }
}
impl<T> Copy for SendPtr<T> {}
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

impl<T> SendPtr<T> {
    #[inline(always)]
    fn as_ptr(&self) -> *const T {
        self.0
    }
}

unsafe fn map_file(file: &File) -> (*const u8, usize) {
    let size = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
    if size == 0 {
        return (std::ptr::null(), 0);
    }
    let fd = file.as_raw_fd();
    let p = mmap(
        std::ptr::null_mut(),
        size + 4096,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if p == MAP_FAILED {
        _exit(1);
    }
    if mmap(p, size, PROT_READ, MAP_PRIVATE | MAP_FIXED, fd, 0) == MAP_FAILED {
        _exit(1);
    }
    (p as *const u8, size)
}

#[repr(C, align(32))]
#[derive(Copy, Clone, Default)]
struct Key {
    w: [u64; 4],
}

#[inline(always)]
fn hash_key(k: &Key) -> u64 {
    let h = (k.w[0].wrapping_add(k.w[2].wrapping_mul(0x94D049BB133111EB)))
        .wrapping_mul(0x9E3779B97F4A7C15)
        ^ (k.w[1].wrapping_add(k.w[3].wrapping_mul(0xD6E8FEB86659FD93)))
            .wrapping_mul(0xBF58476D1CE4E5B9);
    h ^ (h >> 29)
}

#[inline(always)]
fn same_key(a: &Key, b: &Key) -> bool {
    ((a.w[0] ^ b.w[0]) | (a.w[1] ^ b.w[1]) | (a.w[2] ^ b.w[2]) | (a.w[3] ^ b.w[3])) == 0
}

struct TermTable {
    slots: Vec<u64>,
    keys: Vec<Key>,
    lens: Vec<u8>,
    mask: usize,
}

impl TermTable {
    fn new() -> Self {
        let mut t = Self {
            slots: Vec::new(),
            keys: Vec::new(),
            lens: Vec::new(),
            mask: 0,
        };
        t.rehash(1 << 16);
        t
    }

    #[inline(always)]
    fn size(&self) -> usize {
        self.keys.len()
    }

    #[inline(always)]
    fn bytes(&self, id: u32) -> *const u8 {
        self.keys[id as usize].w.as_ptr() as *const u8
    }

    #[inline(always)]
    fn length(&self, id: u32) -> usize {
        self.lens[id as usize] as usize
    }

    fn insert(&mut self, k: Key, len: usize) -> u32 {
        if (self.keys.len() + 1) * 2 > self.slots.len() {
            let new_cap = self.slots.len() * 2;
            self.rehash(new_cap);
        }
        let h = hash_key(&k);
        let mut i = (h as usize) & self.mask;
        loop {
            let s = self.slots[i];
            if s == 0 {
                let id = self.keys.len() as u32;
                self.keys.push(k);
                self.lens.push(len as u8);
                self.slots[i] = (h & 0xFFFFFFFF00000000) | ((id as u64) + 1);
                return id;
            }
            if (s >> 32) == (h >> 32) && same_key(&self.keys[(s as u32 - 1) as usize], &k) {
                return (s as u32) - 1;
            }
            i = (i + 1) & self.mask;
        }
    }

    fn renumber(&mut self, new_id: &[u32]) {
        let n = self.keys.len();
        let mut new_keys = vec![Key::default(); n];
        let mut new_lens = vec![0u8; n];
        for i in 0..n {
            let nid = new_id[i] as usize;
            new_keys[nid] = self.keys[i];
            new_lens[nid] = self.lens[i];
        }
        self.keys = new_keys;
        self.lens = new_lens;
        let cap = self.slots.len();
        self.rehash(cap);
    }

    #[inline(always)]
    fn prefetch(&self, h: u64) {
        unsafe {
            _mm_prefetch(
                self.slots.as_ptr().add((h as usize) & self.mask) as *const i8,
                _MM_HINT_T0,
            );
        }
    }

    #[inline(always)]
    fn find(&self, k: &Key, h: u64) -> u32 {
        let mut i = (h as usize) & self.mask;
        loop {
            let s = self.slots[i];
            if s == 0 {
                return NONE;
            }
            if (s >> 32) == (h >> 32) && same_key(&self.keys[(s as u32 - 1) as usize], k) {
                return (s as u32) - 1;
            }
            i = (i + 1) & self.mask;
        }
    }

    fn rehash(&mut self, cap: usize) {
        self.slots.clear();
        self.slots.resize(cap, 0);
        self.mask = cap - 1;
        for id in 0..self.keys.len() {
            let h = hash_key(&self.keys[id]);
            let mut i = (h as usize) & self.mask;
            while self.slots[i] != 0 {
                i = (i + 1) & self.mask;
            }
            self.slots[i] = (h & 0xFFFFFFFF00000000) | ((id as u64) + 1);
        }
    }
}

#[inline(always)]
unsafe fn token_bytes_256(q: *const u8) -> u32 {
    let v = _mm256_loadu_si256(q as *const __m256i);
    let lower_v = _mm256_or_si256(v, _mm256_set1_epi8(0x20));
    let letter = _mm256_sub_epi8(lower_v, _mm256_set1_epi8(b'a' as i8));
    let digit = _mm256_sub_epi8(v, _mm256_set1_epi8(b'0' as i8));
    let zero = _mm256_setzero_si256();
    let is_letter = _mm256_cmpeq_epi8(_mm256_subs_epu8(letter, _mm256_set1_epi8(25)), zero);
    let is_digit = _mm256_cmpeq_epi8(_mm256_subs_epu8(digit, _mm256_set1_epi8(9)), zero);
    let in_set = _mm256_or_si256(is_letter, is_digit);
    _mm256_movemask_epi8(in_set) as u32
}

#[inline(always)]
unsafe fn token_bytes(p: *const u8) -> u64 {
    let lo = token_bytes_256(p) as u64;
    let hi = token_bytes_256(p.add(32)) as u64;
    lo | (hi << 32)
}

#[inline(always)]
unsafe fn for_each_token<F: FnMut(u32)>(
    mut q: *const u8,
    e: *const u8,
    terms: &TermTable,
    mut emit: F,
) {
    const LAG: usize = 8;
    let mut keys = [Key::default(); LAG];
    let mut hashes = [0u64; LAG];
    let mut n = 0usize;
    let mut m = 0u64;
    let mut fresh = true;

    while q < e {
        if fresh {
            m = token_bytes(q);
            let rem = e as usize - q as usize;
            if rem < 64 {
                m &= (1u64 << rem).wrapping_sub(1);
            }
            fresh = false;
        }
        if m == 0 {
            q = q.add(64);
            fresh = true;
            continue;
        }
        let s_idx = m.trailing_zeros() as usize;
        let s = q.add(s_idx);
        let after = (!m) & (!0u64 << s_idx);
        if after == 0 && s != q {
            q = s;
            fresh = true;
            continue;
        }
        let mut t = if after != 0 {
            q.add(after.trailing_zeros() as usize)
        } else {
            s.add(64)
        };
        while t < e && LOWER.t[*t as usize] != 0 {
            t = t.add(1);
        }
        m &= if after != 0 {
            !0u64 << after.trailing_zeros()
        } else {
            0
        };
        if t > q.add(64) {
            q = t;
            fresh = true;
        }
        let len = t as usize - s as usize;
        if len > MAX_TERM {
            continue;
        }
        let mut k = Key::default();
        let mut i = 0usize;
        while i < 4 {
            let b = if len > 8 * i { (len - 8 * i).min(8) } else { 0 };
            let w = (s.add(8 * i) as *const u64).read_unaligned();
            let mask = if b == 8 {
                !0u64
            } else {
                (1u64 << (8 * b)).wrapping_sub(1)
            };
            k.w[i] = (w | 0x2020202020202020) & mask;
            i += 1;
        }
        let h = hash_key(&k);
        terms.prefetch(h);
        let slot = n % LAG;
        if n >= LAG {
            emit(terms.find(&keys[slot], hashes[slot]));
        }
        keys[slot] = k;
        hashes[slot] = h;
        n += 1;
    }

    let start = if n >= LAG { n - LAG } else { 0 };
    for i in start..n {
        let slot = i % LAG;
        emit(terms.find(&keys[slot], hashes[slot]));
    }
}

struct Query {
    phrase: bool,
    terms: Vec<u32>,
    ops: Vec<u8>,
}

fn parse_queries(data: *const u8, size: usize, terms: &mut TermTable) -> Vec<Query> {
    let mut queries = Vec::new();
    let mut p = data;
    let end = unsafe { data.add(size) };

    while p < end {
        let mut nl = p;
        while nl < end && unsafe { *nl } != b'\n' {
            nl = unsafe { nl.add(1) };
        }
        let phrase = unsafe { *p == b'P' };
        let mut s = unsafe { p.add(if phrase { 8 } else { 5 }) };
        let e = if phrase { unsafe { nl.sub(1) } } else { nl };

        let mut q_terms = Vec::new();
        let mut q_ops = Vec::new();
        let mut word = 0;

        while s < e {
            let mut sp = s;
            while sp < e && unsafe { *sp } != b' ' {
                sp = unsafe { sp.add(1) };
            }
            if !phrase && word % 2 == 1 {
                let op = unsafe { *s };
                q_ops.push(op);
            } else {
                let len = sp as usize - s as usize;
                let mut k = Key::default();
                let copy_len = len.min(MAX_TERM);
                unsafe {
                    std::ptr::copy_nonoverlapping(s, k.w.as_mut_ptr() as *mut u8, copy_len);
                }
                let id = terms.insert(k, len);
                q_terms.push(id);
            }
            s = if sp < e { unsafe { sp.add(1) } } else { e };
            word += 1;
        }

        queries.push(Query {
            phrase,
            terms: q_terms,
            ops: q_ops,
        });

        p = if nl < end { unsafe { nl.add(1) } } else { end };
    }
    queries
}

#[allow(dead_code)]
struct Index {
    docs: u32,
    words: usize,
    dense: Vec<u32>,
    bits: Vec<AtomicU64>,
    df: Vec<u32>,
    threads: usize,
    first: Vec<u32>,
    start: Vec<u32>,
    seg_end: Vec<Vec<u32>>,
    post: Vec<u16>,
    wide_post: Vec<u32>,
    text: Vec<u32>,
}

unsafe impl Send for Index {}
unsafe impl Sync for Index {}

const BUCKET_BITS: u32 = 9;

#[inline(always)]
unsafe fn parse_id(mut p: *const u8) -> u32 {
    let mut v = 0u32;
    while *p >= b'0' && *p <= b'9' {
        v = v * 10 + (*p - b'0') as u32;
        p = p.add(1);
    }
    v
}

fn build_index(
    base: *const u8,
    size: usize,
    terms: &mut TermTable,
    queries: &mut [Query],
    nthreads: usize,
) -> Index {
    let nterms = terms.size() as u32;
    let mut docs = 0u32;
    if size > 0 {
        let mut last = size - 1;
        while last > 0 && unsafe { *base.add(last - 1) } != b'\n' {
            last -= 1;
        }
        docs = unsafe { parse_id(base.add(last)) } + 1;
    }

    let words = ((docs as usize) + 63) / 64;
    let mut dense = vec![NONE; nterms as usize];
    let nbuckets = ((nterms >> BUCKET_BITS) + 1) as usize;

    let mut cut = vec![size; nthreads + 1];
    for t in 0..=nthreads {
        let mut c = size * t / nthreads;
        while c > 0 && c < size && unsafe { *base.add(c - 1) } != b'\n' {
            c += 1;
        }
        cut[t] = c;
    }

    let mut seen: Vec<Vec<Vec<u32>>> = (0..nthreads)
        .map(|_| (0..nbuckets).map(|_| Vec::new()).collect())
        .collect();
    let mut count: Vec<Vec<u32>> = (0..nthreads).map(|_| vec![0u32; nterms as usize]).collect();
    let mut sampled = vec![0u32; nthreads];
    let mut first_docs = vec![0u32; nthreads];
    let mut text = vec![0u32; 2 * docs as usize];

    let text_send = SendPtr(text.as_mut_ptr());
    let base_send = SendPtr(base);

    thread::scope(|s| {
        let cut_ref = &cut;
        let terms_ref = &terms;

        let mut handles = Vec::new();

        for t in 0..nthreads {
            let b_ptr = base_send;
            let tx_ptr = text_send;
            let c_ref = cut_ref;

            handles.push(s.spawn(move || unsafe {
                let begin = b_ptr.as_ptr().add(c_ref[t]);
                let end = b_ptr.as_ptr().add(c_ref[t + 1]);
                let first = if begin < end { parse_id(begin) } else { 0 };

                let mut last_doc = vec![NONE; nterms as usize];
                let mut local_cnt = vec![0u32; nterms as usize];

                let stop = begin.add((end as usize - begin as usize) / 32);
                let mut p = begin;
                let mut doc = first;

                while p < stop {
                    let mut nl = p;
                    while nl < end && *nl != b'\n' {
                        nl = nl.add(1);
                    }
                    let mut tx = p;
                    while tx < nl && *tx != b'\t' {
                        tx = tx.add(1);
                    }
                    if tx < nl {
                        tx = tx.add(1);
                    }
                    let d_idx = doc as usize;
                    *tx_ptr.as_ptr().cast_mut().add(2 * d_idx) =
                        (tx as usize - b_ptr.as_ptr() as usize) as u32;
                    *tx_ptr.as_ptr().cast_mut().add(2 * d_idx + 1) =
                        (nl as usize - b_ptr.as_ptr() as usize) as u32;

                    for_each_token(tx, nl, terms_ref, |id| {
                        if id != NONE && last_doc[id as usize] != doc {
                            last_doc[id as usize] = doc;
                            local_cnt[id as usize] += 1;
                        }
                    });

                    doc += 1;
                    p = if nl < end { nl.add(1) } else { end };
                }

                (first, doc - first, local_cnt)
            }));
        }

        for (t, h) in handles.into_iter().enumerate() {
            let (f, samp, cnt) = h.join().unwrap();
            first_docs[t] = f;
            sampled[t] = samp;
            count[t] = cnt;
        }
    });

    // Empty byte ranges still need the next document's base. Use a wide
    // representation if any partition exceeds the entire u16 offset range.
    for t in 0..nthreads {
        first_docs[t] = if cut[t] < size {
            unsafe { parse_id(base.add(cut[t])) }
        } else {
            docs
        };
    }
    let compact = (0..nthreads).all(|t| {
        let end = first_docs.get(t + 1).copied().unwrap_or(docs);
        end - first_docs[t] <= u16::MAX as u32 + 1
    });

    // Sample barrier aggregation
    let mut total_sampled = 0u64;
    for &s in &sampled {
        total_sampled += s as u64;
    }
    let mut df_sample = vec![0u64; nterms as usize];
    let mut order: Vec<u32> = (0..nterms).collect();
    for id in 0..nterms as usize {
        for t in 0..nthreads {
            df_sample[id] += count[t][id] as u64;
        }
    }
    order.sort_by(|&a, &b| df_sample[b as usize].cmp(&df_sample[a as usize]));

    let mut ndense = 0u32;
    let mut new_id = vec![0u32; nterms as usize];
    for rank in 0..nterms as usize {
        let orig = order[rank];
        new_id[orig as usize] = rank as u32;
        if df_sample[orig as usize] * 32 > total_sampled {
            dense[rank] = ndense;
            ndense += 1;
        }
    }

    terms.renumber(&new_id);
    for q in queries.iter_mut() {
        for term_id in &mut q.terms {
            *term_id = new_id[*term_id as usize];
        }
    }

    let bits: Vec<AtomicU64> = (0..(ndense as usize * words))
        .map(|_| AtomicU64::new(0))
        .collect();

    // Second pass: full corpus indexing
    let bits_send = SendPtr(bits.as_ptr());
    let dense_ref = &dense;

    thread::scope(|s| {
        let cut_ref = &cut;
        let terms_ref = &terms;

        let mut handles = Vec::new();

        for t in 0..nthreads {
            let b_ptr = base_send;
            let tx_ptr = text_send;
            let c_ref = cut_ref;
            let f = first_docs[t];
            let d_ref = dense_ref;
            let b_atomic = bits_send;

            handles.push(s.spawn(move || unsafe {
                let begin = b_ptr.as_ptr().add(c_ref[t]);
                let end = b_ptr.as_ptr().add(c_ref[t + 1]);
                let mut local_cnt = vec![0u32; nterms as usize];
                let mut last_doc = vec![NONE; nterms as usize];
                let mut local_seen: Vec<Vec<u32>> = (0..nbuckets).map(|_| Vec::new()).collect();

                let mut p = begin;
                let mut doc = f;

                while p < end {
                    let mut nl = p;
                    while nl < end && *nl != b'\n' {
                        nl = nl.add(1);
                    }
                    let mut tx = p;
                    while tx < nl && *tx != b'\t' {
                        tx = tx.add(1);
                    }
                    if tx < nl {
                        tx = tx.add(1);
                    }
                    let d_idx = doc as usize;
                    *tx_ptr.as_ptr().cast_mut().add(2 * d_idx) =
                        (tx as usize - b_ptr.as_ptr() as usize) as u32;
                    *tx_ptr.as_ptr().cast_mut().add(2 * d_idx + 1) =
                        (nl as usize - b_ptr.as_ptr() as usize) as u32;

                    for_each_token(tx, nl, terms_ref, |id| {
                        if id != NONE && last_doc[id as usize] != doc {
                            last_doc[id as usize] = doc;
                            let d = d_ref[id as usize];
                            if d != NONE {
                                let w_idx = (d as usize) * words + (doc as usize / 64);
                                (*b_atomic.as_ptr().add(w_idx))
                                    .fetch_or(1u64 << (doc % 64), Ordering::Relaxed);
                            } else {
                                let bucket = (id >> BUCKET_BITS) as usize;
                                if compact {
                                    // Nine bucket-local term bits + sixteen document bits.
                                    let offset = doc - f;
                                    debug_assert!(offset <= u16::MAX as u32);
                                    local_seen[bucket]
                                        .push(((id & ((1 << BUCKET_BITS) - 1)) << 16) | offset);
                                } else {
                                    local_seen[bucket].extend_from_slice(&[id, doc]);
                                }
                                local_cnt[id as usize] += 1;
                            }
                        }
                    });

                    doc += 1;
                    p = if nl < end { nl.add(1) } else { end };
                }

                (local_cnt, local_seen)
            }));
        }

        for (t, h) in handles.into_iter().enumerate() {
            let (cnt, s_vec) = h.join().unwrap();
            count[t] = cnt;
            seen[t] = s_vec;
        }
    });

    let mut start = vec![0u32; (nterms + 1) as usize];
    let mut total_postings = 0u32;
    for id in 0..nterms as usize {
        start[id] = total_postings;
        for t in 0..nthreads {
            let c = count[t][id];
            count[t][id] = total_postings;
            total_postings += c;
        }
    }
    start[nterms as usize] = total_postings;

    let mut post = vec![0u16; if compact { total_postings as usize } else { 0 }];
    let mut wide_post = vec![0u32; if compact { 0 } else { total_postings as usize }];
    let post_send = SendPtr(post.as_mut_ptr());
    let wide_send = SendPtr(wide_post.as_mut_ptr());

    let seg_end = thread::scope(|scope| {
        let handles: Vec<_> = seen
            .into_iter()
            .enumerate()
            .map(|(t, buckets)| {
                let mut cursor = count[t].clone();
                scope.spawn(move || unsafe {
                    // Each worker owns disjoint term ranges in the destination.
                    // Consuming buckets releases temporary storage during scatter.
                    for (bucket, items) in buckets.into_iter().enumerate() {
                        if compact {
                            for item in items {
                                let id = (bucket << BUCKET_BITS) | (item >> 16) as usize;
                                *post_send.as_ptr().cast_mut().add(cursor[id] as usize) =
                                    item as u16;
                                cursor[id] += 1;
                            }
                        } else {
                            for pair in items.chunks_exact(2) {
                                let id = pair[0] as usize;
                                *wide_send.as_ptr().cast_mut().add(cursor[id] as usize) = pair[1];
                                cursor[id] += 1;
                            }
                        }
                    }
                    cursor
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let mut df = vec![0u32; nterms as usize];
    for id in 0..nterms as usize {
        df[id] = start[id + 1] - start[id];
        let d = dense[id];
        if d != NONE {
            let mut bits_cnt = 0u32;
            for w in 0..words {
                bits_cnt += bits[d as usize * words + w]
                    .load(Ordering::Relaxed)
                    .count_ones();
            }
            df[id] += bits_cnt;
        }
    }

    Index {
        docs,
        words,
        dense,
        bits,
        df,
        threads: nthreads,
        first: first_docs,
        start,
        seg_end,
        post,
        wide_post,
        text,
    }
}

#[allow(dead_code)]
struct Postings<'a> {
    p: &'a [u32],
    bits: Option<&'a [AtomicU64]>,
    words: usize,
    df: usize,
}

#[inline(always)]
fn has_bit(bits: &[AtomicU64], doc: u32) -> bool {
    let w = doc as usize / 64;
    (bits[w].load(Ordering::Relaxed) >> (doc % 64)) & 1 != 0
}

#[inline(always)]
fn for_each_bit<F: FnMut(u32)>(bits: &[AtomicU64], words: usize, mut f: F) {
    for w in 0..words {
        let mut m = bits[w].load(Ordering::Relaxed);
        while m != 0 {
            let bit_idx = m.trailing_zeros();
            f((w * 64 + bit_idx as usize) as u32);
            m &= m - 1;
        }
    }
}

#[inline(always)]
fn gallop(a: &[u32], mut lo: usize, x: u32) -> usize {
    let n = a.len();
    let mut step = 1;
    let mut hi = lo;
    while hi < n && a[hi] < x {
        lo = hi + 1;
        hi += step;
        step <<= 1;
    }
    let upper = hi.min(n);
    match a[lo..upper].binary_search(&x) {
        Ok(idx) => lo + idx,
        Err(idx) => lo + idx,
    }
}

fn op_and(a: &[u32], b: Postings, out: &mut Vec<u32>) {
    out.clear();
    if let Some(bits) = b.bits {
        for &x in a {
            if has_bit(bits, x) {
                out.push(x);
            }
        }
    } else if a.len() * 16 < b.p.len() {
        let mut j = 0;
        for &x in a {
            j = gallop(b.p, j, x);
            if j == b.p.len() {
                break;
            }
            if b.p[j] == x {
                out.push(x);
            }
        }
    } else if b.p.len() * 16 < a.len() {
        let mut i = 0;
        for &bx in b.p {
            i = gallop(a, i, bx);
            if i == a.len() {
                break;
            }
            if a[i] == bx {
                out.push(bx);
            }
        }
    } else {
        let mut i = 0;
        let mut j = 0;
        while i < a.len() && j < b.p.len() {
            if a[i] == b.p[j] {
                out.push(a[i]);
                i += 1;
                j += 1;
            } else if a[i] < b.p[j] {
                i += 1;
            } else {
                j += 1;
            }
        }
    }
}

fn op_not(a: &[u32], b: Postings, out: &mut Vec<u32>) {
    out.clear();
    if let Some(bits) = b.bits {
        for &x in a {
            if !has_bit(bits, x) {
                out.push(x);
            }
        }
    } else if a.len() * 16 < b.p.len() {
        let mut j = 0;
        for &x in a {
            j = gallop(b.p, j, x);
            if j == b.p.len() || b.p[j] != x {
                out.push(x);
            }
        }
    } else {
        let mut i = 0;
        let mut j = 0;
        while i < a.len() {
            if j == b.p.len() {
                out.extend_from_slice(&a[i..]);
                break;
            }
            if a[i] < b.p[j] {
                out.push(a[i]);
                i += 1;
            } else if a[i] == b.p[j] {
                i += 1;
                j += 1;
            } else {
                j += 1;
            }
        }
    }
}

fn op_or(a: &[u32], b: Postings, out: &mut Vec<u32>) {
    out.clear();
    let bp = b.p;
    let mut i = 0;
    let mut j = 0;
    while i < a.len() && j < bp.len() {
        if a[i] < bp[j] {
            out.push(a[i]);
            i += 1;
        } else if a[i] > bp[j] {
            out.push(bp[j]);
            j += 1;
        } else {
            out.push(a[i]);
            i += 1;
            j += 1;
        }
    }
    if i < a.len() {
        out.extend_from_slice(&a[i..]);
    }
    if j < bp.len() {
        out.extend_from_slice(&bp[j..]);
    }
}

const SEL: [u64; 6] = [
    0xAAAAAAAAAAAAAAAA,
    0xCCCCCCCCCCCCCCCC,
    0xF0F0F0F0F0F0F0F0,
    0xFF00FF00FF00FF00,
    0xFFFF0000FFFF0000,
    0xFFFFFFFF00000000,
];

#[inline(always)]
fn position_sum(m: u64) -> u64 {
    let mut s = 0u64;
    let mut k = 0;
    while k < 6 {
        s += ((m & SEL[k]).count_ones() as u64) << k;
        k += 1;
    }
    s
}

struct Solver<'a> {
    ix: &'a Index,
    terms: &'a TermTable,
    docs_base: *const u8,
    cur: Vec<u32>,
    tmp: Vec<u32>,
    bm: Vec<u64>,
    dense: bool,
    decoded: Vec<u32>,
}

impl<'a> Solver<'a> {
    fn new(ix: &'a Index, terms: &'a TermTable, docs_base: *const u8) -> Self {
        Self {
            ix,
            terms,
            docs_base,
            cur: Vec::new(),
            tmp: Vec::new(),
            bm: Vec::new(),
            dense: false,
            decoded: Vec::new(),
        }
    }

    #[inline(always)]
    fn postings<'b>(&self, id: u32, decoded: &'b mut Vec<u32>) -> Postings<'b>
    where
        'a: 'b,
    {
        let d = self.ix.dense[id as usize];
        if d != NONE {
            let start = d as usize * self.ix.words;
            let end = start + self.ix.words;
            return Postings {
                p: &[],
                bits: Some(&self.ix.bits[start..end]),
                words: self.ix.words,
                df: self.ix.df[id as usize] as usize,
            };
        }
        let start = self.ix.start[id as usize] as usize;
        let end = self.ix.start[id as usize + 1] as usize;
        let p = if !self.ix.wide_post.is_empty() {
            &self.ix.wide_post[start..end]
        } else {
            decoded.clear();
            decoded.reserve(end - start);
            let mut offset = start;
            for t in 0..self.ix.threads {
                let segment_end = self.ix.seg_end[t][id as usize] as usize;
                let base = self.ix.first[t];
                decoded.extend(
                    self.ix.post[offset..segment_end]
                        .iter()
                        .map(|&x| base + x as u32),
                );
                offset = segment_end;
            }
            &decoded[..]
        };
        Postings {
            p,
            bits: None,
            words: self.ix.words,
            df: end - start,
        }
    }

    fn answer(&mut self, q: &Query, out: &mut [u8]) -> usize {
        let mut decoded = std::mem::take(&mut self.decoded);
        if q.phrase {
            self.phrase(q, &mut decoded);
        } else {
            self.boolean(q, &mut decoded);
        }
        self.decoded = decoded;
        self.format(out)
    }

    fn boolean(&mut self, q: &Query, decoded: &mut Vec<u32>) {
        if q.terms.is_empty() {
            self.dense = false;
            self.cur.clear();
            return;
        }
        let first = self.postings(q.terms[0], decoded);
        self.dense = first.bits.is_some();
        if self.dense {
            let b_slice = first.bits.unwrap();
            self.bm.clear();
            for w in b_slice {
                self.bm.push(w.load(Ordering::Relaxed));
            }
        } else {
            self.cur.clear();
            self.cur.extend_from_slice(first.p);
        }

        for i in 0..q.ops.len() {
            let b = self.postings(q.terms[i + 1], decoded);
            let op = q.ops[i];
            if self.dense {
                self.apply_dense(op, b);
            } else if b.bits.is_some() && op == b'O' {
                let b_slice = b.bits.unwrap();
                self.bm.clear();
                for w in b_slice {
                    self.bm.push(w.load(Ordering::Relaxed));
                }
                for &x in &self.cur {
                    self.bm[x as usize / 64] |= 1u64 << (x % 64);
                }
                self.dense = true;
            } else {
                match op {
                    b'A' => op_and(&self.cur, b, &mut self.tmp),
                    b'O' => op_or(&self.cur, b, &mut self.tmp),
                    _ => op_not(&self.cur, b, &mut self.tmp),
                }
                std::mem::swap(&mut self.cur, &mut self.tmp);
            }
        }
    }

    fn apply_dense(&mut self, op: u8, b: Postings) {
        if let Some(bits) = b.bits {
            let n = self.bm.len();
            if op == b'A' {
                for w in 0..n {
                    self.bm[w] &= bits[w].load(Ordering::Relaxed);
                }
            } else if op == b'O' {
                for w in 0..n {
                    self.bm[w] |= bits[w].load(Ordering::Relaxed);
                }
            } else {
                for w in 0..n {
                    self.bm[w] &= !bits[w].load(Ordering::Relaxed);
                }
            }
        } else if op == b'A' {
            self.cur.clear();
            for &x in b.p {
                let w = x as usize / 64;
                if (self.bm[w] >> (x % 64)) & 1 != 0 {
                    self.cur.push(x);
                }
            }
            self.dense = false;
        } else {
            for &x in b.p {
                let bit = 1u64 << (x % 64);
                let w = x as usize / 64;
                if op == b'O' {
                    self.bm[w] |= bit;
                } else {
                    self.bm[w] &= !bit;
                }
            }
        }
    }

    fn phrase(&mut self, q: &Query, decoded: &mut Vec<u32>) {
        self.dense = false;
        if q.terms.is_empty() {
            self.cur.clear();
            return;
        }
        let mut order = q.terms.clone();
        order.sort_unstable();
        order.dedup();
        let ix = self.ix;
        order.sort_by_key(|&id| ix.df[id as usize]);

        let rarest = order[0];
        let p_rarest = self.postings(rarest, decoded);
        self.cur.clear();
        if let Some(bits) = p_rarest.bits {
            for_each_bit(bits, p_rarest.words, |x| self.cur.push(x));
        } else {
            self.cur.extend_from_slice(p_rarest.p);
        }

        for &id in &order[1..] {
            if self.cur.is_empty() {
                break;
            }
            op_and(&self.cur, self.postings(id, decoded), &mut self.tmp);
            std::mem::swap(&mut self.cur, &mut self.tmp);
        }

        let j = q.terms.iter().position(|&t| t == rarest).unwrap();
        let mut kept = 0;
        let n_cand = self.cur.len();

        for i in 0..n_cand {
            let doc = self.cur[i];
            let tx_start = self.ix.text[2 * doc as usize] as usize;
            let tx_end = self.ix.text[2 * doc as usize + 1] as usize;
            let b = unsafe { self.docs_base.add(tx_start) };
            let e = unsafe { self.docs_base.add(tx_end) };

            if self.contains(b, e, &q.terms, j) {
                self.cur[kept] = doc;
                kept += 1;
            }
        }
        self.cur.truncate(kept);
    }

    #[inline(always)]
    fn is_term(&self, p: *const u8, len: usize, id: u32) -> bool {
        if len != self.terms.length(id) {
            return false;
        }
        let k = self.terms.bytes(id);
        let mut i = 0;
        while i < len {
            unsafe {
                if LOWER.t[*p.add(i) as usize] != *k.add(i) {
                    return false;
                }
            }
            i += 1;
        }
        true
    }

    fn contains(&self, b: *const u8, e: *const u8, phrase: &[u32], j: usize) -> bool {
        let anchor = phrase[j];
        let first = unsafe { *self.terms.bytes(anchor) };
        let want = unsafe { _mm256_set1_epi8(first as i8) };
        let case_bit = unsafe { _mm256_set1_epi8(0x20) };

        let mut p = b;
        while p < e {
            let rem = e as usize - p as usize;
            let mut m = 0u32;
            if rem >= 32 {
                unsafe {
                    let v = _mm256_loadu_si256(p as *const __m256i);
                    let cmp = _mm256_cmpeq_epi8(_mm256_or_si256(v, case_bit), want);
                    m = _mm256_movemask_epi8(cmp) as u32;
                }
            } else {
                for i in 0..rem {
                    unsafe {
                        if (*p.add(i) | 0x20) == first {
                            m |= 1u32 << i;
                        }
                    }
                }
            }

            while m != 0 {
                let off = m.trailing_zeros() as usize;
                let s = unsafe { p.add(off) };
                if s > b && unsafe { LOWER.t[*s.sub(1) as usize] != 0 } {
                    m &= m - 1;
                    continue;
                }
                let mut t = s;
                while t < e && unsafe { LOWER.t[*t as usize] != 0 } {
                    t = unsafe { t.add(1) };
                }
                let len = t as usize - s as usize;
                if self.is_term(s, len, anchor) && self.around(b, e, s, t, phrase, j) {
                    return true;
                }
                m &= m - 1;
            }

            p = unsafe { p.add(32) };
        }
        false
    }

    fn around(
        &self,
        b: *const u8,
        e: *const u8,
        mut s: *const u8,
        mut t: *const u8,
        phrase: &[u32],
        j: usize,
    ) -> bool {
        let mut i = j;
        while i > 0 {
            i -= 1;
            while s > b && unsafe { LOWER.t[*s.sub(1) as usize] == 0 } {
                s = unsafe { s.sub(1) };
            }
            let end_w = s;
            while s > b && unsafe { LOWER.t[*s.sub(1) as usize] != 0 } {
                s = unsafe { s.sub(1) };
            }
            let len = end_w as usize - s as usize;
            if s == end_w || !self.is_term(s, len, phrase[i]) {
                return false;
            }
        }

        let mut i = j + 1;
        while i < phrase.len() {
            while t < e && unsafe { LOWER.t[*t as usize] == 0 } {
                t = unsafe { t.add(1) };
            }
            let start_w = t;
            while t < e && unsafe { LOWER.t[*t as usize] != 0 } {
                t = unsafe { t.add(1) };
            }
            let len = t as usize - start_w as usize;
            if t == start_w || !self.is_term(start_w, len, phrase[i]) {
                return false;
            }
            i += 1;
        }

        true
    }

    fn format(&self, out: &mut [u8]) -> usize {
        let mut count = 0u64;
        let mut sum = 0u64;
        let mut smallest = [0u32; 10];
        let mut m = 0usize;

        if self.dense {
            for (w, &word_atomic) in self.bm.iter().enumerate() {
                let mut bits = word_atomic;
                let c = bits.count_ones() as u64;
                count += c;
                sum += c * 64 * (w as u64) + position_sum(bits);
                while bits != 0 && m < 10 {
                    smallest[m] = (w * 64 + bits.trailing_zeros() as usize) as u32;
                    m += 1;
                    bits &= bits - 1;
                }
            }
        } else {
            count = self.cur.len() as u64;
            for &x in &self.cur {
                sum += x as u64;
            }
            while m < self.cur.len() && m < 10 {
                smallest[m] = self.cur[m];
                m += 1;
            }
        }

        let mut pos = 0;
        pos = append_u64(out, count, pos);
        out[pos] = b' ';
        pos += 1;

        pos = append_u64(out, sum, pos);

        for i in 0..m {
            out[pos] = b' ';
            pos += 1;
            pos = append_u64(out, smallest[i] as u64, pos);
        }
        out[pos] = b'\n';
        pos + 1
    }
}

#[inline(always)]
fn append_u64(buf: &mut [u8], mut val: u64, mut pos: usize) -> usize {
    if val == 0 {
        buf[pos] = b'0';
        return pos + 1;
    }
    let mut temp = [0u8; 20];
    let mut len = 0;
    while val > 0 {
        temp[len] = b'0' + (val % 10) as u8;
        val /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        buf[pos] = temp[len];
        pos += 1;
    }
    pos
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        prctl(14, 1, 0, 0, 0); // PR_SET_THP_DISABLE = 14
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

    std::fs::create_dir_all(&out_dir).ok();

    let q_path = data_dir.join("queries.txt");
    let d_path = data_dir.join("docs.txt");
    let out_path = out_dir.join("results.txt");

    if !q_path.exists() || !d_path.exists() {
        File::create(&out_path)?;
        return Ok(());
    }

    let q_file = File::open(&q_path)?;
    let d_file = File::open(&d_path)?;

    let (q_data, q_size) = unsafe { map_file(&q_file) };
    let (d_data, d_size) = unsafe { map_file(&d_file) };

    let mut terms = TermTable::new();
    let mut queries = parse_queries(q_data, q_size, &mut terms);
    let ix = build_index(d_data, d_size, &mut terms, &mut queries, NTHREADS);

    const SLOT: usize = 128;
    let nq = queries.len();
    let mut slots = vec![0u8; nq * SLOT];
    let mut lens = vec![0u8; nq];
    let next_q = AtomicUsize::new(0);

    let slots_send = SendPtr(slots.as_mut_ptr());
    let lens_send = SendPtr(lens.as_mut_ptr());

    let d_send = SendPtr(d_data);

    thread::scope(|s| {
        let ix_ref = &ix;
        let terms_ref = &terms;
        let queries_ref = &queries;
        let next_ref = &next_q;

        for _ in 0..NTHREADS {
            let s_ptr = slots_send;
            let l_ptr = lens_send;

            s.spawn(move || {
                let mut solver = Solver::new(ix_ref, terms_ref, d_send.as_ptr());
                loop {
                    let start = next_ref.fetch_add(64, Ordering::Relaxed);
                    if start >= nq {
                        break;
                    }
                    let end = (start + 64).min(nq);
                    for j in start..end {
                        unsafe {
                            let slot_slice = std::slice::from_raw_parts_mut(
                                s_ptr.as_ptr().cast_mut().add(j * SLOT),
                                SLOT,
                            );
                            let written = solver.answer(&queries_ref[j], slot_slice);
                            *l_ptr.as_ptr().cast_mut().add(j) = written as u8;
                        }
                    }
                }
            });
        }
    });

    let mut out_file = File::create(&out_path)?;
    let mut out_buf = Vec::with_capacity(nq * 32);
    for i in 0..nq {
        let len = lens[i] as usize;
        let start = i * SLOT;
        out_buf.extend_from_slice(&slots[start..start + len]);
    }
    out_file.write_all(&out_buf)?;
    out_file.flush()?;

    unsafe {
        _exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offset_case(documents: u32, threads: usize, expect_compact: bool) {
        let mut corpus = String::new();
        for doc in 0..documents {
            let text = if doc == 0 || doc == documents - 1 {
                "needle"
            } else {
                "other"
            };
            corpus.push_str(&format!("{doc}\t{text}\n"));
        }
        let size = corpus.len();
        let mut bytes = corpus.into_bytes();
        bytes.extend_from_slice(&[0; 128]);
        let mut query = b"BOOL needle\n".to_vec();
        let qsize = query.len();
        query.extend_from_slice(&[0; 128]);
        let mut terms = TermTable::new();
        let mut queries = parse_queries(query.as_ptr(), qsize, &mut terms);
        let ix = build_index(bytes.as_ptr(), size, &mut terms, &mut queries, threads);
        assert_eq!(ix.wide_post.is_empty(), expect_compact);
        let mut solver = Solver::new(&ix, &terms, bytes.as_ptr());
        let mut out = [0; 128];
        let len = solver.answer(&queries[0], &mut out);
        let expected = format!("2 {} 0 {}\n", documents - 1, documents - 1);
        assert_eq!(&out[..len], expected.as_bytes());
    }

    #[test]
    fn compact_offsets_cover_u16_max_and_wide_fallback() {
        offset_case(65536, 1, true);
        offset_case(65537, 1, false);
        offset_case(65537, 12, true);
    }

    #[test]
    fn empty_worker_segments_preserve_posting_order() {
        offset_case(2, 12, true);
    }
}
