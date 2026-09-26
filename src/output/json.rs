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
            let output = serde_json::json!({
                "path": file_match.path,
                "count": file_match.matches.len(),
            });
            let _ = writeln!(out, "{}", output);
        }
        return;
    }

    if json_file {
        print_per_file(results);
    } else {
        print_per_match(results);
    }
}

fn print_per_match(results: &[FileMatches]) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    // Borrow line text into the serializer: no per-match String clones.
    #[derive(serde::Serialize)]
    struct JsonMatchRef<'a> {
        path: &'a str,
        line: u64,
        match_text: &'a str,
        submatches: &'a [(usize, usize)],
    }
    for file_match in results {
        for m in &file_match.matches {
            // Fall back to the parent file path when per-match clones
            // were skipped (need_submatches off for --json).
            let p = if m.path.is_empty() {
                file_match.path.as_str()
            } else {
                m.path.as_str()
            };
            let output = JsonMatchRef {
                path: p,
                line: m.line_number,
                match_text: &m.line,
                submatches: &m.submatches,
            };
            let _ = writeln!(out, "{}", serde_json::to_string(&output).unwrap());
        }
    }
}

fn print_per_file(results: &[FileMatches]) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    for file_match in results {
        let output = serde_json::json!({
            "path": file_match.path,
            "total_matches": file_match.matches.len(),
            "matches": file_match.matches.iter().map(|m| serde_json::json!({
                "path": if m.path.is_empty() { &file_match.path } else { &m.path },
                "line": m.line_number,
                "match_text": m.line,
                "submatches": m.submatches,
            })).collect::<Vec<_>>(),
        });
        let _ = writeln!(out, "{}", output);
    }
}
