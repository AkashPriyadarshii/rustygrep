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
    /// Empty when spans are skipped (need_submatches=false). The file
    /// path lives on FileMatches; printers read matches through their
    /// parent file. MCP/json paths that need per-match paths use
    /// `match_path()` which falls back to FileMatches.path.
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
    /// Empty when no printer needs per-match paths (borrow parent's
    /// FileMatches.path instead). Saves one clone per hit (41k hits).
    path: String,
    matcher: &'a grep_regex::RegexMatcher,
    invert: bool,
    max_matches: usize,
    /// false for -l/-c/--llm/--no-color: skip re-scanning each matched
    /// line for submatch spans (searcher already confirmed the hit,
    /// and no printer in that mode reads the spans).
    need_submatches: bool,
    /// Reserved: prefix-truncation was tried and reverted (printers
    /// truncate for display but tests + rg parity require full lines).
    /// Left as a field to avoid touching all call sites again.
    keep_prefix: usize,
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

/// If the pattern is a plain literal (no regex metachars, no backslash
/// escapes), return its bytes for the memchr fast path. Covers the
/// common case (`HashMap`, `fn main`, `TODO`) without a new dep.
fn literal_bytes(pattern: &str) -> Option<Vec<u8>> {
    if pattern.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // Keep simple escapes literal (\., \*, ...); anything
            // else bails to the regex engine.
            match chars.next() {
                Some(e) if ".*+?()[]{}|^$\\".contains(e) => out.push(e as u8),
                _ => return None,
            }
        } else if c.is_ascii_alphanumeric() || " _-/.:@#".contains(c) {
            out.push(c as u8);
        } else {
            return None;
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Literal fast path: no regex engine at all. Split buffer into lines
/// with memchr(b'\n'), confirm each with memmem. Valid only when the
/// matcher is a plain case-sensitive literal with no word/invert/
/// context flags (checked by the caller) — then memmem == regex.
///
/// Arena variant: appends `path:lineno:line\n` bytes straight into
/// `arena` and returns only (line_number, start, end) spans — zero
/// per-hit String allocs. `count_only` appends nothing (totals only).
fn search_literal_lines(
    content: &[u8],
    finder: &memchr::memmem::Finder,
    count_only: bool,
) -> Vec<(u64, String)> {
    let mut out = Vec::with_capacity(1024);
    let mut lineno: u64 = 1;
    let mut start = 0;
    // Skip lines that can't contain the literal without slicing them:
    // memchr the first byte, then only bound+check that one line.
    // Non-candidate lines cost one SIMD scan, zero slices/copies.
    let lit = finder.needle();
    if lit.len() == 1 {
        while start <= content.len() {
            let end = match memchr::memchr(b'\n', &content[start..]) {
                Some(rel) => start + rel,
                None => content.len(),
            };
            if end > start && memchr::memchr(lit[0], &content[start..end]).is_some() {
                if count_only {
                    out.push((lineno, String::new()));
                } else {
                    let line = &content[start..end];
                    if !line.contains(&b'\x00') {
                        out.push((lineno, trim_line_ending(line)));
                    }
                }
            }
            if end == content.len() {
                break;
            }
            lineno += 1;
            start = end + 1;
        }
        return out;
    }
    // First-byte prefilter (rare-byte tried: Finder confirm dominates,
    // prefilter choice within noise — keep the simple correct one).
    let mut pos = 0;
    let mut last_ls = usize::MAX;
    while pos < content.len() {
        let rel = match memchr::memchr(lit[0], &content[pos..]) {
            Some(r) => r,
            None => break,
        };
        let abs = pos + rel;
        if content[abs] == b'\x00' {
            return Vec::new(); // binary quit, matches search_path
        }
        let mut ls = abs;
        while ls > 0 && content[ls - 1] != b'\n' {
            ls -= 1;
        }
        if ls == last_ls {
            pos = abs + 1;
            continue;
        }
        last_ls = ls;
        // Newlines crossed since the last candidate line.
        lineno += memchr::memchr_iter(b'\n', &content[start..ls]).count() as u64;
        let mut le = abs;
        while le < content.len() && content[le] != b'\n' {
            le += 1;
        }
        start = if le < content.len() { le + 1 } else { content.len() };
        let line = &content[ls..le];
        if !line.contains(&b'\x00') && finder.find(line).is_some() {
            if count_only {
                out.push((lineno, String::new()));
            } else {
                out.push((lineno, trim_line_ending(line)));
            }
        }
        pos = abs + 1;
    }
    out
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
            // Fast path: skip per-match re-scan entirely (searcher
            // already confirmed a hit; no printer needs the spans).
            // NOTE: count mode no longer reaches this collector —
            // count_only files take search_count_file (plain usize,
            // zero Match allocs). keep_prefix==MAX now only guards
            // stale callers; treat as normal line (never empty).
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
    // Hot path: the whole file buffer was already validated UTF-8 in
    // search_file, so every line slice is valid by construction. Skip
    // the second 20MB from_utf8 scan (safe: slice of validated input).
    // Falls back to lossy only if the caller skipped validation.
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
    keep_prefix: usize,
    fast_literal: Option<Vec<u8>>,
}

impl SearchEngine {
    /// True when the arena fast path is valid: literal path is active
    /// (implies no context/invert/word/ignore-case/span flags) plus
    /// plain full-output mode with no ranking or truncation-bypass.
    pub fn can_use_arena(&self) -> bool {
        self.fast_literal.is_some()
    }

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

        // Highlight/JSON spans only matter when the line text is shown
        // with highlights. files_with_matches/count/--llm/--no-color
        // modes never print spans — skipping the per-line find_iter
        // re-scan is the single biggest hot-path win (~20ms on 41k hits).
        let need_submatches = !cli.files_with_matches
            && !cli.count
            && !cli.llm
            && !cli.no_color
            && !cli.json
            && !cli.json_file;
        // -l needs only existence: stop the searcher at the first hit.
        // (count mode still needs every match for totals.)
        let files_only_fast = cli.files_with_matches && !cli.invert_match && !cli.count;

        // Count mode (-c) prints only totals: store no line text at
        // all (usize::MAX sentinel). Full-output modes keep full lines
        // for rg parity (printers truncate for display only).
        let keep_prefix = if cli.count { usize::MAX } else { 0 };
        // Literal fast path: valid exactly when spans aren't needed
        // AND no regex-altering flags are set. memchr+memmem, no
        // regex engine per line.
        let fast_literal = if need_submatches
            || cli.ignore_case
            || cli.word_regexp
            || cli.invert_match
            || context_before > 0
            || context_after > 0
        {
            None
        } else {
            cli.pattern.as_deref().and_then(literal_bytes)
        };
        Ok(Self {
            matcher,
            context_before,
            context_after,
            invert_match: cli.invert_match,
            max_matches: cli.max_matches,
            need_submatches,
            files_only_fast,
            keep_prefix,
            fast_literal,
        })
    }

    /// True when -l takes the first-hit short-circuit path.
    pub fn is_files_only_fast(&self) -> bool {
        self.files_only_fast
    }

    pub fn search(&self, files: &[PathBuf]) -> Vec<FileMatches> {
        // Sharded results: one Vec per rayon thread, concatenated at
        // the end. Kills the Mutex lock/unlock per file (200 files =
        // 200 lock round-trips on the hot path).
        let shards: Vec<Mutex<Vec<FileMatches>>> = (0..rayon::current_num_threads())
            .map(|_| Mutex::new(Vec::new()))
            .collect();
        files.par_iter().for_each(|path| {
            let idx =
                rayon::current_thread_index().unwrap_or(0) % shards.len().max(1);
            self.push_hit(path, &shards[idx]);
        });
        let mut final_results: Vec<FileMatches> = shards
            .into_iter()
            .flat_map(|s| s.into_inner().unwrap_or_default())
            .collect();
        final_results.sort_by(|a, b| a.path.cmp(&b.path));
        final_results
    }

    /// Arena fast path for plain `--no-color` full output: each rayon
    /// thread owns a byte arena; matching bytes are appended per file
    /// (`path:lineno:line\n`) with zero per-hit String allocs. Sorts
    /// FILES only (matches are sequential by construction), then
    /// writes shards in order with one write_all each.
    /// Returns (bytes_in_order, total_matches, files_matched).
    /// Valid only when: literal fast path active, no count/files-only,
    /// no top/rank/context (checked by caller in main).
    pub fn search_arena(&self, files: &[PathBuf], max_cols: usize) -> (Vec<u8>, usize, usize) {
        struct Shard {
            bytes: Vec<u8>,
            // (path, start, end, hits) — path owned for final sort.
            files: Vec<(String, usize, usize, usize)>,
        }
        let n = rayon::current_num_threads().max(1);
        let shards: Vec<Mutex<Shard>> = (0..n)
            .map(|_| {
                Mutex::new(Shard {
                    bytes: Vec::with_capacity(1 << 20),
                    files: Vec::new(),
                })
            })
            .collect();
        let lit = self.fast_literal.clone().unwrap_or_default();
        let finder = memchr::memmem::Finder::new(&lit);
        files.par_iter().for_each(|path| {
            let idx = rayon::current_thread_index().unwrap_or(0) % n;
            let content = match std::fs::read(path) {
                Ok(c) => c,
                Err(_) => return,
            };
            // Candidate-line walk inlined here to append directly:
            // collect (lineno, ls, le) spans, then append bytes.
            let mut spans: Vec<(u64, usize, usize)> = Vec::with_capacity(256);
            {
                let mut lineno: u64 = 1;
                let mut start = 0;
                let mut pos = 0;
                let mut last_ls = usize::MAX;
                while pos < content.len() {
                    let rel = match memchr::memchr(lit[0], &content[pos..]) {
                        Some(r) => r,
                        None => break,
                    };
                    let abs = pos + rel;
                    if content[abs] == b'\x00' {
                        return; // binary quit
                    }
                    let mut ls = abs;
                    while ls > 0 && content[ls - 1] != b'\n' {
                        ls -= 1;
                    }
                    if ls == last_ls {
                        pos = abs + 1;
                        continue;
                    }
                    last_ls = ls;
                    lineno += memchr::memchr_iter(b'\n', &content[start..ls]).count() as u64;
                    let mut le = abs;
                    while le < content.len() && content[le] != b'\n' {
                        le += 1;
                    }
                    start = if le < content.len() { le + 1 } else { content.len() };
                    let line = &content[ls..le];
                    if !line.contains(&b'\x00') && finder.find(line).is_some() {
                        spans.push((lineno, ls, le));
                    }
                    pos = abs + 1;
                }
            }
            if spans.is_empty() {
                return;
            }
            let path_str = path.to_string_lossy();
            let path_bytes = path_str.as_bytes();
            let mut shard = shards[idx].lock().unwrap();
            let start_off = shard.bytes.len();
            for (ln, ls, le) in spans.iter() {
                crate::output::pretty::append_plain_match(
                    &mut shard.bytes,
                    path_bytes,
                    *ln,
                    &content[*ls..*le],
                    max_cols,
                );
            }
            let end_off = shard.bytes.len();
            shard.files.push((path_str.into_owned(), start_off, end_off, spans.len()));
        });
        // Merge: sort files by path, concat slices in order.
        let mut all: Vec<(String, usize, usize, usize, usize)> = Vec::new();
        for (si, s) in shards.iter().enumerate() {
            let shard = s.lock().unwrap();
            for (p, st, en, hits) in shard.files.iter() {
                all.push((p.clone(), si, *st, *en, *hits));
            }
        }
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let total: usize = all.iter().map(|f| f.4).sum();
        let nfiles = all.len();
        let mut out = Vec::with_capacity(all.iter().map(|f| f.3 - f.2).sum());
        for (_, si, st, en, _) in all {
            let shard = shards[si].lock().unwrap();
            out.extend_from_slice(&shard.bytes[st..en]);
        }
        (out, total, nfiles)
    }

    /// Fused walk+search arena: the search runs INSIDE the ignore
    /// parallel walker's visitor — one pool, not walk-pool feeding a
    /// rayon pool. Each visitor call appends straight to a shard
    /// (picked round-robin by an atomic counter, no TLS needed).
    /// Same output bytes as search_arena; same gating (caller).
    /// LLM arena: same fused walk+search, LLM line format
    /// (`lineno:truncated\n` + `--- path (N)` headers + summary).
    /// Zero per-hit Strings; honors truncate/budget via post-cut.
    /// Budget mode falls back to the Match path (char counting).
    pub fn search_arena_llm(
        &self,
        walker: &FileWalker,
        max_line_chars: usize,
        truncate: bool,
    ) -> (Vec<u8>, usize, usize) {
        struct Shard {
            bytes: Vec<u8>,
            files: Vec<(String, usize, usize, usize)>,
        }
        let n = 4;
        let shards: Vec<Mutex<Shard>> = (0..n)
            .map(|_| {
                Mutex::new(Shard {
                    bytes: Vec::with_capacity(1 << 20),
                    files: Vec::new(),
                })
            })
            .collect();
        let lit = self.fast_literal.clone().unwrap_or_default();
        let finder = memchr::memmem::Finder::new(&lit);
        let next = std::sync::atomic::AtomicUsize::new(0);
        walker.walk_parallel(|path| {
            let idx = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n;
            let content = match std::fs::read(path) {
                Ok(c) => c,
                Err(_) => return,
            };
            let mut spans: Vec<(u64, usize, usize)> = Vec::with_capacity(256);
            {
                let mut lineno: u64 = 1;
                let mut start = 0;
                let mut pos = 0;
                let mut last_ls = usize::MAX;
                while pos < content.len() {
                    let rel = match memchr::memchr(lit[0], &content[pos..]) {
                        Some(r) => r,
                        None => break,
                    };
                    let abs = pos + rel;
                    if content[abs] == b'\x00' {
                        return;
                    }
                    let mut ls = abs;
                    while ls > 0 && content[ls - 1] != b'\n' {
                        ls -= 1;
                    }
                    if ls == last_ls {
                        pos = abs + 1;
                        continue;
                    }
                    last_ls = ls;
                    lineno += memchr::memchr_iter(b'\n', &content[start..ls]).count() as u64;
                    let mut le = abs;
                    while le < content.len() && content[le] != b'\n' {
                        le += 1;
                    }
                    start = if le < content.len() { le + 1 } else { content.len() };
                    let line = &content[ls..le];
                    if !line.contains(&b'\x00') && finder.find(line).is_some() {
                        spans.push((lineno, ls, le));
                    }
                    pos = abs + 1;
                }
            }
            if spans.is_empty() {
                return;
            }
            let path_str = path.to_string_lossy();
            let mut shard = shards[idx].lock().unwrap();
            let start_off = shard.bytes.len();
            crate::output::llm::append_llm_file(
                &mut shard.bytes,
                &path_str,
                &content,
                &spans,
                max_line_chars,
                truncate,
            );
            let end_off = shard.bytes.len();
            shard.files.push((path_str.into_owned(), start_off, end_off, spans.len()));
        });
        let mut all: Vec<(String, usize, usize, usize, usize)> = Vec::new();
        for (si, s) in shards.iter().enumerate() {
            let shard = s.lock().unwrap();
            for (p, st, en, hits) in shard.files.iter() {
                all.push((p.clone(), si, *st, *en, *hits));
            }
        }
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let total: usize = all.iter().map(|f| f.4).sum();
        let nfiles = all.len();
        let cap: usize = all.iter().map(|f| f.3 - f.2).sum();
        let mut out = Vec::with_capacity(cap + 64);
        for (_, si, st, en, _) in all {
            let shard = shards[si].lock().unwrap();
            out.extend_from_slice(&shard.bytes[st..en]);
        }
        // Summary line (matches old printer shape).
        out.extend_from_slice(b"\n--- ");
        let mut numbuf = [0u8; 20];
        let mut v = total;
        let mut len = 0;
        if v == 0 {
            numbuf[0] = b'0';
            len = 1;
        } else {
            let mut tmp = [0u8; 20];
            while v > 0 {
                tmp[len] = b'0' + (v % 10) as u8;
                v /= 10;
                len += 1;
            }
            let mut i = 0;
            while i < len {
                numbuf[i] = tmp[len - 1 - i];
                i += 1;
            }
        }
        out.extend_from_slice(&numbuf[..len]);
        out.extend_from_slice(if total == 1 { b" match in " } else { b" matches in " });
        let mut numbuf = [0u8; 20];
        let mut v = nfiles;
        let mut len = 0;
        if v == 0 {
            numbuf[0] = b'0';
            len = 1;
        } else {
            let mut tmp = [0u8; 20];
            while v > 0 {
                tmp[len] = b'0' + (v % 10) as u8;
                v /= 10;
                len += 1;
            }
            let mut i = 0;
            while i < len {
                numbuf[i] = tmp[len - 1 - i];
                i += 1;
            }
        }
        out.extend_from_slice(&numbuf[..len]);
        out.extend_from_slice(if nfiles == 1 { b" file\n" } else { b" files\n" });
        (out, total, nfiles)
    }

    pub fn search_arena_fused(&self, walker: &FileWalker, max_cols: usize) -> (Vec<u8>, usize, usize) {
        struct Shard {
            bytes: Vec<u8>,
            files: Vec<(String, usize, usize, usize)>,
        }
        let n = 4;
        let shards: Vec<Mutex<Shard>> = (0..n)
            .map(|_| {
                Mutex::new(Shard {
                    bytes: Vec::with_capacity(1 << 20),
                    files: Vec::new(),
                })
            })
            .collect();
        let lit = self.fast_literal.clone().unwrap_or_default();
        let finder = memchr::memmem::Finder::new(&lit);
        let next = std::sync::atomic::AtomicUsize::new(0);
        walker.walk_parallel(|path| {
            let idx = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n;
            let content = match std::fs::read(path) {
                Ok(c) => c,
                Err(_) => return,
            };
            let mut spans: Vec<(u64, usize, usize)> = Vec::with_capacity(256);
            {
                let mut lineno: u64 = 1;
                let mut start = 0;
                let mut pos = 0;
                let mut last_ls = usize::MAX;
                while pos < content.len() {
                    let rel = match memchr::memchr(lit[0], &content[pos..]) {
                        Some(r) => r,
                        None => break,
                    };
                    let abs = pos + rel;
                    if content[abs] == b'\x00' {
                        return;
                    }
                    let mut ls = abs;
                    while ls > 0 && content[ls - 1] != b'\n' {
                        ls -= 1;
                    }
                    if ls == last_ls {
                        pos = abs + 1;
                        continue;
                    }
                    last_ls = ls;
                    lineno += memchr::memchr_iter(b'\n', &content[start..ls]).count() as u64;
                    let mut le = abs;
                    while le < content.len() && content[le] != b'\n' {
                        le += 1;
                    }
                    start = if le < content.len() { le + 1 } else { content.len() };
                    let line = &content[ls..le];
                    if !line.contains(&b'\x00') && finder.find(line).is_some() {
                        spans.push((lineno, ls, le));
                    }
                    pos = abs + 1;
                }
            }
            if spans.is_empty() {
                return;
            }
            let path_str = path.to_string_lossy();
            let path_bytes = path_str.as_bytes();
            let mut shard = shards[idx].lock().unwrap();
            let start_off = shard.bytes.len();
            for (ln, ls, le) in spans.iter() {
                crate::output::pretty::append_plain_match(
                    &mut shard.bytes,
                    path_bytes,
                    *ln,
                    &content[*ls..*le],
                    max_cols,
                );
            }
            let end_off = shard.bytes.len();
            shard.files.push((path_str.into_owned(), start_off, end_off, spans.len()));
        });
        let mut all: Vec<(String, usize, usize, usize, usize)> = Vec::new();
        for (si, s) in shards.iter().enumerate() {
            let shard = s.lock().unwrap();
            for (p, st, en, hits) in shard.files.iter() {
                all.push((p.clone(), si, *st, *en, *hits));
            }
        }
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let total: usize = all.iter().map(|f| f.4).sum();
        let nfiles = all.len();
        let mut out = Vec::with_capacity(all.iter().map(|f| f.3 - f.2).sum());
        for (_, si, st, en, _) in all {
            let shard = shards[si].lock().unwrap();
            out.extend_from_slice(&shard.bytes[st..en]);
        }
        (out, total, nfiles)
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
        // -l needs existence only: no line numbers needed. Disable
        // the searcher's per-match line-number bookkeeping.
        let file_path = path.to_string_lossy().to_string();
        let mut searcher = SearcherBuilder::new()
            .line_number(false)
            .binary_detection(grep_searcher::BinaryDetection::quit(b'\x00'))
            .build();
        let mut collector = MatchCollector {
            path: String::new(), // printers use FileMatches.path for -l
            matcher: &self.matcher,
            invert: false,
            max_matches: 1, // stop after first line
            need_submatches: false,
            keep_prefix: 0,
            matches: Vec::new(),
        };
        let _ = searcher.search_path(self.matcher.clone(), path, &mut collector);
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

    /// True count path: no Match objects at all. Literal mode counts
    /// memmem hits; regex mode counts searcher callbacks. Returns the
    /// line-match count (rg -c semantics: lines, not occurrences).
    /// `foo foo foo` on one line counts 1 — the Sink fires per line.
    pub fn search_count_file(&self, path: &Path) -> usize {
        if let Some(ref lit) = self.fast_literal {
            let content = match std::fs::read(path) {
                Ok(c) => c,
                Err(_) => return 0,
            };
            let finder = memchr::memmem::Finder::new(lit);
            return search_literal_lines(&content, &finder, true).len();
        }
        struct Counter(usize);
        impl Sink for Counter {
            type Error = std::io::Error;
            fn matched(
                &mut self,
                _searcher: &Searcher,
                _mat: &SinkMatch<'_>,
            ) -> Result<bool, std::io::Error> {
                self.0 += 1;
                Ok(true)
            }
        }
        let mut searcher = SearcherBuilder::new()
            .line_number(false)
            .before_context(0)
            .after_context(0)
            .invert_match(self.invert_match)
            .binary_detection(grep_searcher::BinaryDetection::quit(b'\x00'))
            .build();
        let mut counter = Counter(0);
        let _ = searcher.search_path(self.matcher.clone(), path, &mut counter);
        counter.0
    }

    /// Count sweep over files: sharded (path, count) pairs, no Match
    /// vecs anywhere. Sorted by path for deterministic -c output.
    pub fn search_counts(&self, files: &[PathBuf]) -> Vec<(String, usize)> {
        use std::sync::Mutex;
        let n = rayon::current_num_threads().max(1);
        let shards: Vec<Mutex<Vec<(String, usize)>>> =
            (0..n).map(|_| Mutex::new(Vec::new())).collect();
        files.par_iter().for_each(|path| {
            let c = self.search_count_file(path);
            if c > 0 {
                let idx = rayon::current_thread_index().unwrap_or(0) % n;
                shards[idx]
                    .lock()
                    .unwrap()
                    .push((path.to_string_lossy().into_owned(), c));
            }
        });
        let mut all: Vec<(String, usize)> = shards
            .into_iter()
            .flat_map(|s| s.into_inner().unwrap_or_default())
            .collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }

    fn search_file(&self, path: &Path) -> Option<FileMatches> {
        // files_with_matches: ask the searcher to stop at the first hit
        // instead of scanning + collecting every line in the file.
        if self.files_only_fast {
            return self.search_file_first_hit(path);
        }
        // Literal fast path first: single fs::read + memchr/memmem,
        // no regex engine, no searcher line buffer. Wins big on
        // dense-hit corpora (regex engine per-line overhead dominates).
        if let Some(ref lit) = self.fast_literal {
            // --no-binary probe stays in the walker (opt-in flag).
            // Finder built once per file (not per line) — reuses the
            // precomputed skip table instead of rebuilding per hit.
            let content = std::fs::read(path).ok()?;
            let file_path = path.to_string_lossy().to_string();
            let count_only = self.keep_prefix == usize::MAX;
            let finder = memchr::memmem::Finder::new(lit);
            let hits = search_literal_lines(&content, &finder, count_only);
            if hits.is_empty() {
                return None;
            }
            let path_str = if self.need_submatches {
                file_path.clone()
            } else {
                String::new()
            };
            let total = hits.len();
            let matches = hits
                .into_iter()
                .map(|(ln, line)| Match {
                    path: path_str.clone(),
                    line_number: ln,
                    line,
                    submatches: vec![],
                })
                .collect();
            return Some(FileMatches {
                path: file_path,
                total_matches: total,
                matches,
            });
        }
        // One pass: search_path streams/mmaps internally and quits on
        // NUL via BinaryDetection — replaces fs::read + from_utf8
        // (two full extra passes over every byte before searching).
        let file_path = path.to_string_lossy().to_string();

        // Count mode: line numbers never printed — disable the
        // searcher's per-match line-number bookkeeping (~5-10ms on
        // dense corpora). Full-output modes keep it enabled.
        let count_only = self.keep_prefix == usize::MAX;
        let mut searcher = SearcherBuilder::new()
            .line_number(!count_only)
            .before_context(self.context_before)
            .after_context(self.context_after)
            .invert_match(self.invert_match)
            .binary_detection(grep_searcher::BinaryDetection::quit(b'\x00'))
            .build();

        // Pre-size: one file ≈ 600-700 hits in the bench corpus.
        // Kills ~200 realloc+memcpy rounds per file (133k matches).
        // Skip the per-match path clone when no printer needs it
        // (-l/-c/--llm/--no-color read FileMatches.path instead).
        let skip_path_clone = !self.need_submatches;
        let mut collector = MatchCollector {
            path: if skip_path_clone {
                String::new()
            } else {
                file_path.clone()
            },
            matcher: &self.matcher,
            invert: self.invert_match,
            max_matches: self.max_matches,
            need_submatches: self.need_submatches,
            keep_prefix: self.keep_prefix,
            matches: Vec::with_capacity(1024),
        };

        let _ = searcher.search_path(self.matcher.clone(), path, &mut collector);

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
