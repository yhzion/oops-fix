# didyoumean

Shell command typo correction tool using Damerau-Levenshtein distance.

[![CI](https://github.com/yhzion/didyoumean/actions/workflows/ci.yml/badge.svg)](https://github.com/yhzion/didyoumean/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

<!-- TODO: Add GIF demo -->

## Features

- Suggests similar commands when you mistype in the shell
- Optional auto-correction mode (opt-in via `DYM_AUTO_CORRECT=on`)
- Fast single binary — scans `$PATH` and computes Damerau-Levenshtein distance
- Supports zsh and bash via `command_not_found` hook
- Korean and English message support (auto-detected from `$LANG`)
- No auto-correct as root for safety
- Respects `NO_COLOR` environment variable

## Install

**One-liner:**

```bash
curl -sSfL https://raw.githubusercontent.com/yhzion/didyoumean/main/install.sh | bash
```

**From source:**

```bash
cargo install --git https://github.com/yhzion/didyoumean
```

**Manual download:**

Download the binary for your platform from [GitHub Releases](https://github.com/yhzion/didyoumean/releases), verify the SHA256 checksum, and place it in your `$PATH`.

## Setup

Add to your shell config file (`.zshrc` or `.bashrc`):

```bash
eval "$(didyoumean init zsh)"   # for zsh
eval "$(didyoumean init bash)"  # for bash
```

## Examples

**Suggestion mode** (default):
```
$ gti status
[dym] Did you mean 'git'?
```

**Auto-correct mode** (`DYM_AUTO_CORRECT=on`):
```
$ export DYM_AUTO_CORRECT=on
$ gti status
[dym] Correcting 'gti' to 'git'
On branch main
...
```

**No match:**
```
$ xyzabc123
[dym] Command 'xyzabc123' not found, no similar commands
```

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DYM_AUTO_CORRECT` | Enable auto-correction (`on`/`1`/`true`) | `off` |
| `DYM_MAX_DISTANCE` | Maximum Damerau-Levenshtein distance | `2` |
| `DYM_MAX_SUGGESTIONS` | Maximum number of suggestions | `5` |
| `NO_COLOR` | Disable color output (any value) | unset |

## How It Works

When you type a command that doesn't exist, the shell calls `command_not_found_handler` (zsh) or `command_not_found_handle` (bash). `didyoumean` hooks into this function, scans all executables in your `$PATH`, computes the Damerau-Levenshtein distance against your input, and returns the closest matches. Exit codes tell the shell handler what to do: 0 = auto-correct (execute the suggestion), 1 = show suggestions, 2 = no match found.

## Supported Environments

| OS | Shell | Architecture |
|---|---|---|
| Linux | zsh, bash | x86_64, aarch64 |
| macOS | zsh, bash | x86_64 (Intel), aarch64 (Apple Silicon) |

## Comparison

| Tool | Language | Approach |
|---|---|---|
| **didyoumean** | Rust | `command_not_found` hook, single binary, zero config |
| thefuck | Python | Post-execution correction, requires Python runtime |
| pay-respects | Rust | Similar approach, more features |

## Uninstall

```bash
didyoumean uninstall
```

## Building from Source

```bash
git clone https://github.com/yhzion/didyoumean
cd didyoumean
cargo build --release
# Binary at target/release/didyoumean
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes
4. Push to the branch and open a Pull Request

## License

MIT
