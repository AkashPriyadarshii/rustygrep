mod cli;
mod output;
mod search;
mod walker;

use clap::Parser;
use cli::{Cli, OutputFormat};
use std::process;

fn main() {
    let cli = Cli::parse();

    let output_format = if cli.llm {
        OutputFormat::Llm
    } else if cli.json {
        OutputFormat::Json
    } else {
        cli.format.clone()
    };

    let engine = match search::SearchEngine::new(&cli) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("rustygrep: invalid pattern: {}", err);
            process::exit(2);
        }
    };

    let files = walker::FileWalker::new(
        cli.paths.clone(),
        cli.hidden,
        cli.no_ignore,
        cli.no_binary,
        cli.file_type.clone(),
        cli.file_type_not.clone(),
        cli.threads,
    )
    .walk();

    if files.is_empty() {
        process::exit(1);
    }

    let results = engine.search(&files);

    let has_matches = !results.is_empty()
        && results.iter().any(|r| !r.matches.is_empty());

    output::print_results(
        &results,
        &output_format,
        cli.no_color,
        cli.files_with_matches,
        cli.count,
    );

    if has_matches {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
