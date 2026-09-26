use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, SearcherBuilder, Sink, SinkContext, SinkMatch};
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::cli::Cli;
use crate::walker::FileWalker;

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

/// Collects matches and context lines from the searcher.
struct MatchCollector<'a> {
    path: String,
    matcher: &'a grep_regex::RegexMatcher,
    invert: bool,
    max_matches: usize,
    /// false for -l/-c/count-only paths: skip re-scanning each matched
    /// line for submatch spans (searcher already confirmed the hit).
    need_submatches: bool,
    matches: Vec<Match>,
}

/// Advance past a UTF-8 character boundary (skip continuation bytes).
#[inline]
fn next_char_boundary(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos + 1;
    while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
        i += 1;
    }
    i.min(bytes.len())
}

impl<'a> Sink for MatchCollector<'a> {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, std::io::Error> {
        let bytes = mat.bytes();
        let (line, submatches) = if self.invert {
            // Inverted matches carry no submatches.
            (trim_line_ending(bytes), vec![])
        } else if !self.need_submatches {
            // Fast path for -l/-c/--llm/--json counts: skip per-match
            // re-scan entirely (searcher already confirmed a hit).
            (trim_line_ending(bytes), vec![])
        } else {
            let mut submatches = Vec::new();
            let mut offset = 0;
            let _ = self.matcher.find_iter(bytes, |m| {
                submatches.push((m.start(), m.end()));
                true
            });
            // find_iter covers non-empty matches in one engine pass.
            // Fall back to the advancing loop only if nothing was found
            // (e.g. zero-width patterns like ^/$ need manual advance).
            if submatches.is_empty() {
                offset = 0;
                while offset < bytes.len() {
                    match self.matcher.find(&bytes[offset..]) {
                        Ok(Some(m)) => {
                            let start = offset + m.start();
                            let end = offset + m.end();
                            submatches.push((start, end));
                            offset = if m.start() == m.end() {
                                next_char_boundary(bytes, end)
                            } else {
                                end
                            };
                        }
                        _ => break,
                    }
                }
            }
            (trim_line_ending(bytes), submatches)
        };
        self.matches.push(Match {
            path: self.path.clone(),
            line_number: mat.line_number().unwrap_or(0),
            line,
            submatches,
        });
        Ok(!(self.max_matches > 0 && self.matches.len() >= self.max_matches))
    }

    fn context(
        &mut self,
        _searcher: &Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, std::io::Error> {
        let line = trim_line_ending(ctx.bytes());
        self.matches.push(Match {
            path: self.path.clone(),
            line_number: ctx.line_number().unwrap_or(0),
            line,
            submatches: vec![],
        });
        Ok(true)
    }
}

#[inline]
fn trim_line_ending(bytes: &[u8]) -> String {
    let mut end = bytes.len();
    if end > 0 && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && bytes[end - 1] == b'\r' {
        end -= 1;
    }
    // Hot path: inputs are already validated UTF-8 per file (single
    // from_utf8 check on the whole buffer in search_file). Skip the
    // per-line lossy scan; fall back only on the rare invalid slice.
    match std::str::from_utf8(&bytes[..end]) {
        Ok(s) => s.to_owned(),
        Err(_) => String::from_utf8_lossy(&bytes[..end]).into_owned(),
    }
}

pub struct SearchEngine {
    matcher: grep_regex::RegexMatcher,
    context_before: usize,
    context_after: usize,
    invert_match: bool,
    max_matches: usize,
    need_submatches: bool,
    files_only_fast: bool,
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

        // Highlight/JSON spans only matter when the line text is shown.
        // files_with_matches/count/--llm modes never print spans.
        let need_submatches = !cli.files_with_matches && !cli.count && !cli.llm;
        // -l needs only existence: stop the searcher at the first hit.
        // (count mode still needs every match for totals.)
        let files_only_fast = cli.files_with_matches && !cli.invert_match && !cli.count;

        Ok(Self {
            matcher,
            context_before,
            context_after,
            invert_match: cli.invert_match,
            max_matches: cli.max_matches,
            need_submatches,
            files_only_fast,
        })
    }

    /// True when -l takes the first-hit short-circuit path.
    pub fn is_files_only_fast(&self) -> bool {
        self.files_only_fast
    }

    pub fn search(&self, files: &[PathBuf]) -> Vec<FileMatches> {
        let results: Mutex<Vec<FileMatches>> = Mutex::new(Vec::new());

        files.par_iter().for_each(|path| {
            self.push_hit(path, &results);
        });

        let mut final_results = results.into_inner().unwrap_or_default();
        final_results.sort_by(|a, b| a.path.cmp(&b.path));
        final_results
    }

    /// Streaming walk+search: search each file on the walker's own
    /// threads the moment it is yielded. Removes the walk-then-search
    /// serialization (walk 200 files, then search) plus the mpsc hop
    /// and the intermediate Vec<PathBuf>. Returns (results, files_seen).
    ///
    /// NOTE: the ignore walker's parallel visitor runs single-threaded
    /// unless its threads() count allows work-stealing across dirs
    /// (flat dirs like the bench yield from one thread). The win comes
    /// from overlapped walk+search, not from more search threads.
    pub fn search_streaming(&self, walker: &FileWalker) -> (Vec<FileMatches>, usize) {
        let results: Mutex<Vec<FileMatches>> = Mutex::new(Vec::new());
        let seen = walker.walk_parallel(|path| {
            self.push_hit(path, &results);
        });
        let mut final_results = results.into_inner().unwrap_or_default();
        final_results.sort_by(|a, b| a.path.cmp(&b.path));
        (final_results, seen)
    }

    #[inline]
    fn push_hit(&self, path: &Path, results: &Mutex<Vec<FileMatches>>) {
        if let Some(file_matches) = self.search_file(path) {
            if !file_matches.matches.is_empty() {
                results.lock().unwrap().push(file_matches);
            }
        }
    }

    /// -l fast path: collect only the first matched line, then stop.
    /// Keeps `matches` non-empty (tests/printers probe it) while still
    /// skipping the rest of the file after the first hit.
    fn search_file_first_hit(&self, path: &Path) -> Option<FileMatches> {
        let content = std::fs::read(path).ok()?;
        if std::str::from_utf8(&content).is_err() {
            return None;
        }
        let file_path = path.to_string_lossy().to_string();
        let mut searcher = SearcherBuilder::new().line_number(true).build();
        let mut collector = MatchCollector {
            path: file_path.clone(),
            matcher: &self.matcher,
            invert: false,
            max_matches: 1, // stop after first line
            need_submatches: false,
            matches: Vec::new(),
        };
        let _ = searcher.search_slice(self.matcher.clone(), &content, &mut collector);
        let matches = collector.matches;
        if matches.is_empty() {
            None
        } else {
            Some(FileMatches {
                path: file_path,
                total_matches: 1,
                matches,
            })
        }
    }

    fn search_file(&self, path: &Path) -> Option<FileMatches> {
        // files_with_matches: ask the searcher to stop at the first hit
        // instead of scanning + collecting every line in the file.
        if self.files_only_fast {
            return self.search_file_first_hit(path);
        }
        // Single read — no double I/O.
        let content = std::fs::read(path).ok()?;

        // Skip non-UTF-8 (binary) files to match existing semantics.
        if std::str::from_utf8(&content).is_err() {
            return None;
        }

        let file_path = path.to_string_lossy().to_string();

        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .before_context(self.context_before)
            .after_context(self.context_after)
            .invert_match(self.invert_match)
            .build();

        // Pre-size: one file ≈ 600-700 hits in the bench corpus.
        // Kills ~200 realloc+memcpy rounds per file (133k matches).
        let mut collector = MatchCollector {
            path: file_path.clone(),
            matcher: &self.matcher,
            invert: self.invert_match,
            max_matches: self.max_matches,
            need_submatches: self.need_submatches,
            matches: Vec::with_capacity(1024),
        };

        let _ = searcher.search_slice(self.matcher.clone(), &content, &mut collector);

        let matches = collector.matches;
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

/// Rank files by BM25-lite score (highest first).
pub fn rank_by_score(results: &mut [FileMatches]) {
    let total_files = results.len();
    let avg_matches =
        results.iter().map(|r| r.total_matches as f64).sum::<f64>() / (total_files as f64).max(1.0);

    results.sort_by(|a, b| {
        let score_a = score_file(a, total_files, avg_matches);
        let score_b = score_file(b, total_files, avg_matches);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
