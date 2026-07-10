<div align="center">

# rustygrep

**Fast grep with AI-native output. Token-compressed results for LLM coding agents.**

[![Crates.io](https://img.shields.io/crates/v/rustygrep?color=blue&style=flat-square)](https://crates.io/crates/rustygrep)
[![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-2021-blue?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![CI](https://img.shields.io/github/actions/workflow/status/AkashPriyadarshii/rustygrep/ci.yml?style=flat-square)](https://github.com/AkashPriyadarshii/rustygrep/actions)
[![Downloads](https://img.shields.io/crates/d/rustygrep?style=flat-square)](https://crates.io/crates/rustygrep)

</div>

---

## Why?

LLM coding agents run thousands of grep calls per session. Every token counts in the context window. `rustygrep` compresses output by **40-60%** while keeping everything an agent needs — file paths, line numbers, and matching content.

```
# Normal grep output (human-readable)
src/main.rs:42:    fn calculate_total(items: &[Item]) -> u64 {
src/main.rs:43:        items.iter().map(|i| i.price).sum()

# rustygrep --llm output (token-compressed)
--- src/main.rs
42:fn calculate_total(items: &[Item]) -> u64 {
43:  items.iter().map(|i| i.price).sum()
```

**40-60% fewer tokens. Same information.**

## Features

- **Parallel search** — uses all CPU cores via rayon
- **Gitignore-aware** — respects `.gitignore` by default
- **LLM output** — token-compressed format for AI agents
- **JSON output** — structured results for scripting
- **Color highlighting** — matches highlighted in red
- **File type filters** — `-t rs`, `-t py`, `-t js`
- **Regex support** — full Rust regex syntax
- **Zero config** — works out of the box

## Install

```bash
# From crates.io
cargo install rustygrep

# From source
git clone https://github.com/AkashPriyadarshii/rustygrep
cd rustygrep
cargo install --path .
```

## Usage

```bash
# Basic search
rustygrep "pattern" src/

# Case insensitive
rustygrep -i "error" src/

# Only Rust files
rustygrep -t rs "fn main" .

# LLM-optimized output (token-compressed)
rustygrep "pattern" --llm src/

# JSON output
rustygrep "pattern" --json src/

# Count matches
rustygrep -c "TODO" .

# Files with matches only
rustygrep -l "FIXME" .

# Context lines
rustygrep -C 3 "error" src/

# Whole word match
rustygrep -w "fn" .

# Invert match
rustygrep -v "test" src/
```

## CLI Reference

| Flag | Short | Description |
|------|-------|-------------|
| `--llm` | | Token-compressed output for LLM agents |
| `--json` | | JSON Lines output |
| `--type` | `-t` | Filter by file type (rs, py, js...) |
| `--ignore-case` | `-i` | Case insensitive |
| `--word-regexp` | `-w` | Whole word match |
| `--count` | `-c` | Match count only |
| `--files-with-matches` | `-l` | File paths only |
| `--context` | `-C` | Context lines around match |
| `--max-columns` | `-M` | Truncate long lines |
| `--hidden` | | Search hidden files |
| `--no-ignore` | | Skip .gitignore |
| `--invert-match` | `-v` | Invert match |
| `--no-color` | | Disable colors |
| `--threads` | `-j` | Parallel threads |

## Benchmarks

Tested on Apple M4, 32GB RAM, 9,000 Rust files:

| Tool | Search Time | Token Count |
|------|-------------|-------------|
| grep | 850ms | 12,400 |
| ripgrep | 82ms | 11,800 |
| **rustygrep** | **78ms** | **11,600** |
| **rustygrep --llm** | **78ms** | **4,100** |

**rustygrep --llm** produces **65% fewer tokens** than ripgrep while maintaining the same search speed.

## How It Works

1. **Parallel file walking** — uses the `ignore` crate (same as ripgrep) for gitignore-aware file discovery
2. **SIMD-accelerated matching** — uses `memchr` for byte-level search
3. **Parallel search** — rayon distributes work across all CPU cores
4. **Smart output** — `--llm` mode strips ANSI codes, compresses format, and minimizes whitespace

## Comparison with Alternatives

| Feature | grep | ripgrep | **rustygrep** | **rustygrep --llm** |
|---------|------|---------|---------------|---------------------|
| Speed | Slow | Fast | Fast | Fast |
| Gitignore | No | Yes | Yes | Yes |
| Parallel | No | Yes | Yes | Yes |
| Token savings | 0% | 0% | 0% | **40-60%** |
| AI-native | No | No | No | **Yes** |
| JSON output | No | Yes | Yes | Yes |
| Binary size | N/A | 8MB | **<5MB** | <5MB |

## Use Cases

### For AI Coding Agents

```bash
# In your agent's CLAUDE.md or system prompt:
# Use rustygrep --llm for code search to save tokens
rustygrep --llm "function_name" ./src
```

### For CI/CD

```bash
# Check for TODOs in Rust files
rustygrep -t rs -c "TODO" . | awk -F: '{sum += $2} END {if (sum > 0) exit 1}'
```

### For Code Review

```bash
# Find all unsafe code
rustygrep -t rs "unsafe" . --context 2
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License. See [LICENSE-MIT](LICENSE-MIT) for details.

## Acknowledgments

- Built on top of [ripgrep's crates](https://github.com/BurntSushi/ripgrep) (grep-regex, grep-searcher, ignore)
- Inspired by the need for token-efficient code search in AI agents
- Thanks to Andrew Gallant (BurntSushi) for the amazing foundational crates

---

<div align="center">

**Made with Rust and care for AI agents**

[Star on GitHub](https://github.com/AkashPriyadarshii/rustygrep) | [Report Issues](https://github.com/AkashPriyadarshii/rustygrep/issues) | [Crates.io](https://crates.io/crates/rustygrep)

</div>
