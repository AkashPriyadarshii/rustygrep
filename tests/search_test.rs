mod common;

use common::{make_cli, setup_repo};
use rustygrep::search::SearchEngine;
use rustygrep::walker::FileWalker;

#[test]
fn basic_match() {
    let dir = setup_repo();
    let cli = make_cli(&["fn", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.path.contains("main.rs")));
}

#[test]
fn case_insensitive() {
    let dir = setup_repo();
    let cli = make_cli(&["-i", "Fn", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    assert!(!results.is_empty());
}

#[test]
fn whole_word_match() {
    let dir = setup_repo();
    let cli = make_cli(&["-w", "fn", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    let all_lines: Vec<&str> = results
        .iter()
        .flat_map(|r| r.matches.iter().map(|m| m.line.as_str()))
        .collect();
    for line in all_lines {
        assert!(
            line.contains(" fn ") || line.starts_with("fn "),
            "Whole word match failed for: {}",
            line
        );
    }
}

#[test]
fn invert_match() {
    let dir = setup_repo();
    let cli = make_cli(&["-v", "fn", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    for r in &results {
        for m in &r.matches {
            assert!(
                !m.line.contains("fn"),
                "Inverted match should not contain 'fn': {}",
                m.line
            );
        }
    }
}

#[test]
fn no_results_returns_empty() {
    let dir = setup_repo();
    let cli = make_cli(&["zzz_nonexistent_pattern_zzz", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    assert!(results.is_empty());
}

#[test]
fn submatch_positions_correct() {
    let dir = setup_repo();
    let cli = make_cli(&["error", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    for r in &results {
        for m in &r.matches {
            for &(start, end) in &m.submatches {
                let matched = &m.line[start..end];
                assert_eq!(matched, "error", "Submatch position mismatch");
            }
        }
    }
}
