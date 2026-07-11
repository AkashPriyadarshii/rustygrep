use crate::search::FileMatches;

pub struct LlmOptions {
    pub truncate: bool,
    pub max_line_chars: usize,
    pub budget_tokens: Option<usize>,
}

impl Default for LlmOptions {
    fn default() -> Self {
        Self {
            truncate: true,
            max_line_chars: 120,
            budget_tokens: None,
        }
    }
}

pub fn print(results: &[FileMatches], files_only: bool, count_only: bool, opts: &LlmOptions) {
    if files_only {
        for file_match in results {
            println!("{}", file_match.path);
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

    let mut output = String::new();
    let mut current_file = String::new();

    for file_match in results {
        for m in &file_match.matches {
            if m.path != current_file {
                if !current_file.is_empty() {
                    output.push('\n');
                }
                current_file = m.path.clone();
                output.push_str(&format!(
                    "--- {} ({} match{})\n",
                    current_file,
                    file_match.matches.len(),
                    if file_match.matches.len() == 1 {
                        ""
                    } else {
                        "es"
                    }
                ));
            }

            let content = if opts.truncate && m.line.len() > opts.max_line_chars {
                let mut end = opts.max_line_chars;
                while end > 0 && !m.line.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &m.line[..end])
            } else {
                m.line.clone()
            };

            output.push_str(&format!("{}:{}\n", m.line_number, content));
        }
    }

    output.push_str(&format!(
        "\n--- {} match{} in {} file{}\n",
        total,
        if total == 1 { "" } else { "es" },
        results.len(),
        if results.len() == 1 { "" } else { "s" }
    ));

    if let Some(budget) = opts.budget_tokens {
        let char_budget = budget * 4;
        if output.len() > char_budget {
            let truncated: String = output.chars().take(char_budget).collect();
            println!("{}...", truncated);
            return;
        }
    }

    print!("{}", output);
}
