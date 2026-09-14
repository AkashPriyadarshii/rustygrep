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
) {
    match format {
        crate::cli::OutputFormat::Pretty => {
            pretty::print(results, no_color, files_only, count_only, max_cols)
        }
        crate::cli::OutputFormat::Json => json::print(results, files_only, count_only, json_file),
        crate::cli::OutputFormat::Llm => llm::print(results, files_only, count_only, llm_opts),
    }
}
