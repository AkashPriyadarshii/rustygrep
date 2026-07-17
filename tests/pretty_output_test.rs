mod common;

use common::{make_cli, setup_repo};
use rustygrep::search::SearchEngine;
use rustygrep::walker::FileWalker;

fn search_pretty(args: &[&str]) -> Vec<rustygrep::search::FileMatches> {
    let dir = setup_repo();
    let mut full_args = args.to_vec();
    full_args.push(dir.path().to_str().unwrap());

    let cli = make_cli(&full_args);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    engine.search(&files)
}

#[test]
fn pretty_results_contain_paths() {
    let results = search_pretty(&["fn"]);
    assert!(!results.is_empty());
    for r in &results {
        assert!(r.path.contains("src") || r.path.contains("tests"));
    }
}

#[test]
fn context_lines_before() {
    let results = search_pretty(&["-B", "1", "error_handler"]);
    for r in &results {
        for m in &r.matches {
            if m.submatches.is_empty() {
                // Context line — should exist before a match line
                continue;
            }
            assert!(!m.line.is_empty());
        }
    }
}

#[test]
fn context_lines_after() {
    let results = search_pretty(&["-A", "1", "fn main"]);
    assert!(!results.is_empty());
}

#[test]
fn count_only_mode() {
    let results = search_pretty(&["-c", "fn"]);
    assert!(!results.is_empty());
    for r in &results {
        assert!(r.matches.len() > 0);
        assert!(r.total_matches > 0);
    }
}

#[test]
fn files_only_mode() {
    let results = search_pretty(&["-l", "fn"]);
    assert!(!results.is_empty());
    for r in &results {
        assert!(!r.path.is_empty());
        assert!(r.matches.len() > 0);
    }
}
