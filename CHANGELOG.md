# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Core suggestion engine using Damerau-Levenshtein distance (`strsim` crate)
- PATH scanner with executable filtering and deduplication
- Shell integration for zsh (`command_not_found_handler`) and bash (`command_not_found_handle`)
- CLI subcommands: `<command>`, `init <shell>`, `uninstall`, `--version`, `--help`
- Exit code protocol: 0 (auto-correct), 1 (suggestions), 2 (no match)
- Auto-correction mode (opt-in via `DYM_AUTO_CORRECT=on`, disabled for root)
- Color output on stderr with `NO_COLOR` support
- Korean/English message auto-detection from `$LANG`
- Configurable via environment variables (`DYM_MAX_DISTANCE`, `DYM_MAX_SUGGESTIONS`)
- One-line install script with OS/arch detection and SHA256 checksum verification
- CI workflow (fmt, clippy, test on Linux/macOS)
- Release workflow for cross-platform builds (x86_64/aarch64, Linux musl/macOS)
- MIT license

[Unreleased]: https://github.com/yhzion/didyoumean/compare/main...HEAD
