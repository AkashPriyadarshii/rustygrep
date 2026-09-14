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

    assert!(
        results.iter().any(|r| !r.matches.is_empty()),
        "inverted search should find non-matching lines"
    );

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
fn zero_width_pattern_does_not_hang() {
    let dir = setup_repo();
    let cli = make_cli(&["\\b", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    assert!(!results.is_empty(), "zero-width pattern should match lines");
    for r in &results {
        for m in &r.matches {
            assert!(m.line_number > 0);
        }
    }
}

#[test]
fn context_lines_are_included() {
    let dir = setup_repo();
    let cli = make_cli(&["-C", "1", "error_handler", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    assert!(!results.is_empty());
    // The match line itself, plus its forward context line, must appear.
    let match_lines = results
        .iter()
        .flat_map(|r| r.matches.iter().map(|m| m.line.as_str()))
        .collect::<Vec<_>>();
    assert!(match_lines.iter().any(|l| l.contains("error_handler")));
    assert!(
        match_lines.len() > 1,
        "context lines missing: {:?}",
        match_lines
    );
    assert!(
        match_lines.iter().any(|l| l.contains("eprintln\"Error")
            || l.contains("eprintln!")
            || l.contains("msg: &str")),
        "forward context line missing: {:?}",
        match_lines
    );
}

#[test]
fn max_columns_does_not_lose_late_matches() {
    let dir = setup_repo();
    // Line longer than max_columns with the match past the truncation point.
    let long_file = dir.path().join("long.txt");
    let mut content = "x".repeat(600) + "needle_at_end";
    content.push('\n');
    std::fs::write(&long_file, content).unwrap();

    let cli = make_cli(&["-M", "200", "needle_at_end", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    assert!(
        results.iter().any(|r| r.path.contains("long.txt")),
        "match beyond max-columns was lost"
    );
}

#[test]
fn multi_path_searches_all_paths() {
    let dir = setup_repo();
    let src = dir.path().join("src");
    let tests = dir.path().join("tests");

    let cli = make_cli(&["main", src.to_str().unwrap(), tests.to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();

    // Walker must cover every given path, not just the first one.
    assert!(
        files
            .iter()
            .any(|p| p.to_string_lossy().contains("main.rs")),
        "first path not walked"
    );
    assert!(
        files
            .iter()
            .any(|p| p.to_string_lossy().contains("integration.rs")),
        "second path not walked"
    );

    let results = engine.search(&files);
    assert!(!results.is_empty(), "no results from either path");
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
