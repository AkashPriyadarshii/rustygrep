use clap::Parser;

fn parse_cli(args: &[&str]) -> rustygrep::cli::Cli {
    let mut full_args = vec!["test"];
    full_args.extend(args);
    rustygrep::cli::Cli::parse_from(full_args)
}

#[test]
fn flag_llm() {
    let cli = parse_cli(&["--llm", "pattern"]);
    assert!(cli.llm);
}

#[test]
fn flag_json() {
    let cli = parse_cli(&["--json", "pattern"]);
    assert!(cli.json);
}

#[test]
fn flag_type_filter() {
    let cli = parse_cli(&["-t", "rs", "pattern"]);
    assert_eq!(cli.file_type.as_deref(), Some("rs"));
}

#[test]
fn flag_case_insensitive() {
    let cli = parse_cli(&["-i", "pattern"]);
    assert!(cli.ignore_case);
}

#[test]
fn flag_word_regexp() {
    let cli = parse_cli(&["-w", "pattern"]);
    assert!(cli.word_regexp);
}

#[test]
fn flag_top() {
    let cli = parse_cli(&["--top", "10", "pattern"]);
    assert_eq!(cli.top, Some(10));
}

#[test]
fn flag_budget() {
    let cli = parse_cli(&["--llm-budget", "500", "pattern"]);
    assert_eq!(cli.llm_budget, Some(500));
}

#[test]
fn flag_max_columns_default() {
    let cli = parse_cli(&["pattern"]);
    assert_eq!(cli.max_columns, 500);
}

#[test]
fn flag_threads_default() {
    let cli = parse_cli(&["pattern"]);
    assert_eq!(cli.threads, 0);
}

#[test]
fn flag_context_lines() {
    let cli = parse_cli(&["-C", "3", "pattern"]);
    assert_eq!(cli.context, Some(3));
}
