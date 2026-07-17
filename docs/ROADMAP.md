# rustygrep Roadmap

**Last updated:** 2026-07-10

---

## v0.1.0 — Initial Release (shipped)

**Tag:** v0.1.0 | **Date:** 2026-07-10

- [x] Parallel grep built on ripgrep crates (`grep-regex`, `grep-searcher`, `ignore`)
- [x] `--llm` flag for token-compressed output (file headers, line truncation, summary)
- [x] `--json` flag for JSON Lines output
- [x] Pretty colored output with match highlighting
- [x] File type filtering (`-t rs`, `-T py`)
- [x] Context lines (`-A`, `-B`, `-C`)
- [x] Case insensitive (`-i`), whole word (`-w`), invert match (`-v`)
- [x] Count mode (`-c`), files-only mode (`-l`)
- [x] .gitignore aware, skips binary/hidden files
- [x] Multi-platform releases: macOS ARM64/x86_64, Linux ARM64/x86_64, Windows x86_64
- [x] CI: clippy, fmt, check, test, build matrix
- [x] 2.4MB binary with LTO + strip

---

## v0.1.1 — Agent Integration (next)

**Target:** 2026-07-17 | **Theme:** "The grep agents actually use"

### P0 — Must Ship

- [x] **MCP server** (`rustygrep mcp`)
  - [x] JSON-RPC over stdio
  - [x] `rustygrep_search` tool (pattern, format, max-results)
  - [x] `rustygrep_files` tool (files-with-matches)
  - [x] `rustygrep_count` tool (match counts per file)
  - [x] Works with Claude Code, Cursor, OpenCode via MCP config
  - [x] Zero external dependencies (manual JSON-RPC preferred)

- [x] **Improved `--llm` output**
  - [x] Per-file match count in `---` headers: `--- file (N matches)`
  - [x] Default line truncation at 120 chars (was 200)
  - [x] `--llm-budget N` — cap total output at N tokens (4 chars ≈ 1 token heuristic)
  - [x] `--llm-no-truncate` — disable line truncation
  - [x] Summary line: `--- N matches in M files`

### P1 — Should Ship

- [x] **Match ranking** (`--top N`)
  - [x] Sort results by match density (matches per file, descending)
  - [x] Return only top N files
  - [x] Works with all output formats

- [ ] **RTK integration**
  - [x] Match ripgrep's exit codes exactly (0=match, 1=no-match, 2=error)
  - [ ] Document RTK setup in README
  - [ ] Test with `rtk grep "pattern"` passthrough

- [x] **JSON output improvement**
  - [x] `--json` produces JSONL (one object per match line)
  - [x] `--json-file` produces per-file format (backward compat)
  - [x] Valid JSONL for `jq` piping

### P2 — Nice to Have

- [ ] `--context-only` — only show matching lines (no file paths)
- [ ] `--stats` — show timing and match statistics to stderr
- [x] Completion scripts (bash, zsh, fish)

---

## v0.2.0 — Smart Search (future)

**Theme:** "Knows what you mean"

- [ ] **BM25-lite ranking** — rank results by TF-IDF without a full index
- [ ] **Fuzzy matching** — `--fuzzy N` allows N character differences
- [ ] **Symbol search** — `--symbol fn` searches only function definitions
- [ ] **Git context** — `--since`, `--branch`, `--diff` (search only changed files)
- [ ] **Piped output modes**
  - [ ] `rustygrep digest` — compress build/test/CI output (like semble_rs)
  - [ ] `rustygrep summary` — one-line file summary (like ig symbols)

---

## v0.3.0 — Index (far future)

**Theme:** "Instant on repeated searches"

- [ ] **Trigram index** — persistent on-disk index (like ig/grix/xgrep)
- [ ] **Incremental updates** — detect changed files, re-index only those
- [ ] **Mmap persistence** — index memory-mapped for sub-ms queries
- [ ] **`rustygrep index`** — explicit index build command
- [ ] **Hybrid mode** — index + brute-force for freshness

---

## Principles

1. **Simplicity first.** Zero config, zero daemon, one binary. This is our moat.
2. **Measured, not estimated.** All token savings and benchmarks must be real, reproducible numbers.
3. **Backward compatible.** Never break existing flags without a migration path.
4. **Rust-native.** Use Rust crates directly. No Python, no Node, no external runtimes.
5. **Agent-native.** Every feature is designed for AI agents first, humans second.

---

## How to Contribute

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development setup and guidelines.

High-impact areas for contributors:
- MCP server implementation (v0.1.1)
- Token budget algorithm accuracy
- New output format experiments
- Agent integration testing (Claude Code, Cursor, Codex)
