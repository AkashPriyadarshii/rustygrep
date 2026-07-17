use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::sinks::UTF8;
use grep_searcher::SearcherBuilder;
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::cli::Cli;

#[derive(Debug, Clone, Serialize)]
pub struct Match {
    pub path: String,
    pub line_number: u64,
    pub line: String,
    pub submatches: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileMatches {
    pub path: String,
    pub matches: Vec<Match>,
    pub total_matches: usize,
}

pub struct SearchEngine {
    matcher: grep_regex::RegexMatcher,
    context_before: usize,
    context_after: usize,
    max_columns: usize,
    invert_match: bool,
    max_matches: usize,
}

impl SearchEngine {
    pub fn new(cli: &Cli) -> Result<Self, Box<dyn std::error::Error>> {
        let mut pattern = cli.pattern.clone().ok_or("pattern is required")?;

        if cli.ignore_case {
            pattern = format!("(?i){}", pattern);
        }

        if cli.word_regexp {
            pattern = format!(r"\b{}\b", pattern);
        }

        let matcher = RegexMatcherBuilder::new()
            .line_terminator(Some(b'\n'))
            .build(&pattern)?;

        let (context_before, context_after) = cli.context_lines();

        Ok(Self {
            matcher,
            context_before,
            context_after,
            max_columns: cli.max_columns,
            invert_match: cli.invert_match,
            max_matches: cli.max_matches,
        })
    }

    pub fn search(&self, files: &[PathBuf]) -> Vec<FileMatches> {
        let results: Mutex<Vec<FileMatches>> = Mutex::new(Vec::new());

        files.par_iter().for_each(|path| {
            if let Some(file_matches) = self.search_file(path) {
                if !file_matches.matches.is_empty() {
                    results.lock().unwrap().push(file_matches);
                }
            }
        });

        let mut final_results = results.into_inner().unwrap_or_default();
        final_results.sort_by(|a, b| a.path.cmp(&b.path));
        final_results
    }

    fn search_file(&self, path: &Path) -> Option<FileMatches> {
        let content = std::fs::read(path).ok()?;
        let _content_str = std::str::from_utf8(&content).ok()?;

        let mut matches = Vec::new();
        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .before_context(self.context_before)
            .after_context(self.context_after)
            .build();

        let matcher = &self.matcher;
        let max_cols = self.max_columns;
        let invert = self.invert_match;
        let max_matches = self.max_matches;
        let file_path = path.to_string_lossy().to_string();

        let _ = searcher.search_path(
            matcher,
            path,
            UTF8(|line_number, line| {
                let trimmed = if line.len() > max_cols {
                    let mut end = max_cols;
                    while end > 0 && !line.is_char_boundary(end) {
                        end -= 1;
                    }
                    &line[..end]
                } else {
                    line
                };

                let is_match = matcher.is_match(trimmed.as_bytes()).unwrap_or(false);
                let show_line = if invert { !is_match } else { is_match };

                if show_line {
                    let mut submatches = Vec::new();
                    let bytes = trimmed.as_bytes();
                    let mut remaining = bytes;
                    let mut offset = 0;
                    while !remaining.is_empty() {
                        if let Ok(Some(m)) = matcher.find(remaining) {
                            let start = offset + m.start();
                            let end = offset + m.end();
                            submatches.push((start, end));
                            offset = end;
                            remaining = &bytes[end..];
                        } else {
                            break;
                        }
                    }

                    matches.push(Match {
                        path: file_path.clone(),
                        line_number,
                        line: trimmed.trim_end().to_string(),
                        submatches,
                    });

                    if max_matches > 0 && matches.len() >= max_matches {
                        return Ok(false);
                    }
                }

                Ok(true)
            }),
        );

        if matches.is_empty() {
            None
        } else {
            Some(FileMatches {
                path: file_path,
                total_matches: matches.len(),
                matches,
            })
        }
    }
}

/// BM25-lite scoring: definitions > tests > comments > regular code
pub fn score_match(line: &str) -> f64 {
    let trimmed = line.trim_start();

    // Definitions: fn, struct, impl, trait, enum, type, const, static, pub
    if trimmed.starts_with("pub ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("trait ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("const ")
        || trimmed.starts_with("static ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub(crate) ")
    {
        return 10.0;
    }

    // Tests
    if trimmed.contains("#[test]")
        || trimmed.starts_with("test ")
        || trimmed.starts_with("fn test_")
        || trimmed.contains("assert")
    {
        return 6.0;
    }

    // Comments / docs
    if trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with("*")
        || trimmed.starts_with("#[")
    {
        return 2.0;
    }

    // Regular code
    4.0
}

/// BM25-lite file score: aggregates match scores with IDF weighting
pub fn score_file(file_matches: &FileMatches, total_files: usize, avg_matches: f64) -> f64 {
    let n = total_files as f64;
    let df = file_matches.matches.len() as f64;
    let k1 = 1.2;
    let b = 0.75;

    // IDF: log((N - df + 0.5) / (df + 0.5) + 1)
    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

    // TF component: weighted by match quality
    let tf: f64 = file_matches
        .matches
        .iter()
        .map(|m| score_match(&m.line))
        .sum::<f64>()
        / (file_matches.matches.len() as f64);

    // BM25 formula: IDF * (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl/avgdl))
    let dl = file_matches.matches.len() as f64;
    let norm = dl / avg_matches.max(1.0);

    idf * (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * norm))
}

/// Rank files by BM25-lite score
pub fn rank_by_score(results: &mut [FileMatches]) {
    let total_files = results.len() as f64;
    let avg_matches =
        results.iter().map(|r| r.total_matches as f64).sum::<f64>() / total_files.max(1.0);

    results.sort_by(|a, b| {
        let score_a = score_file(a, total_files as usize, avg_matches);
        let score_b = score_file(b, total_files as usize, avg_matches);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
