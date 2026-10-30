use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

// Scan for the next '"' byte using AVX2 when available.
fn find_quote(bytes: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    if std::is_x86_feature_detected!("avx2") {
        return unsafe { find_quote_avx2(bytes) };
    }
    bytes.iter().position(|&b| b == b'"').unwrap_or(bytes.len())
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn find_quote_avx2(bytes: &[u8]) -> usize {
    use std::arch::x86_64::*;
    let mut pos = 0;
    let quote = _mm256_set1_epi8(b'"' as i8);
    while pos + 32 <= bytes.len() {
        let data = _mm256_loadu_si256(bytes.as_ptr().add(pos).cast());
        let mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(data, quote)) as u32;
        if mask != 0 {
            return pos + mask.trailing_zeros() as usize;
        }
        pos += 32;
    }
    pos + bytes[pos..]
        .iter()
        .position(|&b| b == b'"')
        .unwrap_or(bytes.len() - pos)
}

// Per-thread hash table for (country_id, name_bytes) pairs.
// Uses a Vec<u8> arena for exact byte storage; collisions are resolved by
// full byte comparison (no false positives from hash collisions).
struct DistinctNameTable {
    slots: Vec<u32>,
    mask: usize,
    arena: Vec<u8>,
    items: Vec<(u8, u32, u16)>,
}

impl DistinctNameTable {
    fn with_capacity(capacity_pow2: usize) -> Self {
        Self {
            slots: vec![0; capacity_pow2],
            mask: capacity_pow2 - 1,
            arena: Vec::with_capacity(2 * 1024 * 1024),
            items: Vec::with_capacity(capacity_pow2 / 2),
        }
    }

    #[inline(always)]
    fn insert_if_absent(&mut self, country_id: u8, name: &[u8]) -> bool {
        let mut h: u64 = 0x517cc1b727220a95 ^ (country_id as u64);
        for &b in name {
            h = (h.rotate_left(5) ^ (b as u64)).wrapping_mul(0x517cc1b727220a95);
        }
        let mut idx = (h as usize) & self.mask;
        loop {
            let slot = self.slots[idx];
            if slot == 0 {
                let offset = self.arena.len() as u32;
                let len = name.len() as u16;
                self.arena.extend_from_slice(name);
                self.items.push((country_id, offset, len));
                let item_idx = self.items.len() as u32;
                self.slots[idx] = item_idx;
                if self.items.len() * 2 > self.slots.len() {
                    self.grow();
                }
                return true;
            } else {
                let item_idx = (slot - 1) as usize;
                let (cid, off, len) = self.items[item_idx];
                if cid == country_id && len as usize == name.len() {
                    let existing = &self.arena[off as usize..(off as usize + len as usize)];
                    if existing == name {
                        return false;
                    }
                }
                idx = (idx + 1) & self.mask;
            }
        }
    }

    fn grow(&mut self) {
        let new_cap = self.slots.len() * 2;
        let new_mask = new_cap - 1;
        let mut new_slots = vec![0u32; new_cap];
        for (i, &(cid, off, len)) in self.items.iter().enumerate() {
            let name = &self.arena[off as usize..(off as usize + len as usize)];
            let mut h: u64 = 0x517cc1b727220a95 ^ (cid as u64);
            for &b in name {
                h = (h.rotate_left(5) ^ (b as u64)).wrapping_mul(0x517cc1b727220a95);
            }
            let mut idx = (h as usize) & new_mask;
            while new_slots[idx] != 0 {
                idx = (idx + 1) & new_mask;
            }
            new_slots[idx] = (i + 1) as u32;
        }
        self.slots = new_slots;
        self.mask = new_mask;
    }

    /// Iterate all interned (country_id, name_bytes) pairs.
    fn iter_items(&self) -> impl Iterator<Item = (u8, &[u8])> {
        self.items.iter().map(|&(cid, off, len)| {
            (cid, &self.arena[off as usize..(off as usize + len as usize)])
        })
    }
}

struct CountryAgg {
    count: u64,
    id_sum: u64,
    amount_sum: i64,
    amount_min: i64,
    amount_max: i64,
    name_bytes: u64,
    note_bytes: u64,
}

impl Default for CountryAgg {
    fn default() -> Self {
        Self {
            count: 0,
            id_sum: 0,
            amount_sum: 0,
            amount_min: i64::MAX,
            amount_max: i64::MIN,
            name_bytes: 0,
            note_bytes: 0,
        }
    }
}

fn format_cents(buf: &mut [u8], cents: i64, mut pos: usize) -> usize {
    if cents == 0 {
        buf[pos..pos + 4].copy_from_slice(b"0.00");
        return pos + 4;
    }
    let ucents = if cents < 0 {
        buf[pos] = b'-';
        pos += 1;
        (-cents) as u64
    } else {
        cents as u64
    };
    let whole = ucents / 100;
    let frac = ucents % 100;
    pos = append_u64(buf, whole, pos);
    buf[pos] = b'.';
    pos += 1;
    buf[pos] = b'0' + (frac / 10) as u8;
    buf[pos + 1] = b'0' + (frac % 10) as u8;
    pos + 2
}

fn append_u64(buf: &mut [u8], mut val: u64, mut pos: usize) -> usize {
    if val == 0 {
        buf[pos] = b'0';
        return pos + 1;
    }
    let mut temp = [0u8; 24];
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

/// Process a chunk `data[from..to]` of records.csv.
/// `is_first_chunk` means we need to skip the CSV header line.
fn process_chunk(
    data: &[u8],
    from: usize,
    to: usize,
    region_to_country: &[u8],
    num_countries: usize,
    is_first_chunk: bool,
) -> (Vec<CountryAgg>, DistinctNameTable) {
    let mut aggs: Vec<CountryAgg> = (0..num_countries).map(|_| CountryAgg::default()).collect();
    let mut distinct_table = DistinctNameTable::with_capacity(65_536);
    let mut name_buf = Vec::with_capacity(256);

    let mut pos = from;

    if is_first_chunk {
        // Skip optional BOM.
        if pos + 3 <= to && data[pos..pos + 3] == [0xEF, 0xBB, 0xBF] {
            pos += 3;
        }
        // Skip header line.
        while pos < to && data[pos] != b'\n' {
            pos += 1;
        }
        if pos < to {
            pos += 1;
        }
    }

    while pos < to {
        if data[pos] == b'\r' || data[pos] == b'\n' {
            pos += 1;
            continue;
        }
        // Optional BOM at record boundary.
        if pos + 3 <= to && data[pos..pos + 3] == [0xEF, 0xBB, 0xBF] {
            pos += 3;
            if pos >= to {
                break;
            }
        }

        // Field 1: id
        let mut id = 0u64;
        while pos < to && data[pos] >= b'0' && data[pos] <= b'9' {
            id = id * 10 + (data[pos] - b'0') as u64;
            pos += 1;
        }
        if pos < to && data[pos] == b',' {
            pos += 1;
        }

        // Field 2: name (quoted or unquoted)
        name_buf.clear();
        if pos < to && data[pos] == b'"' {
            pos += 1;
            loop {
                let end = pos + find_quote(&data[pos..to]);
                name_buf.extend_from_slice(&data[pos..end]);
                pos = end;
                if pos >= to {
                    break;
                }
                pos += 1; // skip the '"'
                if pos < to && data[pos] == b'"' {
                    name_buf.push(b'"'); // escaped quote
                    pos += 1;
                } else {
                    break; // closing quote
                }
            }
        } else {
            let start = pos;
            while pos < to && data[pos] != b',' {
                pos += 1;
            }
            name_buf.extend_from_slice(&data[start..pos]);
        }
        if pos < to && data[pos] == b',' {
            pos += 1;
        }

        // Field 3: note (byte count only)
        let mut note_len = 0u64;
        if pos < to && data[pos] == b'"' {
            pos += 1;
            loop {
                let end = pos + find_quote(&data[pos..to]);
                note_len += (end - pos) as u64;
                pos = end;
                if pos >= to {
                    break;
                }
                pos += 1;
                if pos < to && data[pos] == b'"' {
                    note_len += 1;
                    pos += 1;
                } else {
                    break;
                }
            }
        } else {
            let start = pos;
            while pos < to && data[pos] != b',' {
                pos += 1;
            }
            note_len = (pos - start) as u64;
        }
        if pos < to && data[pos] == b',' {
            pos += 1;
        }

        // Field 4: region_id
        let mut region_id = 0usize;
        while pos < to && data[pos] >= b'0' && data[pos] <= b'9' {
            region_id = region_id * 10 + (data[pos] - b'0') as usize;
            pos += 1;
        }
        if pos < to && data[pos] == b',' {
            pos += 1;
        }

        // Field 5: amount (in cents)
        let mut neg = false;
        if pos < to && data[pos] == b'-' {
            neg = true;
            pos += 1;
        }
        let mut whole = 0i64;
        while pos < to && data[pos] >= b'0' && data[pos] <= b'9' {
            whole = whole * 10 + (data[pos] - b'0') as i64;
            pos += 1;
        }
        let mut frac = 0i64;
        if pos < to && data[pos] == b'.' {
            pos += 1;
            if pos < to && data[pos] >= b'0' && data[pos] <= b'9' {
                frac = (data[pos] - b'0') as i64 * 10;
                pos += 1;
            }
            if pos < to && data[pos] >= b'0' && data[pos] <= b'9' {
                frac += (data[pos] - b'0') as i64;
                pos += 1;
            }
        }
        let mut cents = whole * 100 + frac;
        if neg {
            cents = -cents;
        }

        // Skip CRLF or LF.
        if pos < to && data[pos] == b'\r' {
            pos += 1;
        }
        if pos < to && data[pos] == b'\n' {
            pos += 1;
        }

        // Country lookup.
        let country_id = if region_id < region_to_country.len() {
            region_to_country[region_id] as usize
        } else {
            0
        };

        let target = &mut aggs[country_id];
        target.count += 1;
        target.id_sum += id;
        target.amount_sum += cents;
        if cents < target.amount_min {
            target.amount_min = cents;
        }
        if cents > target.amount_max {
            target.amount_max = cents;
        }
        target.name_bytes += name_buf.len() as u64;
        target.note_bytes += note_len;

        distinct_table.insert_if_absent(country_id as u8, &name_buf);
    }

    (aggs, distinct_table)
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

    let records_path = data_dir.join("records.csv");
    let regions_path = data_dir.join("regions.csv");
    let out_path = out_dir.join("aggregates.txt");

    if !records_path.exists() || !regions_path.exists() {
        File::create(&out_path)?;
        return Ok(());
    }

    // 1. Load regions.csv (small; single-threaded).
    let mut regions_file = File::open(&regions_path)?;
    let reg_len = regions_file.metadata()?.len() as usize;
    let mut regions_bytes = Vec::with_capacity(reg_len + 8);
    regions_file.read_to_end(&mut regions_bytes)?;

    let mut region_to_country = vec![0u8; 10_000_000];
    let mut country_names: Vec<String> = vec!["_UNMATCHED".to_string()];

    let mut rpos = 0;
    while rpos < regions_bytes.len() && regions_bytes[rpos] != b'\n' {
        rpos += 1;
    }
    if rpos < regions_bytes.len() {
        rpos += 1;
    }
    while rpos < regions_bytes.len() {
        if regions_bytes[rpos] == b'\r' || regions_bytes[rpos] == b'\n' {
            rpos += 1;
            continue;
        }
        let mut reg_id = 0usize;
        while rpos < regions_bytes.len()
            && regions_bytes[rpos] >= b'0'
            && regions_bytes[rpos] <= b'9'
        {
            reg_id = reg_id * 10 + (regions_bytes[rpos] - b'0') as usize;
            rpos += 1;
        }
        if rpos < regions_bytes.len() && regions_bytes[rpos] == b',' {
            rpos += 1;
        }
        let code_start = rpos;
        while rpos < regions_bytes.len()
            && regions_bytes[rpos] >= b'A'
            && regions_bytes[rpos] <= b'Z'
        {
            rpos += 1;
        }
        let code_bytes = &regions_bytes[code_start..rpos];
        let code_str = std::str::from_utf8(code_bytes)?.to_string();
        let country_id = match country_names.iter().position(|c| c == &code_str) {
            Some(idx) => idx as u8,
            None => {
                country_names.push(code_str);
                (country_names.len() - 1) as u8
            }
        };
        if reg_id < region_to_country.len() {
            region_to_country[reg_id] = country_id;
        }
        while rpos < regions_bytes.len() && regions_bytes[rpos] != b'\n' {
            rpos += 1;
        }
        if rpos < regions_bytes.len() {
            rpos += 1;
        }
    }
    drop(regions_bytes);

    let num_countries = country_names.len();

    // 2. Load records.csv into memory and split across threads.
    //    The generator never writes CR inside quoted fields, so every '\n' is
    //    a safe record boundary when treated as a split point.
    let mut records_file = File::open(&records_path)?;
    let file_size = records_file.metadata()?.len() as usize;
    let mut file_data = Vec::with_capacity(file_size + 1);
    records_file.read_to_end(&mut file_data)?;
    drop(records_file);

    let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(6)
        .min(12)
        .max(1);

    // Find chunk boundaries aligned to CRLF positions.
    // The generator never writes CR inside quoted fields, so every "\r\n"
    // marks the end of a complete record.
    let mut boundaries = Vec::with_capacity(num_threads + 1);
    boundaries.push(0usize);
    for t in 1..num_threads {
        let approx = t * file_size / num_threads;
        let mut bp = approx.max(boundaries[t - 1]);
        while bp + 1 < file_size && !(file_data[bp] == b'\r' && file_data[bp + 1] == b'\n') {
            bp += 1;
        }
        if bp + 1 < file_size {
            boundaries.push(bp + 2); // start right after "\r\n"
        } else {
            boundaries.push(file_size);
        }
    }
    boundaries.push(file_size);

    let file_data = Arc::new(file_data);
    let region_to_country = Arc::new(region_to_country);

    let mut handles = Vec::with_capacity(num_threads);
    for t in 0..num_threads {
        let from = boundaries[t];
        let to = boundaries[t + 1];
        if from >= to {
            continue;
        }
        let data = Arc::clone(&file_data);
        let rtc = Arc::clone(&region_to_country);
        let is_first = t == 0;
        handles.push(thread::spawn(move || {
            process_chunk(&data, from, to, &rtc, num_countries, is_first)
        }));
    }

    let mut thread_results: Vec<(Vec<CountryAgg>, DistinctNameTable)> = Vec::new();
    for h in handles {
        thread_results.push(h.join().expect("Worker thread panicked"));
    }
    drop(file_data);
    drop(region_to_country);

    // 3. Merge numeric aggregates.
    let mut aggs: Vec<CountryAgg> = (0..num_countries).map(|_| CountryAgg::default()).collect();
    for (thread_aggs, _) in &thread_results {
        for (k, ta) in thread_aggs.iter().enumerate() {
            let a = &mut aggs[k];
            a.count += ta.count;
            a.id_sum += ta.id_sum;
            a.amount_sum += ta.amount_sum;
            if ta.count > 0 && ta.amount_min < a.amount_min {
                a.amount_min = ta.amount_min;
            }
            if ta.count > 0 && ta.amount_max > a.amount_max {
                a.amount_max = ta.amount_max;
            }
            a.name_bytes += ta.name_bytes;
            a.note_bytes += ta.note_bytes;
        }
    }

    // 4. Merge distinct-name tables: global deduplication via a second pass.
    //    Each thread saw its chunk; the same (country_id, name) can appear in
    //    multiple threads. We insert into a global table to count unique pairs.
    let mut global_distinct = DistinctNameTable::with_capacity(262_144);
    let mut distinct_counts: Vec<u64> = vec![0u64; num_countries];
    for (_, local_table) in &thread_results {
        for (cid, name) in local_table.iter_items() {
            if global_distinct.insert_if_absent(cid, name) {
                distinct_counts[cid as usize] += 1;
            }
        }
    }
    drop(thread_results);
    drop(global_distinct);

    // 5. Sort and write output.
    let mut order: Vec<usize> = (0..num_countries)
        .filter(|&k| aggs[k].count > 0)
        .collect();
    order.sort_unstable_by(|&a, &b| country_names[a].cmp(&country_names[b]));

    std::fs::create_dir_all(&out_dir).ok();
    let mut writer = BufWriter::with_capacity(1024 * 1024, File::create(&out_path)?);
    let mut line_buf = [0u8; 512];

    for idx in order {
        let code = country_names[idx].as_bytes();
        let item = &aggs[idx];

        let mut p = 0;
        line_buf[p..p + code.len()].copy_from_slice(code);
        p += code.len();
        line_buf[p] = b' ';
        p += 1;

        p = append_u64(&mut line_buf, item.count, p);
        line_buf[p] = b' ';
        p += 1;

        p = append_u64(&mut line_buf, item.id_sum, p);
        line_buf[p] = b' ';
        p += 1;

        p = format_cents(&mut line_buf, item.amount_sum, p);
        line_buf[p] = b' ';
        p += 1;

        p = format_cents(&mut line_buf, item.amount_min, p);
        line_buf[p] = b' ';
        p += 1;

        p = format_cents(&mut line_buf, item.amount_max, p);
        line_buf[p] = b' ';
        p += 1;

        p = append_u64(&mut line_buf, distinct_counts[idx], p);
        line_buf[p] = b' ';
        p += 1;

        p = append_u64(&mut line_buf, item.name_bytes, p);
        line_buf[p] = b' ';
        p += 1;

        p = append_u64(&mut line_buf, item.note_bytes, p);
        line_buf[p] = b'\n';
        p += 1;

        writer.write_all(&line_buf[..p])?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_scan_covers_unaligned_inputs_and_vector_tails() {
        for padding in 0..32 {
            for len in 0..129 {
                let mut bytes = vec![b'x'; padding + len];
                assert_eq!(find_quote(&bytes[padding..]), len);
                for at in 0..len {
                    bytes[padding + at] = b'"';
                    assert_eq!(find_quote(&bytes[padding..]), at);
                    bytes[padding + at] = b'x';
                }
            }
        }
    }

    #[test]
    fn distinct_table_collision_free_exact_bytes() {
        let mut t = DistinctNameTable::with_capacity(64);
        assert!(t.insert_if_absent(1, b"alice"));
        assert!(!t.insert_if_absent(1, b"alice")); // same country + same name
        assert!(t.insert_if_absent(2, b"alice")); // different country
        assert!(t.insert_if_absent(1, b"Alice")); // different bytes (case-sensitive)
    }
}
