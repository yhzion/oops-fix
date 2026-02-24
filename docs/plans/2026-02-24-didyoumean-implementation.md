# didyoumean Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust CLI tool that hooks into shell's `command_not_found` handler to suggest or auto-correct mistyped commands using Damerau-Levenshtein distance.

**Architecture:** Single Rust binary that directly scans `$PATH` directories for executables, computes Damerau-Levenshtein distance against the mistyped command, and outputs results via exit codes (0=auto-correct, 1=suggest, 2=no match) with suggestions on stdout and messages on stderr. Shell integration via `eval "$(didyoumean init <shell>)"`.

**Tech Stack:** Rust, `strsim` crate (Damerau-Levenshtein), `std::env::args()` for CLI parsing (no clap), GitHub Actions for CI/CD.

---

### Task 1: Cargo.toml Setup + strsim Dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Configure Cargo.toml with strsim dependency and release profile**

```toml
[package]
name = "didyoumean"
version = "0.1.0"
edition = "2021"
description = "Shell command typo correction tool using Damerau-Levenshtein distance"
license = "MIT"
repository = "https://github.com/yhzion/didyoumean"
keywords = ["cli", "shell", "typo", "autocorrect", "command-not-found"]
categories = ["command-line-utilities"]

[dependencies]
strsim = "0.11"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: success

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: configure Cargo.toml with strsim dependency and release profile"
```

---

### Task 2: Core Suggestion Engine (`src/suggest.rs`)

**Files:**
- Create: `src/suggest.rs`

**Step 1: Write tests for suggestion engine**

Tests to include in `src/suggest.rs` (inline tests):
- `test_exact_match_excluded` — exact match is not suggested
- `test_distance_1_single_match` — returns auto-correct for single distance-1 match
- `test_distance_1_multiple_matches` — returns suggest when multiple distance-1 matches
- `test_no_match` — returns no-match for unrelated input
- `test_length_filter` — skips candidates with large length difference
- `test_max_suggestions` — limits to max_suggestions

**Step 2: Run tests to verify they fail**

Run: `cargo test suggest`
Expected: FAIL (module doesn't exist yet)

**Step 3: Implement suggest.rs**

Core structs and functions:
- `SuggestResult` enum: `AutoCorrect(String)`, `Suggestions(Vec<String>)`, `NoMatch`
- `suggest(cmd: &str, candidates: &[String], max_distance: usize, max_suggestions: usize) -> SuggestResult`
- Uses `strsim::damerau_levenshtein` for distance calculation
- Pre-filters by length difference before computing distance
- AutoCorrect only when: distance=1, uniquely best match

**Step 4: Run tests to verify they pass**

Run: `cargo test suggest`
Expected: PASS

**Step 5: Commit**

```bash
git add src/suggest.rs
git commit -m "feat: add core suggestion engine with Damerau-Levenshtein matching"
```

---

### Task 3: PATH Scanner (in `src/suggest.rs`)

**Files:**
- Modify: `src/suggest.rs`

**Step 1: Write tests for PATH scanning**

Tests:
- `test_scan_path_returns_executables` — scans a temp directory with executables
- `test_scan_path_skips_nonexistent_dirs` — doesn't error on missing dirs
- `test_scan_path_deduplicates` — removes duplicate command names

**Step 2: Run tests to verify they fail**

Run: `cargo test scan`
Expected: FAIL

**Step 3: Implement scan_path function**

- `scan_path() -> Vec<String>`
- Reads `$PATH` env var, splits by `:`
- For each directory: `std::fs::read_dir`, filter executables (check permission on unix)
- Deduplicates with `HashSet`
- Skips non-existent directories silently

**Step 4: Run tests to verify they pass**

Run: `cargo test scan`
Expected: PASS

**Step 5: Commit**

```bash
git add src/suggest.rs
git commit -m "feat: add PATH directory scanner with deduplication"
```

---

### Task 4: Shell Integration (`src/shell.rs`)

**Files:**
- Create: `src/shell.rs`

**Step 1: Write tests for shell init output**

Tests:
- `test_init_zsh_contains_handler` — output contains `command_not_found_handler`
- `test_init_bash_contains_handle` — output contains `command_not_found_handle`
- `test_init_zsh_has_markers` — output contains `>>> didyoumean` markers
- `test_init_has_reentry_guard` — output contains reentry guard variable
- `test_uninstall_outputs_instructions` — uninstall prints removal instructions

**Step 2: Run tests to verify they fail**

Run: `cargo test shell`
Expected: FAIL

**Step 3: Implement shell.rs**

Functions:
- `init_zsh() -> String` — returns zsh handler code with:
  - `# >>> didyoumean initialize >>>` / `# <<< didyoumean initialize <<<` markers
  - Reentry guard (`__DYM_RUNNING`)
  - Exit code based dispatch (0→execute, 1→show suggestions, 2→standard not found)
  - `"$@"` for safe argument passing
  - stderr message display
- `init_bash() -> String` — same pattern with `command_not_found_handle` (no r)
- `uninstall_instructions() -> String` — prints manual removal steps

**Step 4: Run tests to verify they pass**

Run: `cargo test shell`
Expected: PASS

**Step 5: Commit**

```bash
git add src/shell.rs
git commit -m "feat: add shell integration (zsh/bash init, uninstall instructions)"
```

---

### Task 5: CLI Entry Point (`src/main.rs`)

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement main.rs with subcommand routing**

- Parse `std::env::args()` directly
- Route to:
  - `didyoumean <cmd>` → scan PATH, compute suggestions, output via exit codes
  - `didyoumean init <zsh|bash>` → print shell handler code
  - `didyoumean uninstall` → print removal instructions
  - `didyoumean --version` → print version from Cargo.toml
  - `didyoumean --help` → print usage
- Color output on stderr:
  - Yellow bold for auto-correct
  - Green for suggestions
  - Respect `NO_COLOR` env var and non-TTY detection
- Auto-correct gating:
  - Check `DYM_AUTO_CORRECT` env var (default: off)
  - Check `EUID` != 0 (no auto-correct as root)
- Read `DYM_MAX_DISTANCE`, `DYM_MAX_SUGGESTIONS` env vars with defaults
- Exit codes: 0 (auto-correct), 1 (suggestions), 2 (no match)
- English default messages, detect `$LANG` for Korean (`ko_KR`)

**Step 2: Build and test manually**

Run: `cargo build && echo "git" | ./target/debug/didyoumean gti`
Expected: suggestions output

**Step 3: Run all tests**

Run: `cargo test`
Expected: all PASS

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add CLI entry point with subcommand routing and color output"
```

---

### Task 6: Install Script (`install.sh`)

**Files:**
- Create: `install.sh`

**Step 1: Write install.sh**

Requirements:
- `set -euo pipefail` + `main()` function pattern (prevents partial download execution)
- curl/wget fallback chain
- OS detection: `uname -s` → linux/darwin
- Arch detection: `uname -m` → x86_64/aarch64 (with Rosetta 2 detection on macOS)
- Download binary from GitHub Releases to `~/.local/bin/didyoumean`
- SHA256 checksum verification
- Ensure `~/.local/bin` is in PATH (add to RC file if not)
- Detect existing didyoumean installation → warn
- Detect existing `command_not_found_handler` → warn
- Add `eval "$(didyoumean init ...)"` to RC file with marker comments
- Idempotent (skip if markers already exist)
- RC file backup before modification
- Detect current shell from `$SHELL`

**Step 2: Test locally (dry-run style review)**

Run: `bash -n install.sh` (syntax check)
Expected: no errors

**Step 3: Commit**

```bash
git add install.sh
git commit -m "feat: add one-line install script with checksum verification"
```

---

### Task 7: CI/CD Workflows

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`

**Step 1: Write ci.yml**

- Trigger: push to main, PRs
- Matrix: ubuntu-latest, macos-latest
- Steps: cargo fmt --check, cargo clippy, cargo test

**Step 2: Write release.yml**

- Trigger: tag push `v*`
- Build matrix: 4 targets (x86_64-linux-musl, aarch64-linux-musl, x86_64-apple-darwin, aarch64-apple-darwin)
- Use `cross` for Linux musl targets
- Native build for macOS targets (macos-13 for x86, macos-14 for arm)
- Generate SHA256SUMS file
- Create draft GitHub Release with all artifacts + SHA256SUMS
- `generate_release_notes: true`

**Step 3: Commit**

```bash
git add .github/workflows/
git commit -m "ci: add CI and release workflows"
```

---

### Task 8: README.md

**Files:**
- Create: `README.md`

**Step 1: Write README.md**

Sections:
1. Title + one-line description + badges (crates.io, CI, license)
2. GIF demo placeholder
3. Features (bullet list)
4. Install (one-liner + manual cargo install + SHA256 verification note)
5. Setup (`eval "$(didyoumean init zsh)"`)
6. Examples (suggest mode / auto-correct mode / no match)
7. Configuration (env vars table)
8. How it works (1 paragraph: command_not_found_handler hooking)
9. Supported environments (OS/shell/arch matrix table)
10. Comparison with thefuck/pay-respects (1-2 lines)
11. Uninstall
12. Building from source
13. Contributing
14. License

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add comprehensive README"
```

---

### Task 9: LICENSE + Initial Push

**Files:**
- Create: `LICENSE`

**Step 1: Create MIT LICENSE file**

**Step 2: Create GitHub repository**

Run: `gh repo create yhzion/didyoumean --public --source=. --remote=origin --description "Shell command typo correction tool"`

**Step 3: Push all commits**

Run: `git push -u origin main`

---

### Task 10: Integration Test (End-to-End)

**Step 1: Build release binary**

Run: `cargo build --release`

**Step 2: Test suggest mode**

Run: `./target/release/didyoumean gti`
Expected: exit code 1, stdout has "git", stderr has "[dym]" message

**Step 3: Test no-match mode**

Run: `./target/release/didyoumean xyzabcdef123`
Expected: exit code 2, stderr message only

**Step 4: Test init zsh output**

Run: `./target/release/didyoumean init zsh`
Expected: shell handler code with markers

**Step 5: Test init bash output**

Run: `./target/release/didyoumean init bash`
Expected: shell handler code with markers

**Step 6: Test --version**

Run: `./target/release/didyoumean --version`
Expected: `didyoumean 0.1.0`

**Step 7: Test --help**

Run: `./target/release/didyoumean --help`
Expected: usage text
