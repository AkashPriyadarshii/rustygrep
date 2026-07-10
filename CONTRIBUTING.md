# Contributing to rustygrep

Thank you for your interest in contributing to rustygrep! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork
3. Create a new branch for your feature/fix
4. Make your changes
5. Submit a pull request

## Development Setup

```bash
# Clone your fork
git clone https://github.com/AkashPriyadarshii/rustygrep
cd rustygrep

# Build
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use `cargo clippy` to check for lints
- Write tests for new functionality
- Update documentation for public APIs

## Pull Request Process

1. Ensure your code compiles without warnings
2. Add tests for new features
3. Update README.md if adding new CLI flags
4. Keep commits focused and well-described
5. Reference any related issues

## Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Include steps to reproduce for bugs
- Include your OS, Rust version, and rustygrep version

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
