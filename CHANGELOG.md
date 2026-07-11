# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
