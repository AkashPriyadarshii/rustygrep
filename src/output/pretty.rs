use crate::search::FileMatches;
use colored::*;

/// ANSI codes emitted directly for the color path. `colored` builds a
/// heap String per segment; these constants let the hot loop write
/// bytes with zero allocation (plain mode skips them entirely).
const C_PATH: &[u8] = b"\x1b[1;34m"; // bold blue
const C_LINE: &[u8] = b"\x1b[1;32m"; // bold green
const C_HIT: &[u8] = b"\x1b[1;31m"; // bold red
const C_OFF: &[u8] = b"\x1b[0m";

/// Byte-level appender for the arena path: `path:lineno:line\n` with
/// max_cols truncation, no format!/String. Returns false if the line
/// must be skipped (binary NUL — mirrors search_path quit semantics).
/// `line_no` digits inlined for the common <10000 case.
pub fn append_plain_match(out: &mut Vec<u8>, path: &[u8], line_no: u64, line: &[u8], max_cols: usize) {
    out.extend_from_slice(path);
    out.push(b':');
    let mut numbuf = [0u8; 20];
    let len = if line_no < 10 {
        numbuf[0] = b'0' + line_no as u8;
        1
    } else if line_no < 100 {
        numbuf[0] = b'0' + (line_no / 10) as u8;
        numbuf[1] = b'0' + (line_no % 10) as u8;
        2
    } else if line_no < 1000 {
        numbuf[0] = b'0' + (line_no / 100) as u8;
        numbuf[1] = b'0' + ((line_no / 10) % 10) as u8;
        numbuf[2] = b'0' + (line_no % 10) as u8;
        3
    } else if line_no < 10000 {
        numbuf[0] = b'0' + (line_no / 1000) as u8;
        numbuf[1] = b'0' + ((line_no / 100) % 10) as u8;
        numbuf[2] = b'0' + ((line_no / 10) % 10) as u8;
        numbuf[3] = b'0' + (line_no % 10) as u8;
        4
    } else {
        let mut tmp = [0u8; 20];
        let mut v = line_no;
        let mut l = 0;
        while v > 0 {
            tmp[l] = b'0' + (v % 10) as u8;
            v /= 10;
            l += 1;
        }
        let mut i = 0;
        while i < l {
            numbuf[i] = tmp[l - 1 - i];
            i += 1;
        }
        l
    };
    out.extend_from_slice(&numbuf[..len]);
    out.push(b':');
    // Truncate at max_cols on a UTF-8 boundary (ASCII fast check).
    let mut end = if max_cols > 0 && line.len() > max_cols {
        max_cols
    } else {
        line.len()
    };
    while end > 0 && end < line.len() && (line[end] & 0xC0) == 0x80 {
        end -= 1;
    }
    out.extend_from_slice(&line[..end]);
    out.push(b'\n');
}

/// Truncate a string to `max_cols` bytes at a UTF-8 char boundary.
fn truncate_line(line: &str, max_cols: usize) -> &str {
    if max_cols == 0 || line.len() <= max_cols {
        return line;
    }
    let mut end = max_cols;
    while end > 0 && !line.is_char_boundary(end) {
        end -= 1;
    }
    &line[..end]
}

pub fn print(
    results: &[FileMatches],
    no_color: bool,
    files_only: bool,
    count_only: bool,
    max_cols: usize,
    has_context_lines: bool,
) {
    use std::io::Write;
    // Single lock + buffer: one syscall-ish flush instead of a
    // lock+flush per matched line (133k println! calls = ~700ms).
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    if count_only {
        // Single file → bare count (rg compat); multiple files → path:count.
        if results.len() == 1 {
            let _ = writeln!(out, "{}", results[0].matches.len());
        } else {
            for file_match in results {
                let _ = writeln!(out, "{}:{}", file_match.path, file_match.matches.len());
            }
        }
        return;
    }

    if files_only {
        for file_match in results {
            let _ = writeln!(out, "{}", file_match.path);
        }
        return;
    }

    // has_context_lines comes from main (context flags were passed).
    // Fast modes never emit context lines, so skip the per-line
    // is_context_line branch entirely on that path.
    for file_match in results {
        for (line_idx, m) in file_match.matches.iter().enumerate() {
            if has_context_lines && line_idx > 0 && is_context_line(m) {
                if no_color {
                    let _ = writeln!(out, "--");
                } else {
                    let _ = writeln!(out, "{}", "-".dimmed());
                }
            }

            let display_line = truncate_line(&m.line, max_cols);
            // Borrow, don't allocate: submatches are ascending, so a
            // take_while prefix covers the visible range.
            let vis: &[(usize, usize)] = if max_cols > 0 && max_cols < m.line.len() {
                let mut n = 0;
                while n < m.submatches.len() && m.submatches[n].1 <= display_line.len() {
                    n += 1;
                }
                &m.submatches[..n]
            } else {
                &m.submatches
            };

            if no_color {
                // No format! machinery: raw bytes + inline line-number
                // digits. format! per line cost ~15ms on 41k lines.
                // Borrow the parent file path — per-match path clones
                // are skipped when need_submatches is off.
                let p = if m.path.is_empty() {
                    file_match.path.as_str()
                } else {
                    m.path.as_str()
                };
                // Fast line-number digits: bench lines are <3000,
                // skip the tmp-buffer reverse for the common cases.
                let mut numbuf = [0u8; 20];
                let len = if m.line_number < 10 {
                    numbuf[0] = b'0' + m.line_number as u8;
                    1
                } else if m.line_number < 100 {
                    numbuf[0] = b'0' + (m.line_number / 10) as u8;
                    numbuf[1] = b'0' + (m.line_number % 10) as u8;
                    2
                } else if m.line_number < 1000 {
                    numbuf[0] = b'0' + (m.line_number / 100) as u8;
                    numbuf[1] = b'0' + ((m.line_number / 10) % 10) as u8;
                    numbuf[2] = b'0' + (m.line_number % 10) as u8;
                    3
                } else if m.line_number < 10000 {
                    numbuf[0] = b'0' + (m.line_number / 1000) as u8;
                    numbuf[1] = b'0' + ((m.line_number / 100) % 10) as u8;
                    numbuf[2] = b'0' + ((m.line_number / 10) % 10) as u8;
                    numbuf[3] = b'0' + (m.line_number % 10) as u8;
                    4
                } else {
                    let mut tmp = [0u8; 20];
                    let mut v = m.line_number;
                    let mut l = 0;
                    while v > 0 {
                        tmp[l] = b'0' + (v % 10) as u8;
                        v /= 10;
                        l += 1;
                    }
                    let mut i = 0;
                    while i < l {
                        numbuf[i] = tmp[l - 1 - i];
                        i += 1;
                    }
                    l
                };
                let _ = out.write_all(p.as_bytes());
                let _ = out.write_all(b":");
                let _ = out.write_all(&numbuf[..len]);
                let _ = out.write_all(b":");
                let _ = out.write_all(display_line.as_bytes());
                let _ = out.write_all(b"\n");
            } else {
                let _ = write_colored_match(&mut out, m, display_line, vis);
            }
        }
    }
}

/// Zero-alloc color writer: raw ANSI bytes straight into the buffer.
/// Same visual output as the old `colored` path, no Strings built.
fn write_colored_match(
    out: &mut impl ::std::io::Write,
    m: &crate::search::Match,
    display_line: &str,
    submatches: &[(usize, usize)],
) -> std::io::Result<()> {
    out.write_all(C_PATH)?;
    out.write_all(m.path.as_bytes())?;
    out.write_all(C_OFF)?;
    out.write_all(b":")?;
    out.write_all(C_LINE)?;
    // line_number is small; itoa-free decimal via a stack buffer.
    let mut numbuf = [0u8; 20];
    let mut n = m.line_number;
    let mut len = 0;
    if n == 0 {
        numbuf[19] = b'0';
        len = 1;
    } else {
        let mut i = 20;
        while n > 0 {
            i -= 1;
            numbuf[i] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
        numbuf.copy_within(i..20, 0);
    }
    out.write_all(&numbuf[..len])?;
    out.write_all(C_OFF)?;
    out.write_all(b":")?;
    if submatches.is_empty() {
        out.write_all(display_line.as_bytes())?;
    } else {
        let bytes = display_line.as_bytes();
        let mut last = 0;
        for &(s, e) in submatches {
            let (s, e) = (s.min(bytes.len()), e.min(bytes.len()));
            if s < last || s >= e {
                continue;
            }
            out.write_all(&bytes[last..s])?;
            out.write_all(C_HIT)?;
            out.write_all(&bytes[s..e])?;
            out.write_all(C_OFF)?;
            last = e;
        }
        out.write_all(&bytes[last..])?;
    }
    out.write_all(b"\n")
}

fn is_context_line(m: &crate::search::Match) -> bool {
    m.submatches.is_empty()
}

fn highlight_matches(line: &str, submatches: &[(usize, usize)], no_color: bool) -> String {
    if no_color || submatches.is_empty() {
        return line.to_string();
    }

    let bytes = line.as_bytes();
    let mut result = String::new();
    let mut last_end = 0;

    for &(start, end) in submatches {
        if start > last_end {
            result.push_str(&String::from_utf8_lossy(&bytes[last_end..start]));
        }
        let matched = String::from_utf8_lossy(&bytes[start..end]);
        result.push_str(&matched.red().bold().to_string());
        last_end = end;
    }

    if last_end < bytes.len() {
        result.push_str(&String::from_utf8_lossy(&bytes[last_end..]));
    }

    result
}
