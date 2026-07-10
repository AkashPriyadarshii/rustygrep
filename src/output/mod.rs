pub mod json;
pub mod llm;
pub mod pretty;

use crate::search::FileMatches;

pub fn print_results(
    results: &[FileMatches],
    format: &crate::cli::OutputFormat,
    no_color: bool,
    files_only: bool,
    count_only: bool,
) {
    match format {
        crate::cli::OutputFormat::Pretty => {
            pretty::print(results, no_color, files_only, count_only)
        }
        crate::cli::OutputFormat::Json => json::print(results, files_only, count_only),
        crate::cli::OutputFormat::Llm => llm::print(results, files_only, count_only),
    }
}
