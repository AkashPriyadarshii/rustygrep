pub mod json;
pub mod llm;
pub mod pretty;

use crate::search::FileMatches;

#[allow(clippy::too_many_arguments)]
pub fn print_results(
    results: &[FileMatches],
    format: &crate::cli::OutputFormat,
    no_color: bool,
    files_only: bool,
    count_only: bool,
    json_file: bool,
    llm_opts: &llm::LlmOptions,
    max_cols: usize,
    has_context_lines: bool,
) {
    match format {
        crate::cli::OutputFormat::Pretty => pretty::print(
            results,
            no_color,
            files_only,
            count_only,
            max_cols,
            has_context_lines,
        ),
        crate::cli::OutputFormat::Json => json::print(results, files_only, count_only, json_file),
        crate::cli::OutputFormat::Llm => llm::print(results, files_only, count_only, llm_opts),
    }
}

/// Count-mode fast printer: works from (path, count) pairs, no
/// FileMatches needed. Pretty keeps rg compat (bare number for one
/// file, path:count otherwise); JSON/LLM keep their shapes.
/// Counts arrive pre-sorted by path from search_counts.
pub fn print_counts(
    counts: &[(String, usize)],
    format: &crate::cli::OutputFormat,
    llm_opts: &llm::LlmOptions,
    json_file: bool,
) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
    match format {
        crate::cli::OutputFormat::Pretty => {
            if counts.len() == 1 {
                let _ = writeln!(out, "{}", counts[0].1);
            } else {
                for (p, c) in counts {
                    let _ = writeln!(out, "{}:{}", p, c);
                }
            }
        }
        crate::cli::OutputFormat::Json => {
            if json_file {
                for (p, c) in counts {
                    let _ = write!(out, "{{\"path\":");
                    let _ = serde_json::to_writer(&mut out, &p);
                    let _ = write!(out, ",\"total_matches\":{},\"matches\":[]}}\n", c);
                }
            } else {
                for (p, c) in counts {
                    let _ = write!(out, "{{\"path\":");
                    let _ = serde_json::to_writer(&mut out, &p);
                    let _ = write!(out, ",\"count\":{}}}\n", c);
                }
            }
        }
        crate::cli::OutputFormat::Llm => {
            let total: usize = counts.iter().map(|c| c.1).sum();
            let _ = writeln!(out, "--- {} match{} in {} file{}", total, if total == 1 { "" } else { "es" }, counts.len(), if counts.len() == 1 { "" } else { "s" });
            for (p, c) in counts {
                let _ = writeln!(out, "{}:{}", p, c);
            }
            let _ = llm_opts.max_line_chars;
        }
    }
}
