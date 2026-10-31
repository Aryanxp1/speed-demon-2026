use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::thread;

// POSIX mmap / munmap / prctl bindings
extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
    fn prctl(option: i32, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32;
}

const PROT_READ: i32 = 1;
const MAP_PRIVATE: i32 = 0x02;
const MAP_FAILED: *mut u8 = !0 as *mut u8;
const PR_SET_THP_DISABLE: i32 = 41;

const RING: usize = 4096;
const RING_WORDS: usize = RING / 64;
const ESC_MARKER: u32 = 0xFFFF;

unsafe fn map_file(file: &File) -> (*const u8, usize) {
    let size = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
    if size == 0 {
        return (std::ptr::null(), 0);
    }
    let fd = file.as_raw_fd();
    let p = mmap(std::ptr::null_mut(), size + 4096, PROT_READ, MAP_PRIVATE, fd, 0);
    if p == MAP_FAILED {
        return (std::ptr::null(), 0);
    }
    (p as *const u8, size)
}

#[inline(always)]
unsafe fn parse_u32(p: &mut *const u8) -> u32 {
    let mut v = (**p - b'0') as u32;
    *p = p.add(1);
    while **p >= b'0' {
        v = v * 10 + (**p - b'0') as u32;
        *p = p.add(1);
    }
    v
}

#[derive(Copy, Clone)]
struct Edge {
    v: u32,
    w: u32,
}

#[derive(Copy, Clone)]
struct Esc {
    e: u32,
    v: u32,
    w: u32,
}


#[inline(always)]
fn edge_at(pk: &[u32], esc: &[Esc], u: u32, e: usize) -> (u32, u32) {
    let x = unsafe { *pk.get_unchecked(e) };
    if (x & 0xFFFF) != ESC_MARKER {
        let dv = (x >> 16) as i16 as i32;
        let v = (u as i32 + dv) as u32;
        let w = x & 0xFFFF;
        (v, w)
    } else {
        let idx = esc.binary_search_by_key(&(e as u32), |x| x.e).unwrap_or(0);
        let item = unsafe { esc.get_unchecked(idx) };
        (item.v, item.w)
    }
}

fn dijkstra_fallback(
    pk: &[u32],
    esc: &[Esc],
    off: &[u32],
    s: u32,
    tg: u32,
) -> i64 {
    use std::cmp::Ordering;
    use std::collections::{BinaryHeap, HashMap};

    #[derive(Copy, Clone, Eq, PartialEq)]
    struct State {
        cost: u64,
        node: u32,
    }

    impl Ord for State {
        fn cmp(&self, other: &Self) -> Ordering {
            other.cost.cmp(&self.cost)
        }
    }
    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut dist: HashMap<u32, u64> = HashMap::new();
    let mut heap = BinaryHeap::new();

    dist.insert(s, 0);
    heap.push(State { cost: 0, node: s });

    while let Some(State { cost, node }) = heap.pop() {
        if cost > *dist.get(&node).unwrap_or(&u64::MAX) {
            continue;
        }
        if node == tg {
            return cost as i64;
        }

        let start = off[node as usize] as usize;
        let end = off[node as usize + 1] as usize;
        for e in start..end {
            let (v, w) = edge_at(pk, esc, node, e);
            let next_cost = cost.saturating_add(w as u64);
            let cur = *dist.get(&v).unwrap_or(&u64::MAX);
            if next_cost < cur {
                dist.insert(v, next_cost);
                heap.push(State { cost: next_cost, node: v });
            }
        }
    }

    -1
}

fn dial(
    pk: &[u32],
    esc: &[Esc],
    off: &[u32],
    s: u32,
    tg: u32,
    d32: &mut [u32],
    touched: &mut Vec<u32>,
    ring: &mut [Vec<u32>],
) -> i64 {
    let d32_ptr = d32.as_mut_ptr();
    let ring_ptr = ring.as_mut_ptr();
    let mut bitmask = [0u64; RING_WORDS];
    let mut d = 0u32;
    let mut res = -1i64;
    let mut upper_bound = u64::MAX;

    unsafe {
        *d32_ptr.add(s as usize) = 0;
        touched.push(s);
        (*ring_ptr).push(s);
        bitmask[0] |= 1u64;

        loop {
            if d as u64 >= upper_bound {
                res = upper_bound as i64;
                break;
            }

            let cur_b = (d as usize) & (RING - 1);
            let word = cur_b / 64;
            let bit = cur_b % 64;
            let cur_bits = bitmask[word] & (!0u64 << bit);

            let next_b = if cur_bits != 0 {
                word * 64 + cur_bits.trailing_zeros() as usize
            } else {
                let mut found = None;
                for i in 1..RING_WORDS {
                    let w = (word + i) & (RING_WORDS - 1);
                    let val = bitmask[w];
                    if val != 0 {
                        found = Some(w * 64 + val.trailing_zeros() as usize);
                        break;
                    }
                }
                match found {
                    Some(b) => b,
                    None => break,
                }
            };

            let delta = (next_b + RING - cur_b) & (RING - 1);
            d += delta as u32;

            if d as u64 >= upper_bound {
                res = upper_bound as i64;
                break;
            }

            let bucket = &mut *ring_ptr.add(next_b);
            let mut k = 0;
            while k < bucket.len() && res == -1 {
                let u = *bucket.get_unchecked(k);
                k += 1;
                if *d32_ptr.add(u as usize) != d {
                    continue;
                }
                if u == tg {
                    res = d as i64;
                    break;
                }
                let start = *off.get_unchecked(u as usize) as usize;
                let end = *off.get_unchecked(u as usize + 1) as usize;
                for e in start..end {
                    let (v, w) = edge_at(pk, esc, u, e);
                    let nd = (d as u64) + (w as u64);
                    if nd >= upper_bound {
                        continue;
                    }
                    if w >= RING as u32 || nd >= (u32::MAX - 1) as u64 {
                        res = -2;
                        break;
                    }
                    if v == tg {
                        upper_bound = nd;
                    }
                    let nd32 = nd as u32;
                    let v_idx = v as usize;
                    let cur_dist = *d32_ptr.add(v_idx);
                    if nd32 < cur_dist {
                        if cur_dist == u32::MAX {
                            touched.push(v);
                        }
                        *d32_ptr.add(v_idx) = nd32;
                        let target_b = (nd32 as usize) & (RING - 1);
                        (*ring_ptr.add(target_b)).push(v);
                        bitmask[target_b / 64] |= 1u64 << (target_b % 64);
                    }
                }
            }
            bucket.clear();
            bitmask[next_b / 64] &= !(1u64 << (next_b % 64));

            if res != -1 {
                break;
            }
        }

        for b in ring.iter_mut() {
            b.clear();
        }
        for &u in touched.iter() {
            *d32_ptr.add(u as usize) = u32::MAX;
        }
        touched.clear();
    }

    if res == -2 {
        dijkstra_fallback(pk, esc, off, s, tg)
    } else if res == -1 && upper_bound != u64::MAX {
        upper_bound as i64
    } else {
        res
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        prctl(PR_SET_THP_DISABLE, 1, 0, 0, 0);
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

    let graph_path = data_dir.join("graph.txt");
    let queries_path = data_dir.join("queries.txt");
    let out_path = out_dir.join("distances.txt");

    std::fs::create_dir_all(&out_dir).ok();

    if !graph_path.exists() || !queries_path.exists() {
        File::create(&out_path)?;
        return Ok(());
    }

    let graph_file = File::open(&graph_path)?;
    let queries_file = File::open(&queries_path)?;

    let nthreads = std::env::var("SPEED_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(8)
                .min(8)
                .max(1)
        });


    // 1. Map graph.txt and queries.txt
    let (graph_ptr, gsize) = unsafe { map_file(&graph_file) };
    let (q_ptr, qsize) = unsafe { map_file(&queries_file) };

    if graph_ptr.is_null() || q_ptr.is_null() {
        File::create(&out_path)?;
        return Ok(());
    }

    // Parse queries
    let mut queries = Vec::new();
    unsafe {
        let mut p = q_ptr;
        let end = q_ptr.add(qsize);
        while p < end {
            while p < end && *p <= b' ' {
                p = p.add(1);
            }
            if p >= end {
                break;
            }
            let s = parse_u32(&mut p);
            p = p.add(1);
            let t = parse_u32(&mut p);
            while p < end && *p != b'\n' {
                p = p.add(1);
            }
            if p < end {
                p = p.add(1);
            }
            queries.push((s, t));
        }
        munmap(q_ptr as *mut u8, qsize + 4096);
    }

    // Parse graph header
    let (n, edge_start, edge_len) = unsafe {
        let mut p = graph_ptr;
        let end = graph_ptr.add(gsize);
        while p < end && *p <= b' ' {
            p = p.add(1);
        }
        let n = parse_u32(&mut p);
        p = p.add(1);
        let _m = parse_u32(&mut p);
        while p < end && *p != b'\n' {
            p = p.add(1);
        }
        if p < end {
            p = p.add(1);
        }
        let start = p;
        let len = gsize - (start as usize - graph_ptr as usize);
        (n as usize, start, len)
    };

    // Partition edge slice across threads
    let mut cut = vec![edge_len; nthreads + 1];
    cut[0] = 0;
    for t in 1..nthreads {
        let mut c = edge_len * t / nthreads;
        unsafe {
            while c < edge_len && *edge_start.add(c - 1) != b'\n' {
                c += 1;
            }
        }
        cut[t] = c;
    }

    let edge_start_usize = edge_start as usize;

    let off_atomic = (0..=n + 1).map(|_| AtomicU32::new(0)).collect::<Vec<_>>();
    let has_in_atomic = (0..=n).map(|_| AtomicBool::new(false)).collect::<Vec<_>>();

    // Pass 1: Count out-degrees and mark has_in
    thread::scope(|s| {
        for t in 0..nthreads {
            let c_start = cut[t];
            let c_end = cut[t + 1];
            let off_ref = &off_atomic;
            let has_in_ref = &has_in_atomic;
            s.spawn(move || unsafe {
                let mut q = (edge_start_usize + c_start) as *const u8;
                let q_end = (edge_start_usize + c_end) as *const u8;
                while q < q_end {
                    while q < q_end && *q <= b' ' {
                        q = q.add(1);
                    }
                    if q >= q_end {
                        break;
                    }
                    let u = parse_u32(&mut q);
                    q = q.add(1);
                    let v = parse_u32(&mut q);
                    q = q.add(1);
                    let _w = parse_u32(&mut q);
                    while q < q_end && *q != b'\n' {
                        q = q.add(1);
                    }
                    if q < q_end {
                        q = q.add(1);
                    }
                    if u != v && (u as usize) < n && (v as usize) < n {
                        off_ref[u as usize + 1].fetch_add(1, Ordering::Relaxed);
                        has_in_ref[v as usize].store(true, Ordering::Relaxed);
                    }
                }
            });
        }
    });
    let mut off = vec![0u32; n + 2];
    for i in 0..=n {
        off[i + 1] = off[i] + off_atomic[i + 1].load(Ordering::Relaxed);
    }
    drop(off_atomic);

    let m_valid = off[n] as usize;
    let mut edges = vec![Edge { v: 0, w: 0 }; m_valid];
    let edges_addr = edges.as_mut_ptr() as usize;

    let cur = (0..=n)
        .map(|i| AtomicU32::new(off[i]))
        .collect::<Vec<_>>();

    // Pass 2: Scatter edges into array
    thread::scope(|s| {
        for t in 0..nthreads {
            let c_start = cut[t];
            let c_end = cut[t + 1];
            let cur_ref = &cur;
            s.spawn(move || unsafe {
                let e_ptr = edges_addr as *mut Edge;
                let mut q = (edge_start_usize + c_start) as *const u8;
                let q_end = (edge_start_usize + c_end) as *const u8;
                while q < q_end {
                    while q < q_end && *q <= b' ' {
                        q = q.add(1);
                    }
                    if q >= q_end {
                        break;
                    }
                    let u = parse_u32(&mut q);
                    q = q.add(1);
                    let v = parse_u32(&mut q);
                    q = q.add(1);
                    let w = parse_u32(&mut q);
                    while q < q_end && *q != b'\n' {
                        q = q.add(1);
                    }
                    if q < q_end {
                        q = q.add(1);
                    }
                    if u != v && (u as usize) < n && (v as usize) < n {
                        let idx = cur_ref[u as usize].fetch_add(1, Ordering::Relaxed);
                        *e_ptr.add(idx as usize) = Edge { v, w };
                    }
                }
            });
        }
    });
    // Unmap graph.txt immediately!
    unsafe {
        munmap(graph_ptr as *mut u8, gsize + 4096);
    }
    drop(cur);
    drop(cut);

    // BFS Reordering
    let mut order = vec![0u32; n];
    let mut perm = vec![u32::MAX; n];
    let mut tail = 0usize;

    for r in 0..n {
        if perm[r] != u32::MAX {
            continue;
        }
        perm[r] = tail as u32;
        order[tail] = r as u32;
        tail += 1;
        let mut h = tail - 1;
        while h < tail {
            #[cfg(target_arch = "x86_64")]
            unsafe {
                if h + 16 < tail {
                    let next_u = order[h + 16] as usize;
                    core::arch::x86_64::_mm_prefetch(off.as_ptr().add(next_u) as *const i8, core::arch::x86_64::_MM_HINT_T0);
                }
                if h + 8 < tail {
                    let next_e = off[order[h + 8] as usize] as usize;
                    core::arch::x86_64::_mm_prefetch(edges.as_ptr().add(next_e) as *const i8, core::arch::x86_64::_MM_HINT_T0);
                }
            }
            let u = order[h] as usize;
            h += 1;
            let start = off[u] as usize;
            let end = off[u + 1] as usize;
            for e in start..end {
                let v = edges[e].v as usize;
                if perm[v] == u32::MAX {
                    perm[v] = tail as u32;
                    order[tail] = v as u32;
                    tail += 1;
                }
            }
        }
    }
    let mut off2 = vec![0u32; n + 2];
    for i in 0..n {
        let u = order[i] as usize;
        off2[i + 1] = off[u + 1] - off[u];
    }
    for i in 0..n {
        off2[i + 1] += off2[i];
    }

    let mut pk = vec![0u32; m_valid];
    let pk_addr = pk.as_mut_ptr() as usize;

    let mut has_in2 = vec![false; n];
    let has_in2_addr = has_in2.as_mut_ptr() as usize;

    let mut thread_escs = vec![Vec::new(); nthreads];

    thread::scope(|s| {
        for (t, esc_out) in thread_escs.iter_mut().enumerate() {
            let lo = n * t / nthreads;
            let hi = n * (t + 1) / nthreads;
            let order_ref = &order;
            let perm_ref = &perm;
            let off_ref = &off;
            let off2_ref = &off2;
            let edges_ref = &edges;
            let has_in_ref = &has_in_atomic;
            s.spawn(move || unsafe {
                let p_pk = pk_addr as *mut u32;
                let p_hi = has_in2_addr as *mut bool;
                for i in lo..hi {
                    let u = order_ref[i] as usize;
                    let mut k = off2_ref[i] as usize;
                    let start = off_ref[u] as usize;
                    let end = off_ref[u + 1] as usize;
                    for e in start..end {
                        let v = perm_ref[edges_ref[e].v as usize];
                        let w = edges_ref[e].w;
                        let dv = v as i64 - i as i64;
                        if dv >= i16::MIN as i64 && dv <= i16::MAX as i64 && w < ESC_MARKER {
                            *p_pk.add(k) = ((dv as i16 as u16 as u32) << 16) | (w & 0xFFFF);
                        } else {
                            *p_pk.add(k) = ESC_MARKER;
                            esc_out.push(Esc { e: k as u32, v, w });
                        }
                        k += 1;
                    }
                    *p_hi.add(i) = has_in_ref[u].load(Ordering::Relaxed);
                }
            });
        }
    });
    let mut esc = Vec::new();
    for te in thread_escs {
        esc.extend(te);
    }

    drop(edges);
    drop(off);
    drop(has_in_atomic);
    drop(order);

    // Remap queries with perm
    for q in &mut queries {
        q.0 = perm[q.0 as usize];
        q.1 = perm[q.1 as usize];
    }
    drop(perm);

    // Parallel query execution with Dial's algorithm
    let mut answers = vec![-1i64; queries.len()];
    let next_q = AtomicUsize::new(0);
    let answers_addr = answers.as_mut_ptr() as usize;

    thread::scope(|s| {
        for _ in 0..nthreads {
            let next_q_ref = &next_q;
            let queries_ref = &queries;
            let pk_ref = &pk;
            let esc_ref = &esc;
            let off2_ref = &off2;
            let has_in2_ref = &has_in2;
            s.spawn(move || {
                let ans = answers_addr as *mut i64;
                let mut d32 = vec![u32::MAX; n];
                let mut touched = Vec::with_capacity(65536);
                let mut ring: Vec<Vec<u32>> = (0..RING).map(|_| Vec::with_capacity(32)).collect();

                loop {
                    let q = next_q_ref.fetch_add(1, Ordering::Relaxed);
                    if q >= queries_ref.len() {
                        break;
                    }
                    let (sq, tq) = queries_ref[q];
                    if sq == tq {
                        unsafe {
                            *ans.add(q) = 0;
                        }
                        continue;
                    }
                    if !has_in2_ref[tq as usize] || off2_ref[sq as usize] == off2_ref[sq as usize + 1] {
                        unsafe {
                            *ans.add(q) = -1;
                        }
                        continue;
                    }

                    let res = dial(
                        pk_ref,
                        esc_ref,
                        off2_ref,
                        sq,
                        tq,
                        &mut d32,
                        &mut touched,
                        &mut ring,
                    );
                    unsafe {
                        *ans.add(q) = res;
                    }
                }
            });
        }
    });

    // Format output
    let mut out_bytes = Vec::with_capacity(answers.len() * 12);
    let mut num_buf = [0u8; 32];
    for &a in &answers {
        if a < 0 {
            out_bytes.extend_from_slice(b"-1\n");
        } else {
            let mut x = a as u64;
            let mut p = 31;
            num_buf[p] = b'\n';
            if x == 0 {
                p -= 1;
                num_buf[p] = b'0';
            } else {
                while x > 0 {
                    p -= 1;
                    num_buf[p] = b'0' + (x % 10) as u8;
                    x /= 10;
                }
            }
            out_bytes.extend_from_slice(&num_buf[p..]);
        }
    }

    let mut out_file = File::create(&out_path)?;
    out_file.write_all(&out_bytes)?;
    out_file.flush()?;

    Ok(())
}
