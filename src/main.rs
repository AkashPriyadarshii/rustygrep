use clap::Parser;
use rustygrep::cli::{Cli, OutputFormat, SubCommand};
use rustygrep::{mcp, output, search, walker};
use std::process;

fn main() {
    let cli = Cli::parse();

    // Handle subcommands
    if let Some(SubCommand::Mcp) = cli.subcommand {
        mcp::run();
        return;
    }

    let _pattern = match &cli.pattern {
        Some(p) => p.clone(),
        None => {
            eprintln!("rustygrep: pattern is required");
            process::exit(2);
        }
    };

    let output_format = if cli.llm {
        OutputFormat::Llm
    } else if cli.json || cli.json_file {
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

    let mut results = results;

    // Apply --top N ranking
    if let Some(top_n) = cli.top {
        if top_n > 0 {
            results.sort_by_key(|b| std::cmp::Reverse(b.total_matches));
            results.truncate(top_n);
        }
    }

    let has_matches = !results.is_empty() && results.iter().any(|r| !r.matches.is_empty());

    let llm_opts = output::llm::LlmOptions {
        truncate: !cli.llm_no_truncate,
        max_line_chars: 120,
        budget_tokens: cli.llm_budget,
    };

    output::print_results(
        &results,
        &output_format,
        cli.no_color,
        cli.files_with_matches,
        cli.count,
        cli.json_file,
        &llm_opts,
    );

    if has_matches {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
