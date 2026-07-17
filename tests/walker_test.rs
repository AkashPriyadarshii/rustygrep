mod common;

use common::setup_repo;
use rustygrep::walker::FileWalker;

#[test]
fn finds_all_source_files() {
    let dir = setup_repo();
    let walker = FileWalker::new(
        vec![dir.path().to_path_buf()],
        false,
        false,
        false,
        None,
        None,
        0,
    );
    let files = walker.walk();

    assert!(files
        .iter()
        .any(|p| p.to_string_lossy().contains("main.rs")));
    assert!(files.iter().any(|p| p.to_string_lossy().contains("lib.rs")));
    assert!(files
        .iter()
        .any(|p| p.to_string_lossy().contains("config.rs")));
}

#[test]
fn respects_gitignore() {
    let dir = setup_repo();
    let root = dir.path();

    std::fs::write(root.join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(root.join("test.log"), "should be ignored\n").unwrap();
    std::fs::write(root.join("test.txt"), "should be found\n").unwrap();

    let walker = FileWalker::new(vec![root.to_path_buf()], false, false, false, None, None, 0);
    let files = walker.walk();

    assert!(files
        .iter()
        .any(|p| p.to_string_lossy().contains("test.txt")));
    assert!(!files
        .iter()
        .any(|p| p.to_string_lossy().contains("test.log")));
}

#[test]
fn file_type_filter() {
    let dir = setup_repo();
    let root = dir.path();

    std::fs::write(root.join("app.py"), "print('hello')\n").unwrap();
    std::fs::write(root.join("style.css"), "body { color: red; }\n").unwrap();

    let walker = FileWalker::new(
        vec![root.to_path_buf()],
        false,
        false,
        false,
        Some("py".into()),
        None,
        0,
    );
    let files = walker.walk();

    assert!(files.iter().any(|p| p.to_string_lossy().contains("app.py")));
    assert!(!files
        .iter()
        .any(|p| p.to_string_lossy().contains("style.css")));
}

#[test]
fn file_type_not_filter() {
    let dir = setup_repo();
    let root = dir.path();

    std::fs::write(root.join("app.py"), "print('hello')\n").unwrap();
    std::fs::write(root.join("style.css"), "body { color: red; }\n").unwrap();

    let walker = FileWalker::new(
        vec![root.to_path_buf()],
        false,
        false,
        false,
        None,
        Some("py".into()),
        0,
    );
    let files = walker.walk();

    assert!(!files.iter().any(|p| p.to_string_lossy().contains("app.py")));
    assert!(files
        .iter()
        .any(|p| p.to_string_lossy().contains("style.css")));
}

#[test]
fn hidden_files_not_found_by_default() {
    let dir = setup_repo();
    let root = dir.path();

    std::fs::write(root.join(".hidden"), "secret\n").unwrap();

    let walker = FileWalker::new(vec![root.to_path_buf()], false, false, false, None, None, 0);
    let files = walker.walk();

    assert!(!files
        .iter()
        .any(|p| p.to_string_lossy().contains(".hidden")));
}

#[test]
fn hidden_files_found_when_enabled() {
    let dir = setup_repo();
    let root = dir.path();

    std::fs::write(root.join(".hidden"), "secret\n").unwrap();

    let walker = FileWalker::new(vec![root.to_path_buf()], true, false, false, None, None, 0);
    let files = walker.walk();

    assert!(files
        .iter()
        .any(|p| p.to_string_lossy().contains(".hidden")));
}
