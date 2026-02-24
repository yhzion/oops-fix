# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Core suggestion engine using Damerau-Levenshtein distance (`strsim` crate)
- PATH scanner with executable filtering and deduplication
- Shell integration for zsh (`command_not_found_handler`) and bash (`command_not_found_handle`)
- Stdin candidate support (builtins, aliases, functions via shell pipe)
- CLI subcommands: `<command> [args...]`, `init <shell>`, `uninstall [-y]`, `--version`, `--help`
- Exit code protocol: 0 (auto-correct), 1 (suggestions), 2 (no match)
- High-confidence auto-correction: distance 1, unique match, length >= 3
- Opt-in auto-correction for lower confidence via `DYM_AUTO_CORRECT=on` (disabled for root)
- Original arguments displayed in correction feedback and preserved on execution
- Automatic uninstall command with RC file cleanup and backup
- Color output on stderr with `NO_COLOR` support
- Detailed `--help` with examples and how-it-works explanation
- Configurable via environment variables (`DYM_MAX_DISTANCE`, `DYM_MAX_SUGGESTIONS`)
- One-line install script with OS/arch detection and SHA256 checksum verification
- CI workflow (fmt, clippy, test on Linux/macOS)
- Release workflow for cross-platform builds (x86_64/aarch64, Linux musl/macOS)
- MIT license

[Unreleased]: https://github.com/yhzion/didyoumean/compare/v0.1.0...HEAD
