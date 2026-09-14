use crate::search::FileMatches;
use colored::*;

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
) {
    if count_only {
        // Single file → bare count (rg compat); multiple files → path:count.
        if results.len() == 1 {
            println!("{}", results[0].matches.len());
        } else {
            for file_match in results {
                println!("{}:{}", file_match.path, file_match.matches.len());
            }
        }
        return;
    }

    if files_only {
        for file_match in results {
            println!("{}", file_match.path);
        }
        return;
    }

    for file_match in results {
        for (line_idx, m) in file_match.matches.iter().enumerate() {
            if line_idx > 0 && is_context_line(m) {
                if no_color {
                    println!("--");
                } else {
                    println!("{}", "-".dimmed());
                }
            }

            let display_line = truncate_line(&m.line, max_cols);
            let display_submatches: Vec<(usize, usize)> = if max_cols > 0 {
                m.submatches
                    .iter()
                    .copied()
                    .filter(|&(_, e)| e <= display_line.len())
                    .collect()
            } else {
                m.submatches.clone()
            };

            if no_color {
                println!("{}:{}:{}", m.path, m.line_number, display_line);
            } else {
                let path = m.path.blue().bold();
                let line_num = m.line_number.to_string().green().bold();
                let line = highlight_matches(display_line, &display_submatches, no_color);
                println!("{}:{}:{}", path, line_num, line);
            }
        }
    }
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
