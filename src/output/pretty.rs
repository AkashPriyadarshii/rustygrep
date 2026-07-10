use crate::search::FileMatches;
use colored::*;

pub fn print(results: &[FileMatches], no_color: bool, files_only: bool, count_only: bool) {
    let _total_matches: usize = results.iter().map(|r| r.matches.len()).sum();

    if count_only {
        if results.is_empty() {
            println!("0");
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

    for (file_idx, file_match) in results.iter().enumerate() {
        if file_idx > 0 {
            println!();
        }

        for (line_idx, m) in file_match.matches.iter().enumerate() {
            if line_idx > 0 && is_context_line(m, file_match) {
                if no_color {
                    println!("--");
                } else {
                    println!("{}", "-".dimmed());
                }
            }

            if no_color {
                println!("{}:{}:{}", m.path, m.line_number, m.line);
            } else {
                let path = m.path.blue().bold();
                let line_num = m.line_number.to_string().green().bold();
                let line = highlight_matches(&m.line, &m.submatches, no_color);
                println!("{}:{}:{}", path, line_num, line);
            }
        }
    }
}

fn is_context_line(m: &crate::search::Match, _file_match: &FileMatches) -> bool {
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
