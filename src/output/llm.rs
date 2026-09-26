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
    use std::io::Write;
    if files_only {
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::with_capacity(1 << 16, stdout.lock());
        for file_match in results {
            let _ = writeln!(out, "{}", file_match.path);
        }
        return;
    }

    if count_only {
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::with_capacity(1 << 16, stdout.lock());
        for file_match in results {
            let _ = writeln!(out, "{}:{}", file_match.path, file_match.matches.len());
        }
        return;
    }

    let total: usize = results.iter().map(|r| r.matches.len()).sum();

    if total == 0 {
        return;
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    // Stream per line: skip the whole-output String (13MB double-copy).
    let mut current_file = String::new();
    let mut streamed = 0usize;
    // Budget mode: count chars as we stream, stop at the cap.
    let mut budget_chars: Option<usize> = opts.budget_tokens.map(|b| b * 4);
    macro_rules! emit {
        ($s:expr) => {{
            let s: &str = $s;
            if let Some(ref mut left) = budget_chars {
                let n = s.chars().count();
                if n >= *left {
                    let take = *left;
                    let part: String = s.chars().take(take).collect();
                    let _ = write!(out, "{}...", part);
                    let _ = out.flush();
                    return;
                }
                *left -= n;
            }
            let _ = write!(out, "{}", s);
        }};
    }

    for file_match in results {
        for m in &file_match.matches {
            streamed += 1;
            if m.path != current_file {
                if !current_file.is_empty() {
                    emit!("\n");
                }
                current_file = m.path.clone();
                emit!(&format!(
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

            // Truncate by byte window without cloning the full line.
            let content: &str = if opts.truncate && m.line.len() > opts.max_line_chars {
                let mut end = opts.max_line_chars;
                while end > 0 && !m.line.is_char_boundary(end) {
                    end -= 1;
                }
                &m.line[..end]
            } else {
                &m.line
            };
            let dots = if opts.truncate && m.line.len() > opts.max_line_chars {
                "..."
            } else {
                ""
            };
            emit!(&format!("{}:{}{}\n", m.line_number, content, dots));
        }
    }
    let _ = streamed;

    emit!(&format!(
        "\n--- {} match{} in {} file{}\n",
        total,
        if total == 1 { "" } else { "es" },
        results.len(),
        if results.len() == 1 { "" } else { "s" }
    ));
}
