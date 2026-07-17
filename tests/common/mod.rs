use tempfile::TempDir;

pub fn setup_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();

    std::fs::write(
        root.join("src/main.rs"),
        "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}\n\nfn error_handler(msg: &str) {\n    eprintln!(\"Error: {}\", msg);\n}\n",
    )
    .unwrap();

    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn subtract(a: i32, b: i32) -> i32 {\n    a - b\n}\n",
    )
    .unwrap();

    std::fs::write(
        root.join("src/config.rs"),
        "pub struct Config {\n    pub name: String,\n    pub debug: bool,\n}\n\nimpl Config {\n    pub fn new() -> Self {\n        Config { name: String::from(\"default\"), debug: false }\n    }\n}\n",
    )
    .unwrap();

    std::fs::write(root.join("tests/integration.rs"), "// integration test\nassert_eq!(2 + 2, 4);\n")
        .unwrap();

    dir
}

pub fn make_cli(args: &[&str]) -> rustygrep::cli::Cli {
    use clap::Parser;
    let mut full_args = vec!["test"];
    full_args.extend(args);
    rustygrep::cli::Cli::parse_from(full_args)
}
