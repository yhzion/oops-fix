# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Renamed project from `didyoumean` to `oops-fix` (binary: `oops`)
- Environment variables renamed: `DYM_*` → `OOPS_*`
- Shell markers renamed: `# >>> oops-fix initialize >>>`
- Output prefix changed: `[dym]` → `[oops]`
- Cache directory: `~/.cache/oops-fix/`

### Added

- Self-update command: `oops update [--check]`
- Background update notification on command typo (cached, non-blocking, every 24h)
- SHA256 checksum verification on update
- Version cache at `~/.cache/oops-fix/latest-version`

## [0.2.0] - 2026-02-24

### Added

- 91 unit tests covering all pure logic (97% line coverage)
- 19 integration tests (stdin pipe, uninstall scenarios, env vars)
- Git pre-commit hook (cargo fmt, clippy, test)

### Changed

- Refactored main.rs into testable pure functions:
  `get_init_script`, `remove_init_block`, `build_uninstall_plan`,
  `confirm_action`, `format_suggest_result`, `parse_candidates`,
  `parse_env_usize`, `parse_auto_correct`, `parse_is_root`,
  `extract_shell_name`, `rc_file_for_shell`, `format_suggestions`

### Fixed

- install.sh `tmpdir: unbound variable` error in EXIT trap

## [0.1.0] - 2026-02-24

### Added

- Core suggestion engine using Damerau-Levenshtein distance (`strsim` crate)
- PATH scanner with executable filtering and deduplication
- Shell integration for zsh (`command_not_found_handler`) and bash (`command_not_found_handle`)
- Stdin candidate support (builtins, aliases, functions via shell pipe)
- CLI subcommands: `<command> [args...]`, `init <shell>`, `uninstall [-y]`, `--version`, `--help`
- Exit code protocol: 0 (auto-correct), 1 (suggestions), 2 (no match)
- High-confidence auto-correction: distance 1, unique match, length >= 3
- Opt-in auto-correction for lower confidence via `OOPS_AUTO_CORRECT=on` (disabled for root)
- Original arguments displayed in correction feedback and preserved on execution
- Automatic uninstall command with RC file cleanup and backup
- Color output on stderr with `NO_COLOR` support
- Detailed `--help` with examples and how-it-works explanation
- Configurable via environment variables (`OOPS_MAX_DISTANCE`, `OOPS_MAX_SUGGESTIONS`)
- One-line install script with OS/arch detection and SHA256 checksum verification
- CI workflow (fmt, clippy, test on Linux/macOS)
- Release workflow for cross-platform builds (x86_64/aarch64, Linux musl/macOS)
- MIT license

[Unreleased]: https://github.com/yhzion/oops-fix/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/yhzion/oops-fix/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yhzion/oops-fix/releases/tag/v0.1.0
