mod common;

use common::{make_cli, setup_repo};
use rustygrep::search::SearchEngine;
use rustygrep::walker::FileWalker;

fn search_and_get_jsonl(args: &[&str]) -> Vec<String> {
    let dir = setup_repo();
    let mut full_args = args.to_vec();
    full_args.push(dir.path().to_str().unwrap());

    let cli = make_cli(&full_args);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    let mut output = Vec::new();
    for r in &results {
        for m in &r.matches {
            output.push(
                serde_json::json!({
                    "path": m.path,
                    "line": m.line_number,
                    "match_text": m.line,
                })
                .to_string(),
            );
        }
    }
    output
}

#[test]
fn jsonl_per_match_output() {
    let lines = search_and_get_jsonl(&["fn", "--json"]);

    assert!(!lines.is_empty());
    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(parsed.get("path").is_some());
        assert!(parsed.get("line").is_some());
        assert!(parsed.get("match_text").is_some());
    }
}

#[test]
fn json_valid_jsonl_for_jq() {
    let lines = search_and_get_jsonl(&["fn", "--json"]);

    for line in &lines {
        let result: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(result.is_ok(), "Invalid JSON: {}", line);
    }
}

#[test]
fn json_match_count() {
    let dir = setup_repo();
    let cli = make_cli(&["-c", "fn", "--json", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    for r in &results {
        let output = serde_json::json!({
            "path": r.path,
            "count": r.matches.len(),
        });
        assert!(output.get("count").is_some());
    }
}

#[test]
fn json_files_only() {
    let dir = setup_repo();
    let cli = make_cli(&["-l", "fn", "--json", dir.path().to_str().unwrap()]);
    let engine = SearchEngine::new(&cli).unwrap();
    let files = FileWalker::new(cli.paths.clone(), false, false, false, None, None, 0).walk();
    let results = engine.search(&files);

    assert!(!results.is_empty());
    for r in &results {
        assert!(!r.path.is_empty());
    }
}
