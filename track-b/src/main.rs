use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::hash::{BuildHasherDefault, Hasher};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

#[derive(Default)]
struct FastHasher(u64);

impl Hasher for FastHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0
    }
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };
        for &b in bytes {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        self.0 = h;
    }
}

type FastHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    And,
    Or,
    Not,
}

fn combine_scalar(acc: &mut [u64], rhs: &[u64], op: Op) {
    assert_eq!(acc.len(), rhs.len());
    for (a, &b) in acc.iter_mut().zip(rhs) {
        *a = match op {
            Op::And => *a & b,
            Op::Or => *a | b,
            Op::Not => *a & !b,
        };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn combine_avx2(acc: &mut [u64], rhs: &[u64], op: Op) {
    use std::arch::x86_64::*;
    assert_eq!(acc.len(), rhs.len());
    let end = acc.len() / 4 * 4;
    for i in (0..end).step_by(4) {
        // Both slices contain four words here; unaligned loads support Vec<u64>.
        let a = _mm256_loadu_si256(acc.as_ptr().add(i).cast());
        let b = _mm256_loadu_si256(rhs.as_ptr().add(i).cast());
        let result = match op {
            Op::And => _mm256_and_si256(a, b),
            Op::Or => _mm256_or_si256(a, b),
            Op::Not => _mm256_andnot_si256(b, a),
        };
        _mm256_storeu_si256(acc.as_mut_ptr().add(i).cast(), result);
    }
    combine_scalar(&mut acc[end..], &rhs[end..], op);
}

fn bitset_combiner() -> fn(&mut [u64], &[u64], Op) {
    #[cfg(target_arch = "x86_64")]
    if std::is_x86_feature_detected!("avx2") {
        // The runtime check guards every call, including in generic CPU builds.
        return |acc, rhs, op| unsafe { combine_avx2(acc, rhs, op) };
    }
    combine_scalar
}

enum Query {
    Bool {
        terms: Vec<u32>,
        ops: Vec<Op>,
    },
    Phrase {
        terms: Vec<u32>,
        raw_terms: Vec<Vec<u8>>,
    },
}

const IS_TOKEN_CHAR: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0usize;
    while i < 256 {
        let b = i as u8;
        if (b >= b'a' && b <= b'z') || (b >= b'0' && b <= b'9') || (b >= b'A' && b <= b'Z') {
            table[i] = true;
        }
        i += 1;
    }
    table
};

const LOWER_ASCII: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let b = i as u8;
        if b >= b'A' && b <= b'Z' {
            table[i] = b + 32;
        } else {
            table[i] = b;
        }
        i += 1;
    }
    table
};

#[inline(always)]
fn is_token_char(b: u8) -> bool {
    IS_TOKEN_CHAR[b as usize]
}

#[inline(always)]
fn to_lower_ascii(b: u8) -> u8 {
    LOWER_ASCII[b as usize]
}

fn tokenize_text<F: FnMut(&[u8])>(text: &[u8], mut callback: F) {
    let mut i = 0;
    let n = text.len();
    let mut buf = [0u8; 32];
    while i < n {
        let b = text[i];
        if is_token_char(b) {
            let mut buf_len = 0;
            while i < n {
                let c = text[i];
                if is_token_char(c) {
                    if buf_len < 32 {
                        buf[buf_len] = to_lower_ascii(c);
                        buf_len += 1;
                    }
                    i += 1;
                } else {
                    break;
                }
            }
            callback(&buf[..buf_len]);
        } else {
            i += 1;
        }
    }
}

fn check_phrase_in_doc(doc_text: &[u8], phrase_terms: &[Vec<u8>]) -> bool {
    let k = phrase_terms.len();
    let mut stack_spans = [(0u32, 0u32); 1024];
    let mut num_tokens = 0;
    let mut heap_spans: Vec<(u32, u32)> = Vec::new();

    let mut i = 0;
    let n = doc_text.len();
    while i < n {
        let b = doc_text[i];
        if is_token_char(b) {
            let start = i;
            while i < n && is_token_char(doc_text[i]) {
                i += 1;
            }
            let len = i - start;
            if num_tokens < 1024 {
                stack_spans[num_tokens] = (start as u32, len as u32);
            } else if num_tokens == 1024 {
                heap_spans.reserve(1024);
                heap_spans.extend_from_slice(&stack_spans);
                heap_spans.push((start as u32, len as u32));
            } else {
                heap_spans.push((start as u32, len as u32));
            }
            num_tokens += 1;
        } else {
            i += 1;
        }
    }

    if num_tokens < k {
        return false;
    }

    let spans: &[(u32, u32)] = if num_tokens <= 1024 {
        &stack_spans[..num_tokens]
    } else {
        &heap_spans[..]
    };

    for i in 0..=(num_tokens - k) {
        let mut matches = true;
        for j in 0..k {
            let (start, len) = spans[i + j];
            let tok = &doc_text[start as usize..(start as usize + len as usize)];
            let target = &phrase_terms[j];
            if tok.len() != target.len() {
                matches = false;
                break;
            }
            let mut eq = true;
            for b_idx in 0..tok.len() {
                if to_lower_ascii(tok[b_idx]) != target[b_idx] {
                    eq = false;
                    break;
                }
            }
            if !eq {
                matches = false;
                break;
            }
        }
        if matches {
            return true;
        }
    }
    false
}

struct ParsedQueries {
    queries: Vec<Query>,
    term_to_id: FastHashMap<Vec<u8>, u32>,
}

fn parse_queries(queries_bytes: &[u8]) -> ParsedQueries {
    let mut queries = Vec::new();
    let mut term_to_id: FastHashMap<Vec<u8>, u32> = FastHashMap::default();
    let mut next_term_id: u32 = 0;

    let mut get_term_id = |term: &[u8], map: &mut FastHashMap<Vec<u8>, u32>| -> u32 {
        if let Some(&id) = map.get(term) {
            id
        } else {
            let id = next_term_id;
            next_term_id += 1;
            map.insert(term.to_vec(), id);
            id
        }
    };

    let mut start = 0;
    let len = queries_bytes.len();
    while start < len {
        let mut end = start;
        while end < len && queries_bytes[end] != b'\n' {
            end += 1;
        }
        let mut line = &queries_bytes[start..end];
        if line.ends_with(b"\r") {
            line = &line[..line.len() - 1];
        }
        start = end + 1;

        if line.is_empty() {
            continue;
        }

        if line.starts_with(b"BOOL ") {
            let rest = &line[5..];
            let parts: Vec<&[u8]> = rest
                .split(|&b| b == b' ')
                .filter(|s| !s.is_empty())
                .collect();
            let mut terms = Vec::with_capacity((parts.len() + 1) / 2);
            let mut ops = Vec::with_capacity(parts.len() / 2);

            terms.push(get_term_id(parts[0], &mut term_to_id));
            let mut i = 1;
            while i < parts.len() {
                let op = match parts[i] {
                    b"AND" => Op::And,
                    b"OR" => Op::Or,
                    b"NOT" => Op::Not,
                    _ => panic!("Unknown operator {:?}", parts[i]),
                };
                ops.push(op);
                terms.push(get_term_id(parts[i + 1], &mut term_to_id));
                i += 2;
            }
            queries.push(Query::Bool { terms, ops });
        } else if line.starts_with(b"PHRASE \"") && line.ends_with(b"\"") {
            let inner = &line[8..line.len() - 1];
            let parts: Vec<&[u8]> = inner
                .split(|&b| b == b' ')
                .filter(|s| !s.is_empty())
                .collect();
            let mut terms = Vec::with_capacity(parts.len());
            let mut raw_terms = Vec::with_capacity(parts.len());
            for part in parts {
                terms.push(get_term_id(part, &mut term_to_id));
                raw_terms.push(part.to_vec());
            }
            queries.push(Query::Phrase { terms, raw_terms });
        }
    }

    ParsedQueries {
        queries,
        term_to_id,
    }
}

fn main() {
    let mut data_dir = PathBuf::from("/data");
    let mut out_dir = PathBuf::from("/out");

    let args: Vec<String> = env::args().collect();
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--data" => {
                if idx + 1 < args.len() {
                    data_dir = PathBuf::from(&args[idx + 1]);
                    idx += 1;
                }
            }
            "--out" => {
                if idx + 1 < args.len() {
                    out_dir = PathBuf::from(&args[idx + 1]);
                    idx += 1;
                }
            }
            _ => {}
        }
        idx += 1;
    }

    let queries_path = data_dir.join("queries.txt");
    let mut queries_bytes = Vec::new();
    File::open(&queries_path)
        .unwrap_or_else(|e| panic!("Failed to open {:?}: {}", queries_path, e))
        .read_to_end(&mut queries_bytes)
        .expect("Failed to read queries.txt");

    let parsed = parse_queries(&queries_bytes);
    let num_query_terms = parsed.term_to_id.len();

    let docs_path = data_dir.join("docs.txt");
    let docs_file = File::open(&docs_path).expect("Failed to open docs.txt");
    let docs_len = docs_file.metadata().expect("Failed to stat docs.txt").len();
    let mut boundary_reader = BufReader::new(docs_file);
    let arc_term_to_id = Arc::new(parsed.term_to_id);

    let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(16);

    let mut chunk_boundaries = Vec::with_capacity(num_threads + 1);
    chunk_boundaries.push(0);
    for t in 1..num_threads {
        let approx = t as u64 * docs_len / num_threads as u64;
        boundary_reader
            .seek(SeekFrom::Start(approx))
            .expect("Failed to seek docs.txt");
        let skipped = boundary_reader
            .skip_until(b'\n')
            .expect("Failed to find document boundary");
        chunk_boundaries.push(approx + skipped as u64);
    }
    chunk_boundaries.push(docs_len);

    let mut ingest_handles = Vec::with_capacity(num_threads);
    for t in 0..num_threads {
        let start_pos = chunk_boundaries[t];
        let end_pos = chunk_boundaries[t + 1];
        if start_pos >= end_pos {
            continue;
        }

        let docs_path = docs_path.clone();
        let term_to_id = Arc::clone(&arc_term_to_id);

        ingest_handles.push(thread::spawn(move || {
            let mut file = File::open(docs_path).expect("Failed to open docs.txt");
            file.seek(SeekFrom::Start(start_pos))
                .expect("Failed to seek document chunk");
            let mut reader = BufReader::with_capacity(1 << 20, file.take(end_pos - start_pos));
            let mut line = Vec::new();
            let mut local_doc_spans = Vec::new();
            let mut local_postings: Vec<Vec<u32>> = vec![Vec::new(); num_query_terms];
            let mut last_seen: Vec<u32> = vec![u32::MAX; num_query_terms];

            let mut line_start = start_pos;
            while line_start < end_pos {
                line.clear();
                let read = reader
                    .read_until(b'\n', &mut line)
                    .expect("Failed to read document");
                assert_ne!(read, 0, "Unexpected end of docs.txt");
                let line_end = line.len() - usize::from(line.ends_with(b"\n"));

                let mut tab_pos = 0;
                while tab_pos < line_end && line[tab_pos] != b'\t' {
                    tab_pos += 1;
                }

                let mut doc_id: u32 = 0;
                for &b in &line[..tab_pos] {
                    doc_id = doc_id * 10 + (b - b'0') as u32;
                }

                let text_start = if tab_pos < line_end {
                    tab_pos + 1
                } else {
                    line_end
                };
                let text_len = line_end - text_start;

                local_doc_spans.push((doc_id, line_start + text_start as u64, text_len));
                let doc_text = &line[text_start..line_end];

                tokenize_text(doc_text, |tok| {
                    if let Some(&term_id) = term_to_id.get(tok) {
                        let tid = term_id as usize;
                        if last_seen[tid] != doc_id {
                            last_seen[tid] = doc_id;
                            local_postings[tid].push(doc_id);
                        }
                    }
                });

                line_start += read as u64;
            }
            (local_doc_spans, local_postings)
        }));
    }

    let mut thread_results = Vec::with_capacity(ingest_handles.len());
    let mut total_docs = 0;
    for h in ingest_handles {
        let (spans, postings) = h.join().expect("Ingest worker thread panicked");
        total_docs += spans.len();
        thread_results.push((spans, postings));
    }

    let mut doc_spans: Vec<(u64, usize)> = vec![(0, 0); total_docs];
    for (spans, _) in &thread_results {
        for &(doc_id, text_start, text_len) in spans {
            doc_spans[doc_id as usize] = (text_start, text_len);
        }
    }

    let mut postings: Vec<Vec<u32>> = vec![Vec::new(); num_query_terms];
    for term_id in 0..num_query_terms {
        let cap: usize = thread_results.iter().map(|(_, p)| p[term_id].len()).sum();
        postings[term_id].reserve_exact(cap);
        for (_, p) in &mut thread_results {
            // Release each worker allocation as soon as its contents are merged.
            postings[term_id].extend_from_slice(&std::mem::take(&mut p[term_id]));
        }
    }
    drop(thread_results);

    let num_docs = total_docs;
    let words_count = (num_docs + 63) / 64;

    // Cache only frequent terms: a bitmap costs at most half its posting bytes.
    // Sparse lists remain available for phrase candidate filtering.
    let dense: Vec<Vec<u64>> = postings
        .iter()
        .map(|posts| {
            if posts.len() < (words_count * 4).max(256) {
                return Vec::new();
            }
            let mut bits = vec![0u64; words_count];
            for &d in posts {
                bits[d as usize >> 6] |= 1u64 << (d & 63);
            }
            bits
        })
        .collect();
    let arc_dense = Arc::new(dense);

    let arc_doc_spans = Arc::new(doc_spans);
    let arc_postings = Arc::new(postings);
    let arc_queries = Arc::new(parsed.queries);

    let total_queries = arc_queries.len();
    let chunk_size = (total_queries + num_threads - 1) / num_threads;

    let mut handles = Vec::with_capacity(num_threads);

    for t in 0..num_threads {
        let q_start = t * chunk_size;
        let q_end = (q_start + chunk_size).min(total_queries);
        if q_start >= total_queries {
            break;
        }

        let docs_path = docs_path.clone();
        let doc_spans = Arc::clone(&arc_doc_spans);
        let postings = Arc::clone(&arc_postings);
        let dense = Arc::clone(&arc_dense);
        let queries = Arc::clone(&arc_queries);

        handles.push(thread::spawn(move || {
            // Read only candidates that survived every phrase term's posting filter.
            // Each worker owns its file cursor and reuses one document buffer.
            let mut docs_file = File::open(docs_path).expect("Failed to open phrase corpus");
            let mut doc_text = Vec::new();
            let combine = bitset_combiner();
            let mut out_chunk = Vec::with_capacity((q_end - q_start) * 48);
            let mut r = vec![0u64; words_count];
            let mut temp = vec![0u64; words_count];
            let mut num_buf = [0u8; 32];

            for q_idx in q_start..q_end {
                match &queries[q_idx] {
                    Query::Bool { terms, ops } => {
                        r.fill(0);
                        let first_tid = terms[0] as usize;
                        if first_tid < postings.len() {
                            if !dense[first_tid].is_empty() {
                                r.copy_from_slice(&dense[first_tid]);
                            } else {
                                for &d in &postings[first_tid] {
                                    r[d as usize >> 6] |= 1u64 << (d & 63);
                                }
                            }
                        }

                        for (op_idx, &op) in ops.iter().enumerate() {
                            let tid = terms[op_idx + 1] as usize;
                            if tid >= postings.len() || postings[tid].is_empty() {
                                match op {
                                    Op::And => r.fill(0),
                                    Op::Or => {}
                                    Op::Not => {}
                                }
                            } else if !dense[tid].is_empty() {
                                combine(&mut r, &dense[tid], op);
                            } else {
                                let term_posts = &postings[tid];
                                match op {
                                    Op::Or => {
                                        for &d in term_posts {
                                            r[d as usize >> 6] |= 1u64 << (d & 63);
                                        }
                                    }
                                    Op::Not => {
                                        for &d in term_posts {
                                            r[d as usize >> 6] &= !(1u64 << (d & 63));
                                        }
                                    }
                                    Op::And => {
                                        if term_posts.len() < 256 {
                                            let mut surviving = [0u32; 256];
                                            let mut s_cnt = 0;
                                            for &d in term_posts {
                                                if (r[d as usize >> 6] & (1u64 << (d & 63))) != 0 {
                                                    surviving[s_cnt] = d;
                                                    s_cnt += 1;
                                                }
                                            }
                                            r.fill(0);
                                            for i in 0..s_cnt {
                                                let d = surviving[i];
                                                r[d as usize >> 6] |= 1u64 << (d & 63);
                                            }
                                        } else {
                                            for &d in term_posts {
                                                temp[d as usize >> 6] |= 1u64 << (d & 63);
                                            }
                                            combine(&mut r, &temp, Op::And);
                                            for &d in term_posts {
                                                temp[d as usize >> 6] = 0;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let mut count: u32 = 0;
                        let mut sum: u64 = 0;
                        let mut first_10 = [0u32; 10];
                        let mut first_count = 0;

                        for (w_idx, &word) in r.iter().enumerate() {
                            if word == 0 {
                                continue;
                            }
                            count += word.count_ones();
                            let mut w = word;
                            while w != 0 {
                                let bit = w.trailing_zeros();
                                let doc_id = (w_idx as u64 * 64) + bit as u64;
                                sum += doc_id;
                                if first_count < 10 {
                                    first_10[first_count] = doc_id as u32;
                                    first_count += 1;
                                }
                                w &= w - 1;
                            }
                        }

                        if count == 0 {
                            out_chunk.extend_from_slice(b"0 0\n");
                        } else {
                            fast_write_u64(&mut out_chunk, count as u64, &mut num_buf);
                            out_chunk.push(b' ');
                            fast_write_u64(&mut out_chunk, sum, &mut num_buf);
                            for &id in &first_10[..first_count] {
                                out_chunk.push(b' ');
                                fast_write_u64(&mut out_chunk, id as u64, &mut num_buf);
                            }
                            out_chunk.push(b'\n');
                        }
                    }
                    Query::Phrase { terms, raw_terms } => {
                        let mut has_empty = false;
                        for &tid in terms {
                            let t = tid as usize;
                            if t >= postings.len() || postings[t].is_empty() {
                                has_empty = true;
                                break;
                            }
                        }

                        if has_empty {
                            out_chunk.extend_from_slice(b"0 0\n");
                            continue;
                        }

                        let mut order: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
                        let k = terms.len();
                        let sub_order = &mut order[..k];
                        for i in 0..k {
                            sub_order[i] = i;
                        }
                        sub_order.sort_unstable_by_key(|&idx| postings[terms[idx] as usize].len());

                        let anchor_tid = terms[sub_order[0]] as usize;
                        let anchor_posts = &postings[anchor_tid];

                        let mut matches = Vec::new();
                        for &doc_id in anchor_posts {
                            let mut present_in_all = true;
                            for &idx in &sub_order[1..] {
                                let other_tid = terms[idx] as usize;
                                let other_posts = &postings[other_tid];
                                if other_posts.binary_search(&doc_id).is_err() {
                                    present_in_all = false;
                                    break;
                                }
                            }
                            if present_in_all {
                                let (start, len) = doc_spans[doc_id as usize];
                                doc_text.resize(len, 0);
                                docs_file
                                    .seek(SeekFrom::Start(start))
                                    .expect("Failed to seek phrase candidate");
                                docs_file
                                    .read_exact(&mut doc_text)
                                    .expect("Failed to read phrase candidate");
                                if check_phrase_in_doc(&doc_text, raw_terms) {
                                    matches.push(doc_id);
                                }
                            }
                        }

                        if matches.is_empty() {
                            out_chunk.extend_from_slice(b"0 0\n");
                        } else {
                            let count = matches.len();
                            let sum: u64 = matches.iter().map(|&x| x as u64).sum();
                            let m = count.min(10);
                            fast_write_u64(&mut out_chunk, count as u64, &mut num_buf);
                            out_chunk.push(b' ');
                            fast_write_u64(&mut out_chunk, sum, &mut num_buf);
                            for &id in &matches[..m] {
                                out_chunk.push(b' ');
                                fast_write_u64(&mut out_chunk, id as u64, &mut num_buf);
                            }
                            out_chunk.push(b'\n');
                        }
                    }
                }
            }
            out_chunk
        }));
    }

    std::fs::create_dir_all(&out_dir).ok();
    let results_path = out_dir.join("results.txt");
    let mut writer = BufWriter::with_capacity(
        1 << 20,
        File::create(&results_path).expect("Failed to create results.txt"),
    );

    let mut chunks = Vec::with_capacity(handles.len());
    for h in handles {
        let chunk = h.join().expect("Worker thread panicked");
        chunks.push(chunk);
    }
    for chunk in chunks {
        writer
            .write_all(&chunk)
            .expect("Failed to write results chunk");
    }
    writer.flush().expect("Failed to flush results.txt");
}

#[inline(always)]
fn fast_write_u64(buf: &mut Vec<u8>, mut n: u64, tmp: &mut [u8; 32]) {
    if n == 0 {
        buf.push(b'0');
        return;
    }
    let mut pos = 0;
    while n > 0 {
        tmp[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        pos += 1;
    }
    while pos > 0 {
        pos -= 1;
        buf.push(tmp[pos]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitset_operations_cover_unaligned_vectors_and_tails() {
        let combine = bitset_combiner();
        for len in (0..70).chain([3907, 4097]) {
            let a: Vec<u64> = (0..len + 2)
                .map(|i| (i as u64).wrapping_mul(0x9e3779b97f4a7c15))
                .collect();
            let b: Vec<u64> = a.iter().map(|x| x.rotate_left(17) ^ u64::MAX).collect();
            for op in [Op::And, Op::Or, Op::Not] {
                let mut actual = a.clone();
                let mut expected = a.clone();
                combine_scalar(&mut expected[1..len + 1], &b[1..len + 1], op);
                combine(&mut actual[1..len + 1], &b[1..len + 1], op);
                assert_eq!(actual, expected, "len={len}, op={op:?}");
            }
        }
    }

    #[test]
    fn phrases_preserve_repetition_ascii_rules_and_heap_fallback() {
        let terms = vec![b"bail".to_vec(), b"bail".to_vec(), b"bond".to_vec()];
        assert!(check_phrase_in_doc("BAIL—bail, BOND".as_bytes(), &terms));
        assert!(!check_phrase_in_doc(b"bail bond", &terms));
        let mut long = b"x ".repeat(1100);
        long.extend_from_slice(b"bail bail bond");
        assert!(check_phrase_in_doc(&long, &terms));
        assert!(!check_phrase_in_doc(
            "Zürich".as_bytes(),
            &[b"zurich".to_vec()]
        ));
    }
}
