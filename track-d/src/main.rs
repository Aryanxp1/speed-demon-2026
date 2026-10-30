use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

const INF: u64 = u64::MAX;

// POSIX mmap / munmap bindings for zero-copy file mapping
extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

const PROT_READ: i32 = 1;
const MAP_PRIVATE: i32 = 0x02;
const MAP_FAILED: *mut u8 = !0 as *mut u8;

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
unsafe fn number(ptr: &mut *const u8, end: *const u8) -> u32 {
    let mut v = 0u32;
    while *ptr < end && **ptr >= b'0' && **ptr <= b'9' {
        v = v * 10 + (**ptr - b'0') as u32;
        *ptr = ptr.add(1);
    }
    v
}

#[derive(Copy, Clone)]
struct Edge {
    v: u32,
    w: u32,
}

#[derive(Copy, Clone)]
struct Item {
    key: u64,
    node: u32,
}

struct RadixHeap {
    buckets: [Vec<Item>; 65],
    last: u64,
    size: usize,
}

impl RadixHeap {
    fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| Vec::new()),
            last: 0,
            size: 0,
        }
    }

    #[inline(always)]
    fn clear(&mut self) {
        for b in &mut self.buckets {
            b.clear();
        }
        self.last = 0;
        self.size = 0;
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.size == 0
    }

    #[inline(always)]
    fn push(&mut self, key: u64, node: u32) {
        let diff = key ^ self.last;
        let bucket = if diff == 0 {
            0
        } else {
            64 - diff.leading_zeros() as usize
        };
        self.buckets[bucket].push(Item { key, node });
        self.size += 1;
    }

    #[inline(always)]
    fn top(&mut self) -> Item {
        if self.buckets[0].is_empty() {
            let mut i = 1;
            while self.buckets[i].is_empty() {
                i += 1;
            }
            let mut min_key = INF;
            for item in &self.buckets[i] {
                if item.key < min_key {
                    min_key = item.key;
                }
            }
            self.last = min_key;
            let items = std::mem::take(&mut self.buckets[i]);
            for item in items {
                let diff = item.key ^ self.last;
                let b = if diff == 0 {
                    0
                } else {
                    64 - diff.leading_zeros() as usize
                };
                self.buckets[b].push(item);
            }
        }
        *self.buckets[0].last().unwrap()
    }

    #[inline(always)]
    fn pop(&mut self) -> Item {
        let item = self.top();
        self.buckets[0].pop();
        self.size -= 1;
        item
    }
}

const PAGE: usize = 512;

struct PagedDist {
    table: Vec<*mut u64>,
    spare: Vec<*mut u64>,
    owned: Vec<*mut u64>,
    used: Vec<u32>,
}

impl PagedDist {
    fn new(n: usize) -> Self {
        let num_pages = (n + PAGE - 1) / PAGE;
        Self {
            table: vec![std::ptr::null_mut(); num_pages],
            spare: Vec::with_capacity(64),
            owned: Vec::with_capacity(64),
            used: Vec::with_capacity(64),
        }
    }

    #[inline(always)]
    fn get(&self, v: u32) -> u64 {
        let page_idx = (v as usize) / PAGE;
        let p = self.table[page_idx];
        if p.is_null() {
            INF
        } else {
            unsafe { *p.add((v as usize) % PAGE) }
        }
    }

    #[inline(always)]
    fn set(&mut self, v: u32, d: u64) {
        let page_idx = (v as usize) / PAGE;
        let p = self.table[page_idx];
        if p.is_null() {
            let new_page = if let Some(sp) = self.spare.pop() {
                sp
            } else {
                let mut vec = vec![INF; PAGE];
                let ptr = vec.as_mut_ptr();
                std::mem::forget(vec);
                self.owned.push(ptr);
                ptr
            };
            unsafe {
                std::ptr::write_bytes(new_page, 0xFF, PAGE);
            }
            self.table[page_idx] = new_page;
            self.used.push(page_idx as u32);
            unsafe {
                *new_page.add((v as usize) % PAGE) = d;
            }
        } else {
            unsafe {
                *p.add((v as usize) % PAGE) = d;
            }
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        for &idx in &self.used {
            let p = self.table[idx as usize];
            self.spare.push(p);
            self.table[idx as usize] = std::ptr::null_mut();
        }
        self.used.clear();
    }
}

impl Drop for PagedDist {
    fn drop(&mut self) {
        for &p in &self.owned {
            unsafe {
                drop(Vec::from_raw_parts(p, PAGE, PAGE));
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let nthreads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(12)
        .min(12)
        .max(1);

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
            let s = number(&mut p, end);
            while p < end && *p <= b' ' {
                p = p.add(1);
            }
            let t = number(&mut p, end);
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
        let n = number(&mut p, end);
        while p < end && *p <= b' ' {
            p = p.add(1);
        }
        let _m = number(&mut p, end);
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

    // Split edge section across threads
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

    // Pass 1: Count forward and backward degrees in parallel
    let mut f_count = vec![vec![0u32; n]; nthreads];
    let mut b_count = vec![vec![0u32; n]; nthreads];

    thread::scope(|s| {
        for (t, (fc, bc)) in f_count.iter_mut().zip(b_count.iter_mut()).enumerate() {
            let c_start = cut[t];
            let c_end = cut[t + 1];
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
                    let u = number(&mut q, q_end);
                    while q < q_end && *q <= b' ' {
                        q = q.add(1);
                    }
                    let v = number(&mut q, q_end);
                    while q < q_end && *q <= b' ' {
                        q = q.add(1);
                    }
                    let _w = number(&mut q, q_end);
                    while q < q_end && *q != b'\n' {
                        q = q.add(1);
                    }
                    if q < q_end {
                        q = q.add(1);
                    }
                    if u != v && (u as usize) < n && (v as usize) < n {
                        fc[u as usize] += 1;
                        bc[v as usize] += 1;
                    }
                }
            });
        }
    });

    // Compute prefix sums (CSR heads)
    let mut fwd_head = vec![0u32; n + 1];
    let mut bwd_head = vec![0u32; n + 1];
    let mut f_cur = vec![vec![0u32; n]; nthreads];
    let mut b_cur = vec![vec![0u32; n]; nthreads];

    let mut f_total = 0u32;
    let mut b_total = 0u32;
    for u in 0..n {
        fwd_head[u] = f_total;
        bwd_head[u] = b_total;
        for t in 0..nthreads {
            f_cur[t][u] = f_total;
            f_total += f_count[t][u];
            b_cur[t][u] = b_total;
            b_total += b_count[t][u];
        }
    }
    fwd_head[n] = f_total;
    bwd_head[n] = b_total;

    drop(f_count);
    drop(b_count);

    let mut fwd_edges = vec![Edge { v: 0, w: 0 }; f_total as usize];
    let mut bwd_edges = vec![Edge { v: 0, w: 0 }; b_total as usize];

    // Pass 2: Scatter edges into CSR arrays in parallel
    let fwd_edges_ptr = fwd_edges.as_mut_ptr() as usize;
    let bwd_edges_ptr = bwd_edges.as_mut_ptr() as usize;

    thread::scope(|s| {
        for (t, (fc, bc)) in f_cur.iter_mut().zip(b_cur.iter_mut()).enumerate() {
            let c_start = cut[t];
            let c_end = cut[t + 1];
            s.spawn(move || unsafe {
                let f_edges = fwd_edges_ptr as *mut Edge;
                let b_edges = bwd_edges_ptr as *mut Edge;
                let mut q = (edge_start_usize + c_start) as *const u8;
                let q_end = (edge_start_usize + c_end) as *const u8;
                while q < q_end {
                    while q < q_end && *q <= b' ' {
                        q = q.add(1);
                    }
                    if q >= q_end {
                        break;
                    }
                    let u = number(&mut q, q_end);
                    while q < q_end && *q <= b' ' {
                        q = q.add(1);
                    }
                    let v = number(&mut q, q_end);
                    while q < q_end && *q <= b' ' {
                        q = q.add(1);
                    }
                    let w = number(&mut q, q_end);
                    while q < q_end && *q != b'\n' {
                        q = q.add(1);
                    }
                    if q < q_end {
                        q = q.add(1);
                    }
                    if u != v && (u as usize) < n && (v as usize) < n {
                        let f_idx = fc[u as usize];
                        fc[u as usize] += 1;
                        *f_edges.add(f_idx as usize) = Edge { v, w };

                        let b_idx = bc[v as usize];
                        bc[v as usize] += 1;
                        *b_edges.add(b_idx as usize) = Edge { v: u, w };
                    }
                }
            });
        }
    });

    // Unmap graph.txt immediately before running queries!
    unsafe {
        munmap(graph_ptr as *mut u8, gsize + 4096);
    }
    drop(f_cur);
    drop(b_cur);
    drop(cut);

    // Query processing: Bidirectional Dijkstra with PagedDist and RadixHeap
    let mut answers = vec![-1i64; queries.len()];
    let next_q = AtomicUsize::new(0);
    let answers_ptr = answers.as_mut_ptr() as usize;
    let queries_ref = &queries;
    let fwd_head_ref = &fwd_head;
    let bwd_head_ref = &bwd_head;
    let fwd_edges_ref = &fwd_edges;
    let bwd_edges_ref = &bwd_edges;

    thread::scope(|s| {
        for _ in 0..nthreads {
            s.spawn(|| {
                let ans = answers_ptr as *mut i64;
                let mut heap_f = RadixHeap::new();
                let mut heap_b = RadixHeap::new();
                let mut dist_f = PagedDist::new(n);
                let mut dist_b = PagedDist::new(n);

                loop {
                    let q = next_q.fetch_add(1, Ordering::Relaxed);
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
                    let su = sq as usize;
                    let tu = tq as usize;
                    if su >= n
                        || tu >= n
                        || fwd_head_ref[su] == fwd_head_ref[su + 1]
                        || bwd_head_ref[tu] == bwd_head_ref[tu + 1]
                    {
                        unsafe {
                            *ans.add(q) = -1;
                        }
                        continue;
                    }

                    heap_f.clear();
                    heap_b.clear();
                    dist_f.reset();
                    dist_b.reset();

                    dist_f.set(sq, 0);
                    heap_f.push(0, sq);
                    dist_b.set(tq, 0);
                    heap_b.push(0, tq);

                    let mut best = INF;

                    while !heap_f.is_empty() && !heap_b.is_empty() {
                        while !heap_f.is_empty() && heap_f.top().key != dist_f.get(heap_f.top().node)
                        {
                            heap_f.pop();
                        }
                        while !heap_b.is_empty() && heap_b.top().key != dist_b.get(heap_b.top().node)
                        {
                            heap_b.pop();
                        }
                        if heap_f.is_empty() || heap_b.is_empty() {
                            break;
                        }

                        if heap_f.top().key.saturating_add(heap_b.top().key) >= best {
                            break;
                        }

                        let dir = if heap_f.top().key <= heap_b.top().key {
                            0
                        } else {
                            1
                        };
                        if dir == 0 {
                            let Item { key: du, node: u } = heap_f.pop();
                            let db_u = dist_b.get(u);
                            if db_u != INF {
                                best = best.min(du + db_u);
                            }

                            let start = fwd_head_ref[u as usize] as usize;
                            let end = fwd_head_ref[u as usize + 1] as usize;
                            for e in start..end {
                                let edge = fwd_edges_ref[e];
                                let v = edge.v;
                                let nd = du + edge.w as u64;
                                if nd < dist_f.get(v) {
                                    dist_f.set(v, nd);
                                    heap_f.push(nd, v);
                                    let db_v = dist_b.get(v);
                                    if db_v != INF {
                                        best = best.min(nd + db_v);
                                    }
                                }
                            }
                        } else {
                            let Item { key: du, node: u } = heap_b.pop();
                            let df_u = dist_f.get(u);
                            if df_u != INF {
                                best = best.min(du + df_u);
                            }

                            let start = bwd_head_ref[u as usize] as usize;
                            let end = bwd_head_ref[u as usize + 1] as usize;
                            for e in start..end {
                                let edge = bwd_edges_ref[e];
                                let v = edge.v;
                                let nd = du + edge.w as u64;
                                if nd < dist_b.get(v) {
                                    dist_b.set(v, nd);
                                    heap_b.push(nd, v);
                                    let df_v = dist_f.get(v);
                                    if df_v != INF {
                                        best = best.min(nd + df_v);
                                    }
                                }
                            }
                        }
                    }

                    unsafe {
                        *ans.add(q) = if best == INF { -1 } else { best as i64 };
                    }
                }
            });
        }
    });

    // Format output in memory buffer
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
