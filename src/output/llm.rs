use crate::search::FileMatches;

pub fn print(results: &[FileMatches], files_only: bool, count_only: bool) {
    if files_only {
        for file_match in results {
            println!("{}", &file_match.path);
        }
        return;
    }

    if count_only {
        for file_match in results {
            println!("{}:{}", file_match.path, file_match.matches.len());
        }
        return;
    }

    let total: usize = results.iter().map(|r| r.matches.len()).sum();

    if total == 0 {
        return;
    }

    let mut current_file = String::new();

    for file_match in results {
        for m in &file_match.matches {
            if m.path != current_file {
                if !current_file.is_empty() {
                    println!();
                }
                current_file = m.path.clone();
                println!("--- {}", current_file);
            }

            let content = if m.line.len() > 200 {
                format!("{}...", &m.line[..200])
            } else {
                m.line.clone()
            };

            println!("{}:{}", m.line_number, content);
        }
    }

    println!("\n--- {} matches in {} files", total, results.len());
}
