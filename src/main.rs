use clap::Parser;
use rustygrep::cli::{Cli, OutputFormat, SubCommand};
use rustygrep::{init, mcp, output, search, walker};
use std::process;
use std::time::Instant;

fn main() {
    let cli = Cli::parse();
    let start = Instant::now();

    // Handle subcommands
    match cli.subcommand {
        Some(SubCommand::Mcp) => {
            mcp::run();
            return;
        }
        Some(SubCommand::Init) => {
            init::run();
            return;
        }
        None => {}
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

    let walker = walker::FileWalker::new(
        cli.paths.clone(),
        cli.hidden,
        cli.no_ignore,
        cli.no_binary,
        cli.file_type.clone(),
        cli.file_type_not.clone(),
        cli.threads,
    );

    // Measured (20MB/60-file bench, this machine): streaming walk+search
    // == collect-then-search within noise (±5ms). Keep the streaming path
    // only where it preserves semantics — -l short-circuits per file
    // either way, so no mode regresses by routing through one path.
    let (mut results, seen) = if cli.files_with_matches && engine.is_files_only_fast() {
        engine.search_streaming(&walker)
    } else {
        let files = walker.walk();
        if files.is_empty() {
            process::exit(1);
        }
        let n = files.len();
        (engine.search(&files), n)
    };

    if seen == 0 && results.is_empty() {
        process::exit(1);
    }

    // Apply --rank BM25-lite scoring
    if cli.rank {
        search::rank_by_score(&mut results);
    }

    // Apply --top N ranking (rank first if --rank given; else sort by match count)
    if let Some(top_n) = cli.top {
        if top_n > 0 {
            if !cli.rank {
                results.sort_by_key(|b| std::cmp::Reverse(b.total_matches));
            }
            results.truncate(top_n);
        }
    }

    // Exit code + stats reflect the search, not the display filters.
    let has_matches = !results.is_empty() && results.iter().any(|r| !r.matches.is_empty());
    if cli.stats {
        let elapsed = start.elapsed();
        let total_matches: usize = results.iter().map(|r| r.total_matches).sum();
        let files_with_matches = results.iter().filter(|r| !r.matches.is_empty()).count();
        eprintln!(
            "{} files matched, {} total matches, {:.3}s",
            files_with_matches,
            total_matches,
            elapsed.as_secs_f64()
        );
    }

    // --context-only: hide match lines, show only surrounding context
    if cli.context_only {
        for r in &mut results {
            r.matches.retain(|m| m.submatches.is_empty());
        }
    }

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
        cli.max_columns,
    );

    if has_matches {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
