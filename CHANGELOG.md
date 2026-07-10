# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
