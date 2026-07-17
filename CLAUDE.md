# CLAUDE.md — rustygrep

## Project Overview

Fast parallel grep with AI-native token-compressed output. Built on ripgrep crates (`grep-regex`, `grep-searcher`, `ignore`) + `rayon` for parallelism. Ships as a single binary with zero config.

**Current version:** 0.1.1 (shipped 2026-07-17)

## Build & Test

```bash
cargo build            # debug build
cargo build --release  # optimized (LTO + strip, ~2.4MB)
cargo test             # run tests
cargo clippy           # lint
cargo fmt              # format
cargo bench            # benchmarks (criterion)
```

## Architecture

```
src/
├── main.rs          # Entry: parse CLI → walker → search → output
├── cli.rs           # Clap CLI flags (--llm, --json, --top, -t, etc.)
├── search.rs        # SearchEngine: parallel regex search → Vec<FileMatches>
├── walker.rs        # FileWalker: gitignore-aware file discovery
├── mcp.rs           # JSON-RPC MCP server over stdio (3 tools)
└── output/
    ├── mod.rs       # Routes to llm/json/pretty
    ├── llm.rs       # Token-compressed output + budget support
    ├── json.rs      # JSONL per-match or per-file output
    └── pretty.rs    # Colored terminal output
```

**Data flow:** `Cli::parse()` → `FileWalker::walk()` → `SearchEngine::search()` → `output::print_results()`

**Key structs:**
- `SearchEngine` — holds `RegexMatcher`, context settings, limits
- `FileMatches` — per-file results with `Vec<Match>`
- `Match` — single match: path, line_number, line, submatches
- `LlmOptions` — truncate flag, max_line_chars, budget_tokens

## Dependencies

Minimal by design. No external MCP library — hand-rolled JSON-RPC with `serde_json`.

| Crate | Purpose |
|-------|---------|
| `clap` (derive) | CLI parsing |
| `grep-regex`, `grep-searcher`, `grep-matcher` | Ripgrep's search engine |
| `ignore` | Gitignore-aware file walking |
| `rayon` | Parallel iteration |
| `serde` + `serde_json` | JSON output + MCP protocol |
| `colored` | Terminal colors |
| `memchr` | SIMD-accelerated byte search |

## Handoff Notes

### What's Done (v0.1.1)

- MCP server (`rustygrep mcp`) — 3 tools: search, files, count
- `--llm` output: per-file match counts, 120-char truncation, summary line
- `--llm-budget N` — cap output at N tokens (4 chars ≈ 1 token)
- `--top N` — rank files by match count
- `--json` = JSONL per-match, `--json-file` = per-file
- Exit codes match ripgrep (0=match, 1=no-match, 2=error)
- RTK integration documented

### What's Next (v0.2.0)

1. **BM25-lite ranking** — `--rank` scores definitions > tests > comments
2. **`rustygrep init`** — one command to set up agent hooks (Claude Code, Cursor, Copilot)
3. **Fuzzy matching** — `--fuzzy N` allows N character differences
4. **Git context** — `--since`, `--branch`, `--diff` (search only changed files)

### What NOT to Build

- **Trigram index** — v0.3.0 territory. ig/grix own this. Our moat is zero-config.
- **Daemon mode** — We're the no-daemon option.
- **Tree-sitter / structural search** — Belongs in hypergrep/ffs.
- **Semantic search** — Needs embeddings. Different product.

### Testing Notes

- Benchmarks in `benches/search.rs` use criterion + tempfile
- Test data is generated (synthetic Rust files), not real repos
- No integration tests yet — add real-world tests before v0.2.0

### Release Process

```bash
cargo release patch   # or minor/major
# CI builds binaries for: macOS ARM64/x86_64, Linux ARM64/x86_64, Windows x86_64
# Binaries go to GitHub Releases + crates.io
```
