use crate::search::FileMatches;
use serde::Serialize;

#[derive(Serialize)]
struct JsonMatch {
    path: String,
    line_number: u64,
    line: String,
    submatches: Vec<(usize, usize)>,
}

#[derive(Serialize)]
struct JsonFileResult {
    path: String,
    total_matches: usize,
    matches: Vec<JsonMatch>,
}

pub fn print(results: &[FileMatches], files_only: bool, count_only: bool) {
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

    for file_match in results {
        let json_result = JsonFileResult {
            path: file_match.path.clone(),
            total_matches: file_match.matches.len(),
            matches: file_match
                .matches
                .iter()
                .map(|m| JsonMatch {
                    path: m.path.clone(),
                    line_number: m.line_number,
                    line: m.line.clone(),
                    submatches: m.submatches.clone(),
                })
                .collect(),
        };

        println!("{}", serde_json::to_string(&json_result).unwrap());
    }
}
