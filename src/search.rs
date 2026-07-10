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
        let mut pattern = cli.pattern.clone();

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
                    &line[..max_cols]
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
