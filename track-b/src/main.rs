//! Team Lex Rust engine: exact inline-prefix dictionary and selective scan improvements.
//! Speed Demon 2026, Track B (Index), in Rust with no dependencies.
//!
//! 1. The queries are read first, and only their terms are looked for.
//! 2. Each thread samples the start of its share of docs.txt, counting the
//!    documents that hold each term: a BOOL term in over 1/32 of them is kept
//!    as a bitmap, and each phrase's rarest term becomes its anchor.
//! 3. The full scan buffers token IDs per document and checks phrases at their
//!    anchors using exact ID slices. A BOOL term sets its bitmap bit or
//!    files the pair under the term's bucket; the buckets are then scattered
//!    into one stretch of 16-bit doc offsets per term.
//! 4. BOOL queries run left to right on sorted lists, or on a bitmap once a
//!    bitmap term starts the query or is OR-ed in.
use std::arch::x86_64::*;
use std::borrow::Cow;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicU16, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Barrier, Mutex, Once, OnceLock};

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
    fn madvise(addr: *mut u8, len: usize, advice: i32) -> i32;
    fn mallopt(param: i32, value: i32) -> i32;
    fn prctl(option: i32, ...) -> i32;
    fn _exit(status: i32) -> !;
}

const THREADS: usize = 12;
const NONE: u32 = u32::MAX;
const BUCKETS: usize = 32; // scan output streams per thread; term i goes to bucket i % BUCKETS
const GAP: u32 = 15; // a unit's 4-bit doc gap; GAP means the whole gap follows in the next unit
const LAG: usize = 8;
type Token = (__m128i, usize, usize, usize);
const DOC_BITS: u32 = 15; // a thread's share holds under 2^15 documents

/// The file, mapped over a spare zero page so loads of 32 bytes past any of
/// its bytes stay inside the mapping.
fn map(path: &str) -> &'static [u8] {
    let file = std::fs::File::open(path).expect(path);
    let len = file.metadata().unwrap().len() as usize;
    unsafe {
        let p = mmap(std::ptr::null_mut(), len + 4096, 1, 0x22, -1, 0); // PROT_READ, MAP_PRIVATE | MAP_ANONYMOUS
        let ok = len == 0 || mmap(p, len, 1, 0x12, file.as_raw_fd(), 0) == p; // MAP_PRIVATE | MAP_FIXED
        assert!(p as isize != -1 && ok, "mmap {path}");
        std::slice::from_raw_parts(p, len)
    }
}

type Job = &'static (dyn Fn(usize) + Sync);
static POOL: OnceLock<(Mutex<Option<Job>>, Barrier)> = OnceLock::new();
static STARTED: Once = Once::new();

/// Runs f(t) on every thread and collects what each returns. The threads are
/// started once and reused by every phase: a new thread's stack is a new
/// mapping, which waits behind every page fault in flight.
fn on_threads<R: Send>(f: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let (job, gate) = POOL.get_or_init(|| (Mutex::new(None), Barrier::new(THREADS)));
    STARTED.call_once(|| {
        for t in 1..THREADS {
            std::thread::spawn(move || loop {
                let (job, gate) = POOL.get().unwrap();
                gate.wait();
                {
                    let f = job.lock().unwrap().unwrap();
                    f(t);
                }
                gate.wait();
            });
        }
    });
    let out: Vec<Mutex<Option<R>>> = (0..THREADS).map(|_| Mutex::new(None)).collect();
    struct AbortOnPanic;
    impl Drop for AbortOnPanic {
        fn drop(&mut self) {
            if std::thread::panicking() {
                std::process::abort();
            }
        }
    }
    let run = |t: usize| {
        let _guard = AbortOnPanic;
        *out[t].lock().unwrap() = Some(f(t));
    };
    let run: &(dyn Fn(usize) + Sync) = &run;
    // Safe while `run` outlives the phase, and the second wait ensures that.
    *job.lock().unwrap() = Some(unsafe { std::mem::transmute::<_, Job>(run) });
    gate.wait();
    run(0);
    gate.wait();
    *job.lock().unwrap() = None;
    std::hint::black_box(run); // keep the scoped job alive through the completion barrier
    let take = |r: Mutex<Option<R>>| r.into_inner().unwrap().unwrap();
    out.into_iter().map(take).collect()
}

/// The decimal number at text[i..].
fn number(text: &[u8], i: usize) -> u32 {
    let digits = text[i..].iter().take_while(|b| b.is_ascii_digit());
    digits.fold(0, |v, &b| v * 10 + (b - b'0') as u32)
}

/// Token bytes: [A-Za-z0-9].
static WORD: [bool; 256] = {
    let mut t = [false; 256];
    let mut c = 0;
    while c < 256 {
        (t[c], c) = ((c as u8).is_ascii_alphanumeric(), c + 1);
    }
    t
};

/// The key of the token of len (at most 32) bytes at p: its bytes lowercased
/// (OR-ing 0x20 lowercases letters and keeps digits) and zero padded.
unsafe fn key(p: *const u8, len: usize) -> __m256i {
    let order = _mm256_setr_epi8(
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    );
    _mm256_and_si256(
        _mm256_or_si256(_mm256_loadu_si256(p.cast()), _mm256_set1_epi8(0x20)),
        _mm256_cmpgt_epi8(_mm256_set1_epi8(len as i8), order),
    )
}

unsafe fn prefix(p: *const u8, len: usize) -> __m128i {
    static MASK: [u8; 32] = [
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    _mm_and_si128(
        _mm_or_si128(_mm_loadu_si128(p.cast()), _mm_set1_epi8(0x20)),
        _mm_loadu_si128(MASK.as_ptr().add(16 - len.min(12)).cast()),
    )
}
fn prefix_hash(k: __m128i) -> usize {
    let words: [u64; 2] = unsafe { std::mem::transmute(k) };
    unsafe { _mm_crc32_u64(words[1], words[0]) }.wrapping_mul(0x9e3779b97f4a7c15) as usize
}

fn same(a: __m256i, b: __m256i) -> bool {
    unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi8(a, b)) == -1 }
}

/// A SIMD word holds twelve exact bytes and an ID. Long entries use an
/// eight-byte prefix and full-key index. Every hash/prefix collision is checked.
struct Entries(*mut __m128i, usize);
unsafe impl Send for Entries {}
unsafe impl Sync for Entries {}
impl Entries {
    fn new(len: usize) -> Self {
        let bytes = (len * 16 + 4095) & !4095;
        let align = 2 << 20;
        unsafe {
            prctl(41, 0u64, 0u64, 0u64, 0u64);
            let raw = mmap(std::ptr::null_mut(), bytes + align, 3, 0x22, -1, 0);
            assert!(raw as isize != -1);
            let address = (raw as usize + align - 1) & !(align - 1);
            let before = address - raw as usize;
            if before > 0 {
                munmap(raw, before);
            }
            munmap((address + bytes) as *mut u8, align - before);
            let ptr = address as *mut __m128i;
            madvise(ptr.cast(), bytes, 14);
            madvise(ptr.cast(), bytes, 23);
            prctl(41, 1u64, 0u64, 0u64, 0u64);
            Self(ptr, len)
        }
    }
}
impl std::ops::Deref for Entries {
    type Target = [__m128i];
    fn deref(&self) -> &[__m128i] {
        unsafe { std::slice::from_raw_parts(self.0, self.1) }
    }
}
impl std::ops::DerefMut for Entries {
    fn deref_mut(&mut self) -> &mut [__m128i] {
        unsafe { std::slice::from_raw_parts_mut(self.0, self.1) }
    }
}
impl Drop for Entries {
    fn drop(&mut self) {
        unsafe {
            munmap(self.0.cast(), self.1 * 16);
        }
    }
}

struct Terms {
    entries: Entries,
    long: Vec<__m256i>,
    n: usize,
}
impl Terms {
    fn find(&self, t: Token) -> u32 {
        let len = t.3;
        if len > 32 {
            return self.n as u32;
        }
        let at = self.locate(t);
        ((unsafe { _mm_extract_epi32::<3>(self.entries[at]) as u32 } & 0x7fffffff).wrapping_sub(1))
            .min(self.n as u32)
    }
    fn locate(&self, t: Token) -> usize {
        let (k, h, p, len) = t;
        let mut i = h & (self.entries.len() - 2);
        unsafe {
            let wanted = _mm256_broadcastsi128_si256(k);
            loop {
                let v = _mm256_load_si256(self.entries.as_ptr().add(i).cast());
                let m =
                    _mm256_movemask_ps(_mm256_castsi256_ps(_mm256_cmpeq_epi32(v, wanted))) as u32;
                let longmask = (_mm256_movemask_ps(_mm256_castsi256_ps(v)) as u32) >> 3;
                let mut candidates = ((m & (m >> 1) & ((m >> 2) | longmask)) | (m >> 3)) & 0x11;
                while candidates != 0 {
                    let at = i + (candidates.trailing_zeros() / 4) as usize;
                    let e = *self.entries.get_unchecked(at);
                    let id = _mm_extract_epi32::<3>(e) as u32;
                    if id == 0 {
                        return at;
                    }
                    if if len <= 12 {
                        id >> 31 == 0
                    } else {
                        id >> 31 != 0
                            && same(
                                *self.long.get_unchecked(_mm_extract_epi32::<2>(e) as usize),
                                key(p as *const u8, len),
                            )
                    } {
                        return at;
                    }
                    candidates &= candidates - 1;
                }
                i = (i + 2) & (self.entries.len() - 1);
            }
        }
    }
    fn renumber(&mut self, new: &[u32]) {
        unsafe {
            for e in self.entries.iter_mut() {
                let id = _mm_extract_epi32::<3>(*e) as u32;
                if id != 0 {
                    *e = _mm_insert_epi32::<3>(
                        *e,
                        ((id & 0x80000000) | (new[(id & 0x7fffffff) as usize - 1] + 1)) as i32,
                    );
                }
            }
        }
    }

    fn insert(&mut self, term: &[u8]) -> u32 {
        assert!(!term.is_empty() && term.len() <= 32);
        let k = unsafe { prefix(term.as_ptr(), term.len()) };
        let h = prefix_hash(k);
        let at = self.locate((k, h, term.as_ptr() as usize, term.len()));
        let old = unsafe { _mm_extract_epi32::<3>(self.entries[at]) as u32 };
        if old != 0 {
            return (old & 0x7fffffff) - 1;
        }
        let (k, flag) = if term.len() > 12 {
            let full = unsafe { key(term.as_ptr(), term.len()) };
            let k = unsafe { _mm_insert_epi32::<2>(k, self.long.len() as i32) };
            self.long.push(full);
            (k, 1u32 << 31)
        } else {
            (k, 0)
        };
        self.entries[at] = unsafe { _mm_insert_epi32::<3>(k, (flag | (self.n as u32 + 1)) as i32) };
        self.n += 1;
        self.n as u32 - 1
    }
}

/// Bits of the 64 bytes at p: token bytes ([A-Za-z0-9]; after OR-ing 0x20 a
/// letter lies in 'a'..='z', and a digit lies in '0'..='9' as is), and LFs.
unsafe fn classes(p: *const u8) -> (u64, u64) {
    let half = |q: *const u8| {
        let v = _mm256_loadu_si256(q.cast());
        let c = |b: u8| _mm256_set1_epi8(b as i8);
        let at_most = |x: __m256i, n: u8| _mm256_cmpeq_epi8(_mm256_subs_epu8(x, c(n)), c(0));
        let letter = _mm256_sub_epi8(_mm256_or_si256(v, c(0x20)), c(b'a'));
        let word = _mm256_or_si256(at_most(letter, 25), at_most(_mm256_sub_epi8(v, c(b'0')), 9));
        let mask = |x: __m256i| _mm256_movemask_epi8(x) as u32 as u64;
        (mask(word), mask(_mm256_cmpeq_epi8(v, c(b'\n'))))
    };
    let ((w0, n0), (w1, n1)) = (half(p), half(p.add(32)));
    (w0 | w1 << 32, n0 | n1 << 32)
}

/// Appends the id of each token of the document text from p on to ids, and
/// returns where the next line starts. Token bounds come from one class mask
/// per 64-byte window; a token that runs into the window's end starts the next
/// window. Each token's slot is prefetched and the token looked up LAG tokens
/// later, so the misses overlap.
unsafe fn tokens(mut p: *const u8, t: &Terms, ids: &mut Vec<u32>, pre: *const u16) -> *const u8 {
    let mut ring = [(_mm_setzero_si128(), 0usize, 0usize, 33usize); LAG];
    let (mut n, mut out) = (0usize, ids.len());
    loop {
        // A window adds at most 32 ids, and the document's end LAG more.
        (ids.set_len(out), ids.reserve(32 + LAG));
        let buf = ids.as_mut_ptr();
        let mut look = |r: &Token| {
            let id = t.find(*r);
            _mm_prefetch::<_MM_HINT_T0>(pre.wrapping_add(id as usize).cast()); // its state, for the caller
            (*buf.add(out), out) = (id, out + 1);
        };
        _mm_prefetch::<_MM_HINT_T0>(p.wrapping_add(2048).cast());
        let (word, nl) = classes(p);
        let stop = if nl != 0 { nl.trailing_zeros() } else { 64 }; // the window's bytes in this document
        let mut m = word & ((1u128 << stop) - 1) as u64;
        let mut next = 64;
        while m != 0 {
            let (s, e) = (m.trailing_zeros(), m.wrapping_add(m & m.wrapping_neg()));
            let mut len = e.trailing_zeros() - s;
            if e == 0 && nl == 0 {
                if s != 0 {
                    next = s; // the token may go on: start the next window at it
                    break;
                }
                while WORD[*p.add(next as usize) as usize] {
                    next += 1; // 64 bytes or more
                }
                len = next;
            }
            m &= e;
            let ptr = p.add(s as usize) as usize;
            let k = prefix(ptr as *const u8, len as usize);
            let h = prefix_hash(k);
            _mm_prefetch::<_MM_HINT_T0>(t.entries.as_ptr().add(h & (t.entries.len() - 2)).cast());
            look(&ring[n % LAG]);
            (ring[n % LAG], n) = ((k, h, ptr, len as usize), n + 1);
        }
        if nl != 0 {
            (n..n + LAG).for_each(|i| look(&ring[i % LAG]));
            ids.set_len(out);
            return p.add(stop as usize + 1);
        }
        p = p.add(next as usize);
    }
}

/// A set of documents: a sorted list, or a bitmap. A term's bitmap is
/// borrowed, and copied only when an operator first writes to it.
enum Docs<'a> {
    List(Vec<u32>),
    Bits(Cow<'a, [u64]>),
}

fn has(bits: &[u64], d: u32) -> bool {
    bits[d as usize / 64] >> (d % 64) & 1 != 0
}

/// Above this many documents, a list meets another list as a bitmap: setting
/// and testing bits in 31 KB beats a merge's unpredictable steps.
const BIG: usize = 768;

/// cur op term, strictly left to right; a list OR a bitmap becomes a bitmap.
/// A bitmap has words words.
fn apply<'a>(cur: Docs<'a>, op: u8, term: Docs<'a>, words: usize) -> Docs<'a> {
    use Docs::*;
    let bit = |d: u32| (d as usize / 64, 1u64 << (d % 64));
    match (cur, op, term) {
        (Bits(mut a), _, Bits(b)) => {
            for (x, &y) in a.to_mut().iter_mut().zip(b.iter()) {
                *x = match op {
                    b'A' => *x & y,
                    b'O' => *x | y,
                    _ => *x & !y,
                };
            }
            Bits(a)
        }
        (Bits(a), b'A', List(mut b)) | (List(mut b), b'A', Bits(a)) => {
            (b.retain(|&d| has(&a, d)), List(b)).1
        }
        (List(mut a), _, Bits(b)) if op == b'N' => (a.retain(|&d| !has(&b, d)), List(a)).1,
        (Bits(mut a), _, List(b)) | (List(b), _, Bits(mut a)) => {
            let x = a.to_mut();
            for (w, m) in b.into_iter().map(bit) {
                x[w] = if op == b'O' { x[w] | m } else { x[w] & !m };
            }
            Bits(a)
        }
        (List(a), _, List(b))
            if (op == b'O' && a.len() + b.len() > 2048)
                || (op != b'O' && a.len() + b.len() > BIG && a.len() * 16 >= b.len()) =>
        {
            let mut bits = vec![0u64; words];
            b.iter()
                .for_each(|&d| bits[d as usize / 64] |= 1 << (d % 64));
            apply(List(a), op, Bits(Cow::Owned(bits)), words)
        }
        (List(a), b'O', List(b)) => {
            let (mut v, mut i, mut j) = (Vec::with_capacity(a.len() + b.len()), 0, 0);
            while i < a.len() && j < b.len() {
                let (x, y) = (a[i], b[j]);
                v.push(x.min(y));
                (i, j) = (i + (x <= y) as usize, j + (y <= x) as usize);
            }
            v.extend_from_slice(&a[i..]);
            v.extend_from_slice(&b[j..]);
            List(v)
        }
        // AND or NOT: a binary search per document when b is far longer.
        (List(mut a), _, List(b)) if a.len() * 16 < b.len() => {
            let mut j = 0;
            a.retain(|&d| {
                j += b[j..].partition_point(|&x| x < d);
                (j < b.len() && b[j] == d) == (op == b'A')
            });
            List(a)
        }
        // Otherwise a merge without branches: x stays when it is in b (AND)
        // or below b's next document (NOT), and a's rest stays for NOT.
        (List(mut a), _, List(b)) => {
            let (mut i, mut j, mut k) = (0, 0, 0);
            while i < a.len() && j < b.len() {
                let (x, y) = (a[i], b[j]);
                a[k] = x;
                k += if op == b'A' { x == y } else { x < y } as usize;
                (i, j) = (i + (x <= y) as usize, j + (y <= x) as usize);
            }
            let rest = if op == b'A' { 0 } else { a.len() - i };
            a.copy_within(i..i + rest, k);
            a.truncate(k + rest);
            List(a)
        }
    }
}

/// The number of set bits in b (whole 4-word vectors) and the sum of their
/// positions, with AVX2: popcounts by nibble lookup summed per word, and bit
/// k of the positions counted from the bits under a mask.
unsafe fn bit_sums(b: &[u64]) -> (u64, u64) {
    const NIB: [u8; 16] = [0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4]; // bits set in 0..16
    let nib = _mm256_broadcastsi128_si256(_mm_loadu_si128(NIB.as_ptr().cast()));
    let (low, z) = (_mm256_set1_epi8(0x0F), _mm256_setzero_si256());
    let h = |v: __m256i| _mm256_shuffle_epi8(nib, _mm256_and_si256(v, low));
    let pop = |v: __m256i| _mm256_sad_epu8(_mm256_add_epi8(h(v), h(_mm256_srli_epi16::<4>(v))), z);
    // The bits whose position has bit k set: 0xAAAA.., 0xCCCC.., ..., 0xFFFF_FFFF_0000_0000.
    let sel = |k: usize| _mm256_set1_epi64x(!(u64::MAX / ((1u64 << (1 << k)) + 1)) as i64);
    let (mut acc, mut word) = ([z; 8], _mm256_setr_epi64x(0, 1, 2, 3));
    for q in b.chunks_exact(4) {
        let v = _mm256_loadu_si256(q.as_ptr().cast());
        let c = pop(v);
        acc[6] = _mm256_add_epi64(acc[6], c);
        acc[7] = _mm256_add_epi64(acc[7], _mm256_mul_epu32(c, word)); // weighted by word index
        for k in 0..6 {
            acc[k] = _mm256_add_epi64(acc[k], pop(_mm256_and_si256(v, sel(k))));
        }
        word = _mm256_add_epi64(word, _mm256_set1_epi64x(4));
    }
    let total = |k: usize| -> u64 { std::mem::transmute::<_, [u64; 4]>(acc[k]).iter().sum() };
    let sum = 64 * total(7) + (0..6).map(|k| total(k) << k).sum::<u64>();
    (total(6), sum)
}

/// Append a decimal integer without allocating a temporary string.
fn num(out: &mut String, space: bool, mut v: u64) {
    let (mut buf, mut i) = ([b' '; 21], 21);
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.push_str(unsafe { std::str::from_utf8_unchecked(&buf[i - space as usize..]) });
}

fn line(out: &mut String, docs: &Docs) {
    match docs {
        Docs::List(v) => {
            let sum: u64 = v.iter().map(|&d| d as u64).sum();
            (num(out, false, v.len() as u64), num(out, true, sum));
            v.iter().take(10).for_each(|&d| num(out, true, d as u64));
        }
        Docs::Bits(b) => {
            let (count, sum) = unsafe { bit_sums(b) };
            (num(out, false, count), num(out, true, sum));
            let mut rem = 10;
            for (w, &x) in b.iter().enumerate() {
                let mut m = x;
                while m != 0 && rem > 0 {
                    num(out, true, w as u64 * 64 + m.trailing_zeros() as u64);
                    (m, rem) = (m & (m - 1), rem - 1);
                }
                if rem == 0 {
                    break;
                }
            }
        }
    }
    out.push('\n');
}

/// Calls f(doc, ids) with the token ids of each line in text, the first
/// being document first, with pre[id] prefetched for each; returns the
/// number of documents.
fn walk(
    text: &[u8],
    first: u32,
    t: &Terms,
    pre: *const u16,
    mut f: impl FnMut(u32, &[u32]),
) -> u32 {
    let (mut p, end) = (text.as_ptr(), text.as_ptr_range().end);
    let (mut doc, mut ids) = (first, Vec::with_capacity(1 << 12));
    while p < end {
        unsafe {
            while *p != b'\t' {
                p = p.add(1);
            }
            ids.clear();
            p = tokens(p, t, &mut ids, pre);
        }
        f(doc, &ids[LAG..]);
        doc += 1;
    }
    doc - first
}

/// Scan output for a bucket of terms, in 64 KB chunks so that only the last
/// one is ever part empty. A unit is a term's index in the bucket (i / BUCKETS)
/// above the gap since the bucket's previous document; a thread meets its
/// documents in order, so the gaps are small.
#[derive(Default)]
struct Units {
    chunks: Vec<Vec<u16>>,
    at: usize, // the address of the next unit's place in the last chunk, up to end
    end: usize,
    prev: u32,
}

impl Units {
    fn put(&mut self, x: u16) {
        if self.at == self.end {
            self.grow();
        }
        unsafe { *(self.at as *mut u16) = x };
        self.at += 2;
    }

    #[cold]
    #[inline(never)]
    fn grow(&mut self) {
        self.finish();
        let c = Vec::<u16>::with_capacity(1 << 15);
        (self.at, self.end) = (c.as_ptr() as usize, c.as_ptr() as usize + 2 * c.capacity());
        self.chunks.push(c);
    }

    fn add(&mut self, offset: u32, doc: u32) {
        let gap = std::mem::replace(&mut self.prev, doc);
        let gap = doc - gap;
        self.put((offset << 4 | gap.min(GAP)) as u16);
        if gap >= GAP {
            self.put(gap as u16);
        }
    }

    /// Sets the last chunk's length to the units written.
    fn finish(&mut self) {
        if let Some(c) = self.chunks.last_mut() {
            unsafe { c.set_len((self.at - c.as_ptr() as usize) / 2) };
        }
    }
}

fn main() {
    unsafe { mallopt(-3, 1 << 16) }; // M_MMAP_THRESHOLD: freed buffers go back to the kernel
                                     // PR_SET_THP_DISABLE: the box backs part-written regions with whole 2 MB pages.
    unsafe { prctl(41, 1u64, 0u64, 0u64, 0u64) };
    let args: Vec<String> = std::env::args().collect();
    let path = |flag, env, default: &str| {
        let arg = args.windows(2).rev().find(|a| a[0] == flag);
        std::env::var(env)
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| arg.map(|a| a[1].clone()))
            .unwrap_or_else(|| default.to_owned())
    };
    let data_dir = path("--data", "DATA_DIR", "/data");
    let out_dir = path("--out", "OUT_DIR", "/out");

    let qtext = map(&format!("{data_dir}/queries.txt"));
    // After a space, lowercase letters, digits and quotes have bit 0x20;
    // uppercase Boolean operators do not. Leave an empty slot even for tiny input.
    let term_count = qtext
        .windows(2)
        .filter(|p| p[0] == b' ' && p[1] & 32 != 0)
        .count();
    let entries = Entries::new((term_count + 1).max(8).next_power_of_two());
    let mut terms = Terms {
        entries,
        long: Vec::new(),
        n: 0,
    };
    let (mut spans, mut ids, mut ops) = (vec![], vec![], vec![]);
    for q in qtext.split(|&b| b == b'\n').filter(|q| !q.is_empty()) {
        let (phrase, from) = (q[0] == b'P', ids.len() as u32);
        let body = &q[if phrase { 8 } else { 5 }..q.len() - phrase as usize];
        for (k, word) in body.split(|&b| b == b' ').enumerate() {
            match !phrase && k % 2 == 1 {
                true => *ops.last_mut().unwrap() = word[0],
                false => (ids.push(terms.insert(word)), ops.push(0)).1,
            }
        }
        spans.push((phrase, from, ids.len() as u32));
    }
    let nterms = terms.n;
    assert!(nterms <= BUCKETS << 12, "too many terms"); // a unit's index has 12 bits
    let mut in_bool = vec![false; nterms];
    for &(_, from, to) in spans.iter().filter(|s| !s.0) {
        let r = from as usize..to as usize;
        ids[r].iter().for_each(|&id| in_bool[id as usize] = true);
    }

    let text = map(&format!("{data_dir}/docs.txt"));
    let mut cut: Vec<usize> = (0..=THREADS).map(|t| text.len() * t / THREADS).collect();
    for c in &mut cut[1..THREADS] {
        while *c > 0 && *c < text.len() && text[*c - 1] != b'\n' {
            *c += 1;
        }
    }
    let mut last_line = text.len().saturating_sub(1);
    while last_line > 0 && text[last_line - 1] != b'\n' {
        last_line -= 1;
    }
    let ndocs = (!text.is_empty()) as usize * (number(text, last_line) as usize + 1);
    let first_doc = |c: usize| text.get(c).map_or(ndocs as u32, |_| number(text, c));
    let firsts: Vec<u32> = cut[..THREADS].iter().map(|&c| first_doc(c)).collect();

    // The sample: documents per term in the first 1/64 of each share.
    let samples = on_threads(|t| {
        let (mut df, mut last) = (vec![0u32; nterms + 1], vec![NONE; nterms + 1]);
        let sample = &text[cut[t]..cut[t] + (cut[t + 1] - cut[t]) / 64];
        let n = walk(sample, firsts[t], &terms, std::ptr::null(), |doc, toks| {
            for &id in toks {
                if std::mem::replace(&mut last[id as usize], doc) != doc {
                    df[id as usize] += 1;
                }
            }
        });
        (n as usize, df)
    });
    let sampled: usize = samples.iter().map(|s| s.0).sum();
    let mut df = vec![0u32; nterms + 1];
    for (_, d) in samples {
        df.iter_mut().zip(d).for_each(|(a, b)| *a += b);
    }
    // Each phrase's anchor, its rarest term: (anchor, query, position).
    let (mut anchored, mut anchor) = (vec![], vec![false; nterms]);
    for (q, &(_, from, to)) in spans.iter().enumerate().filter(|s| s.1 .0) {
        let rarity = |j: &u32| df[ids[*j as usize] as usize];
        let j = (from..to).min_by_key(rarity).unwrap();
        anchored.push((ids[j as usize], q as u32, j - from));
        anchor[ids[j as usize] as usize] = true;
    }
    // Terms renumbered by class, so the scan reads a token's role off its id:
    // bitmaps, lists, lists that anchor phrases, anchors only, and terms only
    // phrases read (their tokens need no state). Within a class the most
    // frequent come first, so the state of common terms shares cache lines.
    let class = |i: usize| match (in_bool[i], anchor[i]) {
        (true, false) => (df[i] as usize * 32 <= sampled) as usize,
        (true, true) => 2,
        (false, true) => 3,
        (false, false) => 4,
    };
    // One sort of (class, rarity, id) packed in a u64: the key is computed
    // once per term instead of twice per comparison.
    let pack = |i: usize| (class(i) as u64) << 56 | ((u32::MAX - df[i]) as u64) << 24 | i as u64;
    let mut packed: Vec<u64> = (0..nterms).map(pack).collect();
    packed.sort_unstable();
    let order: Vec<usize> = packed.iter().map(|&k| k as usize & 0xFF_FFFF).collect();
    let [nbits, a0, lists, nkeep] = [0, 1, 2, 3].map(|c| order.partition_point(|&i| class(i) <= c));
    let mut new = vec![0; nterms];
    for (r, &i) in order.iter().enumerate() {
        new[i] = r as u32;
    }
    terms.renumber(&new);
    ids.iter_mut().for_each(|id| *id = new[*id as usize]);
    anchored.iter_mut().for_each(|a| a.0 = new[a.0 as usize]);
    anchored.sort_unstable();
    // Where the phrases of anchor a0 + k start in anchored.
    let first_of = |i: usize| anchored.partition_point(|a| (a.0 as usize) < i);
    let astart: Vec<usize> = (a0..=nkeep).map(first_of).collect();
    let words = ndocs.div_ceil(256) * 4; // whole 4-word vectors, for bit_sums
                                         // Zero pages from the kernel, written only where a bit is set.
    let bits: Vec<AtomicU64> = unsafe { std::mem::transmute(vec![0u64; nbits * words]) };

    drop((df, in_bool, anchor, order, packed, new));

    // The scan: phrase matches, bitmap bits, and list postings by bucket.
    let found = Mutex::new(vec![]); // (query, document) phrase matches
    let scans: Vec<(Vec<u16>, Mutex<Vec<Units>>)> = on_threads(|t| {
        // Per term (and one id for every token that needs nothing): documents
        // in this share, and the last one's offset + 1.
        let (mut count, mut last, mut mine) =
            (vec![0u16; nkeep + 1], vec![0u16; nkeep + 1], vec![]);
        // Per document: the list terms new to it, and where the phrase anchors are.
        let (mut kept, mut anchors) = (vec![], vec![]);
        let mut buckets: Vec<Units> = (0..BUCKETS).map(|_| Units::default()).collect();
        let first = firsts[t];
        // Each bitmap term's bits for the current 64 documents (one more word
        // takes the bits of every other term), ORed in when the block ends.
        let (mut pending, mut block) = (vec![0u64; nbits + 1], first as usize / 64);
        let flush = |pending: &mut [u64], w: usize| {
            for (b, x) in pending[..nbits].iter_mut().enumerate() {
                if *x != 0 {
                    bits[b * words + w].fetch_or(std::mem::take(x), Relaxed);
                }
            }
        };
        // Every token does the same work, kept or not by masks, so whether a
        // term is new to the document, a list or a bitmap costs no branch.
        let (share, pre) = (&text[cut[t]..cut[t + 1]], last.as_ptr());
        let n = walk(share, first, &terms, pre, |doc, toks| {
            if doc as usize / 64 != block {
                let w = std::mem::replace(&mut block, doc as usize / 64);
                flush(&mut pending, w);
            }
            let (mark, bit) = ((doc - first + 1) as u16, 1u64 << (doc % 64));
            // Read once per document: Units::put stores through a raw pointer,
            // after which captured values would be loaded again for every token.
            let (nkeep, a0, nbits, lists, off) = (nkeep, a0, nbits, lists, doc - first);
            let (last, pending) = (&mut last[..=nkeep], &mut pending[..=nbits]);
            // Retain initialized slots across shorter documents instead of zeroing them again.
            kept.resize(kept.len().max(toks.len()), 0);
            anchors.resize(anchors.len().max(toks.len()), 0);
            let (mut n, mut na) = (0, 0);
            for (at, &id) in toks.iter().enumerate() {
                let i = (id as usize).min(nkeep);
                // i <= nkeep, n and na <= at, and the slices hold nkeep + 1, nbits + 1 and toks.len().
                unsafe {
                    *anchors.get_unchecked_mut(na) = at;
                    na += (i.wrapping_sub(a0) < nkeep - a0) as usize;
                    let new = std::mem::replace(last.get_unchecked_mut(i), mark) != mark;
                    *pending.get_unchecked_mut(i.min(nbits)) |= bit;
                    *kept.get_unchecked_mut(n) = i as u32;
                    n += (new & (i.wrapping_sub(nbits) < lists - nbits)) as usize;
                }
            }
            for &at in &anchors[..na] {
                let k = (toks[at] as usize).min(nkeep) - a0;
                for &(_, q, j) in &anchored[astart[k]..astart[k + 1]] {
                    let span = spans[q as usize];
                    let phrase = &ids[span.1 as usize..span.2 as usize];
                    // Do the tokens around this one spell the phrase? (A start before
                    // the document wraps around, and the slice is then out of range.)
                    let s = at.wrapping_sub(j as usize);
                    let edge = if j == 0 { phrase.len() - 1 } else { 0 };
                    if toks.get(s.wrapping_add(edge)) == phrase.get(edge)
                        && toks.get(s..s.wrapping_add(phrase.len())) == Some(phrase)
                    {
                        mine.push((q, doc));
                    }
                }
            }
            for &i in &kept[..n] {
                buckets[i as usize % BUCKETS].add(i / BUCKETS as u32, off);
                count[i as usize] += 1;
            }
        });
        flush(&mut pending, block);
        // Drop finished interior file pages; preserve the shared first page and VMA.
        let start = (share.as_ptr() as usize + 4096) & !4095;
        let end = (share.as_ptr() as usize + share.len()) & !4095;
        unsafe {
            madvise(start as *mut u8, end.saturating_sub(start), 4);
        }
        buckets.iter_mut().for_each(Units::finish);
        assert!(n < 1 << DOC_BITS, "too many documents per thread");
        found.lock().unwrap().extend(mine);
        (count, Mutex::new(buckets))
    });
    drop((terms, anchored, astart)); // all scan workers have finished
                                     // Each term's postings lie in one stretch, thread by thread: the segment
                                     // from thread t holds seg[i * THREADS + t] offsets from firsts[t]. The
                                     // stretches go bucket by bucket, so a bucket's scatter writes one region,
                                     // and frequency ranks dealt round-robin keep the regions small.
    let (mut seg, mut start, mut end) = (vec![0u16; lists * THREADS], vec![0u32; lists], 0);
    for i in (0..BUCKETS).flat_map(|b| (b..lists).step_by(BUCKETS)) {
        let s = &mut seg[i * THREADS..][..THREADS];
        s.iter_mut().zip(&scans).for_each(|(n, c)| *n = c.0[i]);
        (start[i], end) = (end, end + s.iter().map(|&n| n as u32).sum::<u32>());
    }
    let all_buckets: Vec<_> = scans.into_iter().map(|s| s.1).collect();
    // Written by every thread at its own positions; relaxed stores are plain stores.
    let post: Vec<AtomicU16> = unsafe { std::mem::transmute(vec![0u16; end as usize]) };
    on_threads(|t| {
        let buckets = std::mem::take(&mut *all_buckets[t].lock().unwrap());
        let mut cursor = vec![0u32; lists.div_ceil(BUCKETS)];
        for (b, units) in buckets.into_iter().enumerate() {
            for (c, i) in cursor.iter_mut().zip((b..lists).step_by(BUCKETS)) {
                let before: u32 = seg[i * THREADS..][..t].iter().map(|&n| n as u32).sum();
                *c = start[i] + before;
            }
            let (mut doc, mut term) = (0u32, NONE);
            for x in units.chunks.into_iter().flatten() {
                if term == NONE {
                    term = (x >> 4) as u32;
                    if x as u32 & GAP == GAP {
                        continue; // the whole gap follows
                    }
                    doc += x as u32 & GAP;
                } else {
                    doc += x as u32;
                }
                let c = &mut cursor[std::mem::replace(&mut term, NONE) as usize];
                (post[*c as usize].store(doc as u16, Relaxed), *c += 1);
            }
        }
    });
    drop(all_buckets);
    let post: Vec<u16> = unsafe { std::mem::transmute(post) };
    let bits: Vec<u64> = unsafe { std::mem::transmute(bits) };
    let mut found = found.into_inner().unwrap();
    found.sort_unstable();
    found.dedup();

    // Answers, 64 queries at a time on every thread. Large lists come and go
    // here, so they now reuse heap memory instead of a fresh mapping each.
    unsafe { mallopt(-3, 64 << 20) };
    let docs = |id: u32| match id as usize {
        i if i < nbits => Docs::Bits(Cow::Borrowed(&bits[i * words..(i + 1) * words])),
        i => {
            let mut at = start[i] as usize;
            let segs = &seg[i * THREADS..][..THREADS];
            let mut v = Vec::with_capacity(segs.iter().map(|&n| n as usize).sum());
            for (&n, &first) in segs.iter().zip(&firsts) {
                v.extend(post[at..at + n as usize].iter().map(|&d| first + d as u32));
                at += n as usize;
            }
            Docs::List(v)
        }
    };
    let n = spans.len().div_ceil(64);
    let chunks: Vec<Mutex<String>> = (0..n).map(|_| Mutex::default()).collect();
    let next = AtomicUsize::new(0);
    on_threads(|_| loop {
        let k = next.fetch_add(1, Relaxed);
        let Some(chunk) = chunks.get(k) else { return };
        let mut out = String::new();
        for (q, &(phrase, from, to)) in spans.iter().enumerate().skip(k * 64).take(64) {
            let r = from as usize..to as usize;
            let (tids, ops) = (&ids[r.clone()], &ops[r]);
            let set = if phrase {
                let at = found.partition_point(|m| m.0 < q as u32);
                let end = at + found[at..].partition_point(|m| m.0 == q as u32);
                Docs::List(found[at..end].iter().map(|m| m.1).collect())
            } else {
                let (cur, steps) = (docs(tids[0]), ops.iter().zip(&tids[1..]));
                steps.fold(cur, |cur, (&op, &id)| match &cur {
                    Docs::List(v) if v.is_empty() && op != b'O' => cur, // nothing left to narrow
                    _ => apply(cur, op, docs(id), words),
                })
            };
            line(&mut out, &set);
        }
        *chunk.lock().unwrap() = out;
    });
    // Chunk by chunk, so the answers are never held twice.
    let file = {
        std::fs::create_dir_all(&out_dir).ok();
        std::fs::File::create(format!("{out_dir}/results.txt")).expect("results.txt")
    };
    let mut out = std::io::BufWriter::new(file);
    for c in chunks {
        std::io::Write::write_all(&mut out, c.into_inner().unwrap().as_bytes()).unwrap();
    }
    std::io::Write::flush(&mut out).unwrap();
    unsafe { _exit(0) };
}
