use crate::search::FileMatches;
use serde::Serialize;

#[derive(Serialize)]
struct JsonMatch {
    path: String,
    line: u64,
    match_text: String,
    submatches: Vec<(usize, usize)>,
}

pub fn print(results: &[FileMatches], files_only: bool, count_only: bool, json_file: bool) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    if files_only {
        for file_match in results {
            let _ = writeln!(out, "{}", file_match.path);
        }
        return;
    }

    if count_only {
        for file_match in results {
            // No Value tree: fixed keys as bytes, serde only escapes.
            let _ = out.write_all(COUNT_OPEN);
            let _ = serde_json::to_writer(&mut out, &file_match.path);
            let _ = write!(out, ",\"count\":{}}}\n", file_match.matches.len());
        }
        return;
    }

    if json_file {
        print_per_file(results);
    } else {
        print_per_match(results);
    }
}

// Fixed JSON keys as byte constants (no per-hit format!/Value).
const COUNT_OPEN: &[u8] = b"{\"path\":";
const MATCH_OPEN: &[u8] = b"{\"path\":";
const KEY_LINE: &[u8] = b",\"line\":";
const KEY_TEXT: &[u8] = b",\"match_text\":";
const KEY_SUBS: &[u8] = b",\"submatches\":[";

/// Fixed-key writer: serde_json escapes strings only, numbers via
/// stack digits, keys as constants. Kills the per-hit `json!` Value
/// tree + its allocs (the 117ms JSON path's main cost).
fn write_json_match(
    out: &mut impl std::io::Write,
    path: &str,
    line_no: u64,
    text: &str,
    submatches: &[(usize, usize)],
) {
    use std::io::Write;
    let _ = out.write_all(MATCH_OPEN);
    let _ = serde_json::to_writer(&mut *out, &path);
    let _ = out.write_all(KEY_LINE);
    let mut numbuf = [0u8; 20];
    let mut v = line_no;
    let mut len = 0;
    if v == 0 {
        numbuf[0] = b'0';
        len = 1;
    } else {
        let mut tmp = [0u8; 20];
        while v > 0 {
            tmp[len] = b'0' + (v % 10) as u8;
            v /= 10;
            len += 1;
        }
        let mut i = 0;
        while i < len {
            numbuf[i] = tmp[len - 1 - i];
            i += 1;
        }
    }
    let _ = out.write_all(&numbuf[..len]);
    let _ = out.write_all(KEY_TEXT);
    let _ = serde_json::to_writer(&mut *out, &text);
    let _ = out.write_all(KEY_SUBS);
    for (i, (s, e)) in submatches.iter().enumerate() {
        if i > 0 {
            let _ = out.write_all(b",");
        }
        let _ = write!(out, "[{},{}]", s, e);
    }
    let _ = out.write_all(b"]}\n");
}

fn print_per_match(results: &[FileMatches]) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    for file_match in results {
        for m in &file_match.matches {
            // Fall back to the parent file path when per-match clones
            // were skipped (need_submatches off for --json).
            let p = if m.path.is_empty() {
                file_match.path.as_str()
            } else {
                m.path.as_str()
            };
            write_json_match(&mut out, p, m.line_number, &m.line, &m.submatches);
        }
    }
}

fn print_per_file(results: &[FileMatches]) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    for file_match in results {
        // Same raw treatment: no Value tree per file.
        let _ = out.write_all(MATCH_OPEN);
        let _ = serde_json::to_writer(&mut out, &file_match.path);
        let _ = write!(
            out,
            ",\"total_matches\":{},\"matches\":[",
            file_match.matches.len()
        );
        for (i, m) in file_match.matches.iter().enumerate() {
            if i > 0 {
                let _ = out.write_all(b",");
            }
            let p = if m.path.is_empty() {
                file_match.path.as_str()
            } else {
                m.path.as_str()
            };
            let _ = out.write_all(MATCH_OPEN);
            let _ = serde_json::to_writer(&mut out, &p);
            let _ = out.write_all(KEY_LINE);
            let _ = write!(out, "{}", m.line_number);
            let _ = out.write_all(KEY_TEXT);
            let _ = serde_json::to_writer(&mut out, &m.line);
            let _ = out.write_all(KEY_SUBS);
            for (j, (s, e)) in m.submatches.iter().enumerate() {
                if j > 0 {
                    let _ = out.write_all(b",");
                }
                let _ = write!(out, "[{},{}]", s, e);
            }
            let _ = out.write_all(b"]}");
        }
        let _ = out.write_all(b"]}\n");
    }
}
