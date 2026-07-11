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
    if files_only {
        for file_match in results {
            println!("{}", file_match.path);
        }
        return;
    }

    if count_only {
        for file_match in results {
            let output = serde_json::json!({
                "path": file_match.path,
                "count": file_match.matches.len(),
            });
            println!("{}", output);
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
    for file_match in results {
        for m in &file_match.matches {
            let output = JsonMatch {
                path: m.path.clone(),
                line: m.line_number,
                match_text: m.line.clone(),
                submatches: m.submatches.clone(),
            };
            println!("{}", serde_json::to_string(&output).unwrap());
        }
    }
}

fn print_per_file(results: &[FileMatches]) {
    for file_match in results {
        let output = serde_json::json!({
            "path": file_match.path,
            "total_matches": file_match.matches.len(),
            "matches": file_match.matches.iter().map(|m| serde_json::json!({
                "path": m.path,
                "line": m.line_number,
                "match_text": m.line,
                "submatches": m.submatches,
            })).collect::<Vec<_>>(),
        });
        println!("{}", output);
    }
}
