# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Search hot path (measured, 52MB/200-file corpus, i3-1115G4)** —
  `-l` 24ms vs rg 24ms (parity, first-hit short-circuit + streaming
  walk); miss path 30ms vs 27ms (parity); `--no-color` 91ms vs 64ms
  (1.4x); `-c` 56ms vs 35ms (1.6x). Counts verified identical
  (`-c` diff clean, 112,426 both tools). 46 tests green.
- **Literal fast path** (`memchr`+`memmem`, no regex engine) for plain
  case-sensitive patterns with no word/invert/context flags — biggest
  single win on dense-hit corpora (regex per-line overhead dominates).
- **One-pass file read** — `search_path` streams/mmaps with NUL quit
  (was: `fs::read` + whole-buffer `from_utf8` pre-scan, 2 extra passes).
- **`-c` stores no line text** — totals only, skip String alloc per hit.
- **Sharded results** — one Vec per rayon thread (was: Mutex per file).
- **Context-flag threading** — `has_context_lines` passed to the
  printer (was: per-line `is_context_line` branch on every hit).
- **`--no-binary` is opt-in** — default searches all files, non-UTF-8 skipped
  by the whole-buffer check in search (was: open+read 8KB probe per file).
- **`--llm` skips submatch spans** — no per-line regex re-scan when spans are
  never printed (was: `find_iter` per matched line).
- **Zero-alloc color path** — raw ANSI bytes into the buffer (was: `colored`
  heap String per segment); `--no-color` writes `write!` directly.
- **Buffered stdout everywhere** — 1MB `BufWriter` in pretty/json/llm printers
  (was: one `println!` lock+flush per match line).
- **LLM printer streams** — no whole-output `String` build (was: 13MB copy).
- **Pre-sized match vecs** — `Vec::with_capacity(1024)` per file (was: grow
  from zero across ~700 hits/file).
- **`from_utf8` fast path** — per-line `to_owned` on valid UTF-8 (was:
  `from_utf8_lossy` scan per line).
- **`-l` streams walk+search** — first-hit search runs on the walker's own
  threads as files are yielded (measured ≈ collect-then-search ±5ms, kept
  for `-l` only to preserve `--top`/`--rank` semantics elsewhere).
- **README benchmarks** — replaced stale M4 table with measured Windows/i3
  numbers, method, corpus, and reproduce steps.

## [0.1.3] - 2026-07-19

### Fixed

- **Zero-width regex hang** — patterns like `\b`, `^`, `$`, `a*` no longer hang forever (advance past empty matches)
- **`-v` / `--invert-match`** — actually shows non-matching lines (was empty + exit 1)
- **`-C` / `-A` / `-B` context lines** — context lines now included in output (was silently dropped)
- **Multi-path search** — `rustygrep pattern a/ b/` now searches all given paths (was only first)
- **`-M` match on untruncated line** — matches past `max_columns` boundary no longer silently lost (exit 1 → exit 0)
- **`-c` single file count** — bare number output, rg compat (was `file:count`)
- **`-c` no match** — silent exit 1 (was printing `0`)
- **`--context-only`** — now shows context lines (was always empty)
- **MCP `submatches`** — json output includes `submatches` array
- **MCP `isError`** — tool errors use correct protocol flag
- **MCP null-id** — notifications silently ignored (was responding)
- **MCP `max_results`** — now limits file count (was match count, misleading)
- **`is_binary`** — reads only 8KB prefix (was full file)
- **Walker `is_dir`** — uses `file_type()` (was extra stat syscall)

### Changed

- **Display truncation moved to output layer** — search stores full lines; `-M` only affects display, not search correctness
- **grep-searcher `invert_match` flag** — uses built-in searcher inversion (was broken custom InvertedMatcher)
- **Walker multi-path** — `WalkBuilder::add()` for each path (was `paths[0]` only)
- **Stats computed pre-filter** — `--stats` / exit code reflect search results before `--top` / `--context-only`
- **`--rank` + `--top` compose** — rank first, then top (rank is relevance ordering; top truncates)

---

## [0.1.1] - 2026-07-10

### Added

- **MCP server** (`rustygrep mcp`) — Model Context Protocol server for AI coding agents (Claude Code, Cursor, OpenCode)
  - `rustygrep_search` tool — pattern search with format options
  - `rustygrep_files` tool — files-with-matches discovery
  - `rustygrep_count` tool — match counts per file
- **`--llm-budget N`** — cap total LLM output at N tokens (4 chars ≈ 1 token heuristic)
- **`--llm-no-truncate`** — disable line truncation in LLM output
- **`--top N`** — show only top N files ranked by match count
- **`--json-file`** — per-file JSON format (old `--json` behavior)

### Changed

- **`--llm` output improved**
  - Per-file match count in headers: `--- file (N matches)`
  - Default line truncation reduced from 200 to 120 chars
  - Cleaner summary line format
- **`--json` now produces JSONL** — one JSON object per match line (was per-file)
  - Use `--json-file` for the old per-file format

### Fixed

- Release workflow: Windows binary no longer double-named `.exe.exe`
- CI: removed ARM64 Linux from build matrix (release-only via cross-rs)
- Clippy and fmt warnings resolved

## [0.1.0] - 2026-07-10

### Added

- Initial release
- Parallel recursive search using rayon
- Gitignore-aware file discovery
- LLM-optimized token-compressed output (`--llm`)
- JSON Lines output (`--json`)
- File type filters (`-t`, `-T`)
- Case insensitive search (`-i`)
- Whole word match (`-w`)
- Match count (`-c`)
- Files with matches (`-l`)
- Context lines (`-C`, `-A`, `-B`)
- Line number display
- Color highlighting
- Max columns truncation (`-M`)
- Hidden file search (`--hidden`)
- No-ignore mode (`--no-ignore`)
- Invert match (`-v`)
- Configurable thread count (`-j`)
