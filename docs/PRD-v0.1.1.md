# PRD: rustygrep v0.1.1

**Author:** rustygrep team
**Date:** 2026-07-10
**Status:** Draft
**Target release:** v0.1.1

---

## 1. Problem Statement

rustygrep v0.1.0 shipped a fast parallel grep built on ripgrep crates with a basic `--llm` output flag. However, the AI-native grep space has exploded in 2025-2026. Tools like RTK (69K stars), instant-grep, grix, and hypergrep offer trigram indexes, MCP servers, relevance ranking, token budgets, and structural search. rustygrep's `--llm` flag is a simple file-header + line-truncation formatter — not a defensible differentiator.

**Goal:** Make rustygrep the simplest, fastest zero-config grep+LLM tool. Not the most feature-rich — the one you reach for first.

## 2. Target Users

| User | Need |
|------|------|
| **AI coding agents** (Claude Code, Codex, Cursor, OpenCode) | Token-compressed search output that fits context windows |
| **Developers using AI agents** | A grep that agents can call without burning 80% of context on raw output |
| **CLI power users** | Fast, no-daemon, no-index grep with clean output options |

## 3. Success Metrics

| Metric | Target |
|--------|--------|
| Token savings vs raw grep output | ≥ 70% (measured, not estimated) |
| Binary size | ≤ 3MB |
| Search latency (small repo < 1K files) | < 50ms |
| Zero-config installs | `cargo install rustygrep` → works |
| Agent integrations | MCP server + RTK hook support |

## 4. Features (v0.1.1)

### 4.1 MCP Server (P0 — Table Stakes)

Without an MCP server, no AI agent will integrate rustygrep natively.

**What:** `rustygrep mcp` starts a Model Context Protocol server over stdio.

**Tools exposed:**
| Tool | Description |
|------|-------------|
| `rustygrep_search` | Pattern search with format options (pretty/json/llm) |
| `rustygrep_files` | List matching files only |
| `rustygrep_count` | Match counts per file |

**Implementation:**
- Use `rmcp` or `tower-lsp` crate for MCP protocol
- Single binary, no external dependencies
- Default to `--llm` format in MCP responses

**Acceptance criteria:**
- [ ] `rustygrep mcp` starts and responds to `initialize` request
- [ ] `rustygrep_search` returns results in MCP tool response format
- [ ] Works with Claude Code, Cursor, OpenCode via MCP config

### 4.2 Improved LLM Output (P0)

**Current behavior:** `--llm` shows `--- file` headers + `line:content` lines, truncated at 200 chars, summary at end.

**New behavior:**

```
--- src/main.rs (3 matches)
10:fn main() {
15:    let cli = Cli::parse();
20:    process::exit(0);

--- src/search.rs (5 matches)
37:    pub fn new(cli: &Cli) -> Result<Self, Box<dyn std::error::Error>> {
64:    pub fn search(&self, files: &[PathBuf]) -> Vec<FileMatches> {
80:    fn search_file(&self, path: &Path) -> Option<FileMatches> {
97:        let _ = searcher.search_path(
141:        );

--- 8 matches in 2 files
```

**Changes:**
- Per-file match count in header: `--- file (N matches)`
- Default line truncation at 120 chars (was 200)
- Summary line: `--- N matches in M files` (was `--- N matches in M files`)
- Add `--llm-budget N` flag: cap total output at N tokens (heuristic: 4 chars ≈ 1 token)

**Acceptance criteria:**
- [ ] Per-file match count in `---` headers
- [ ] Default truncation at 120 chars
- [ ] `--llm-budget 500` caps output to ~500 tokens
- [ ] `--llm-no-truncate` disables line truncation (old behavior)

### 4.3 Match Ranking (P1)

**What:** `--top N` flag ranks results by match density (matches per file, descending).

**Use case:** Agent searches for "error" across 200 files. Without ranking, it gets files alphabetically. With `--top 10`, it gets the 10 files with the most "error" matches first.

**Implementation:**
- After parallel search, sort `FileMatches` by `total_matches` descending
- Return only top N files
- Works with all output formats (pretty, json, llm)

**Acceptance criteria:**
- [ ] `rustygrep "error" --top 10` returns 10 files sorted by match count
- [ ] Works with `--llm`, `--json`, and default pretty format
- [ ] `--top 0` means unlimited (default, backward compatible)

### 4.4 RTK Integration (P1)

**What:** Make rustygrep work seamlessly with RTK's auto-rewrite hooks.

**Implementation:**
- Ensure `rustygrep "pattern" .` output is clean enough for RTK to pass through
- Document RTK integration in README
- Match ripgrep's exit codes exactly (0=match, 1=no-match, 2=error)

**Acceptance criteria:**
- [ ] `rtk grep "pattern"` works when rustygrep is installed (rename binary to `rg` or symlink)
- [ ] Exit codes match ripgrep semantics
- [ ] README documents RTK setup

### 4.5 JSON Output Improvements (P2)

**Current:** One JSON object per file, matches nested inside.

**New:** Add `--json-lines` (or make `--json` produce one object per match line for piping to `jq`).

```jsonl
{"path":"src/main.rs","line":10,"match":"fn main()","submatches":[[0,7]]}
{"path":"src/main.rs","line":15,"match":"let cli = Cli::parse();","submatches":[[8,20]]}
```

**Acceptance criteria:**
- [ ] `--json` produces JSONL (one object per match line, not per file)
- [ ] `--json-file` produces the old per-file format (backward compat)
- [ ] Valid JSONL that `jq` can process line-by-line

## 5. Non-Goals (v0.1.1)

These are explicitly NOT in scope for v0.1.1:

| Feature | Why not |
|---------|---------|
| Trigram index | Massive scope. ig/grix already own this. Our niche is zero-config. |
| Daemon mode | Adds complexity. We're the no-daemon option. |
| Tree-sitter / AST search | Belongs in hypergrep/ffs, not a grep tool |
| Semantic search | Needs embeddings, models. Different product category. |
| Build log compression | Belongs in RTK/semble_rs |
| File watching | Not needed for a grep tool |

## 6. Technical Architecture

```
rustygrep mcp          # New: MCP server (stdio JSON-RPC)
    │
    ├── cli.rs         # Add --top, --llm-budget, --llm-no-truncate, --json-lines
    ├── mcp.rs         # New: MCP protocol handler
    ├── search.rs      # Unchanged (parallel grep-regex + rayon)
    ├── walker.rs      # Unchanged (ignore crate)
    └── output/
        ├── mod.rs     # Updated: route new formats
        ├── llm.rs     # Updated: budget mode, per-file counts, truncation
        ├── json.rs    # Updated: JSONL mode
        └── pretty.rs  # Unchanged
```

**New dependencies:**
- `rmcp` (MCP protocol) or `serde_json` + manual JSON-RPC (lighter)

**No new dependencies preferred.** If MCP can be implemented with just `serde_json` + `stdin/stdout`, that's the path.

## 7. Migration / Breaking Changes

- `--json` output format changes from per-file to per-match-line (JSONL)
- Old per-file format available via `--json-file`
- This is a minor version bump (0.1.0 → 0.1.1) — breaking change in `--json` is acceptable at this stage

## 8. Timeline

| Week | Deliverable |
|------|-------------|
| Week 1 | MCP server (`rustygrep mcp`) — basic search tool working |
| Week 2 | Improved `--llm` output: budget mode, per-file counts, truncation |
| Week 3 | `--top N` ranking, `--json-lines` format |
| Week 4 | RTK integration docs, README update, testing, release |

## 9. Risks

| Risk | Mitigation |
|------|-----------|
| MCP protocol complexity | Start with minimal implementation (search tool only). Expand later. |
| `--json` breaking change | Document clearly. `--json-file` provides backward compat. |
| Competition moves fast | Focus on simplicity as the moat. Don't chase features. |
| Binary size growth from MCP deps | Use manual JSON-RPC if `rmcp` adds >500KB |

## 10. Open Questions

1. Should we support MCP `resources` (expose file contents) or just `tools` (search)?
2. Should `--llm-budget` use a simple heuristic (4 chars ≈ 1 token) or integrate `tiktoken-rs`?
3. Do we want a `rustygrep init` command that sets up agent hooks (like RTK's `rtk init`)?
