# Rename Migration: didyoumean → oops-fix

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename the entire project from `didyoumean` to `oops-fix` (crate) / `oops` (binary) in a single commit with all tests passing.

**Architecture:** Mechanical string replacement across all source files, tests, CI, docs, and install script. No logic changes. The suggestion engine (`src/suggest.rs`) is untouched except test temp dir names.

**Tech Stack:** Rust, cargo, GitHub Actions, bash (install.sh)

---

## Naming Convention Reference

| Context | Before | After |
|---|---|---|
| Crate name | `didyoumean` | `oops-fix` |
| Binary name | `didyoumean` | `oops` |
| Output prefix | `[dym]` | `[oops]` |
| Env var prefix | `DYM_` | `OOPS_` |
| Shell reentry guard | `__DYM_RUNNING` | `__OOPS_RUNNING` |
| Shell markers | `# >>> didyoumean initialize >>>` | `# >>> oops-fix initialize >>>` |
| Backup extension | `.dym.bak` | `.oops.bak` |
| Cache directory | `~/.cache/didyoumean/` | `~/.cache/oops-fix/` |
| GitHub repo | `yhzion/didyoumean` | `yhzion/oops-fix` |
| Release artifact | `didyoumean-{target}.tar.gz` | `oops-fix-{target}.tar.gz` |
| install.sh vars | `DYM_TMPDIR`, `DYM_VERSION`, `DYM_INSTALL_DIR` | `OOPS_TMPDIR`, `OOPS_VERSION`, `OOPS_INSTALL_DIR` |
| Test temp dirs | `dym_test_*`, `dym_uninstall_*` | `oops_test_*`, `oops_uninstall_*` |
| Temp dir (update) | `dym_update_` | `oops_update_` |

---

### Task 1: Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update Cargo.toml**

```toml
[package]
name = "oops-fix"
version = "0.2.0"
edition = "2021"
description = "Shell command typo correction tool using Damerau-Levenshtein distance"
license = "MIT"
repository = "https://github.com/yhzion/oops-fix"
keywords = ["cli", "shell", "typo", "autocorrect", "command-not-found"]
categories = ["command-line-utilities"]

[[bin]]
name = "oops"
path = "src/main.rs"

[dependencies]
strsim = "0.11"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

**Step 2: Verify syntax**

Run: `cargo check 2>&1 | head -5`
Expected: may fail due to string references in code, that's OK at this stage

---

### Task 2: src/shell.rs

**Files:**
- Modify: `src/shell.rs`

**Step 1: Apply all replacements**

Replacements (order matters):
1. `# >>> didyoumean initialize >>>` → `# >>> oops-fix initialize >>>`
2. `# <<< didyoumean initialize <<<` → `# <<< oops-fix initialize <<<`
3. `| didyoumean ` → `| oops ` (in shell pipe commands)
4. `__DYM_RUNNING` → `__OOPS_RUNNING`

Full file after changes:

```rust
pub fn init_zsh() -> String {
    r#"# >>> oops-fix initialize >>>
command_not_found_handler() {
    if [[ -n "$__OOPS_RUNNING" ]]; then
        echo "zsh: command not found: $1" >&2
        return 127
    fi
    export __OOPS_RUNNING=1

    local output exit_code
    output=$(print -l ${(ko)commands} | oops "$@")
    exit_code=$?

    case $exit_code in
        0)
            "$output" "${@:2}"
            local ret=$?
            unset __OOPS_RUNNING
            return $ret
            ;;
        1|2)
            unset __OOPS_RUNNING
            return 127
            ;;
        *)
            unset __OOPS_RUNNING
            echo "zsh: command not found: $1" >&2
            return 127
            ;;
    esac
}
# <<< oops-fix initialize <<<"#
        .to_string()
}

pub fn init_bash() -> String {
    r#"# >>> oops-fix initialize >>>
command_not_found_handle() {
    if [[ -n "$__OOPS_RUNNING" ]]; then
        echo "bash: $1: command not found" >&2
        return 127
    fi
    export __OOPS_RUNNING=1

    local output exit_code
    output=$(compgen -c | sort -u | oops "$@")
    exit_code=$?

    case $exit_code in
        0)
            "$output" "${@:2}"
            local ret=$?
            unset __OOPS_RUNNING
            return $ret
            ;;
        1|2)
            unset __OOPS_RUNNING
            return 127
            ;;
        *)
            unset __OOPS_RUNNING
            echo "bash: $1: command not found" >&2
            return 127
            ;;
    esac
}
# <<< oops-fix initialize <<<"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_zsh_contains_handler() {
        let output = init_zsh();
        assert!(output.contains("command_not_found_handler"));
    }

    #[test]
    fn test_init_bash_contains_handle() {
        let output = init_bash();
        assert!(output.contains("command_not_found_handle()"));
    }

    #[test]
    fn test_init_zsh_has_markers() {
        let output = init_zsh();
        assert!(output.contains("# >>> oops-fix initialize >>>"));
        assert!(output.contains("# <<< oops-fix initialize <<<"));
    }

    #[test]
    fn test_init_has_reentry_guard() {
        let zsh = init_zsh();
        let bash = init_bash();
        assert!(zsh.contains("__OOPS_RUNNING"));
        assert!(bash.contains("__OOPS_RUNNING"));
    }
}
```

---

### Task 3: src/main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Apply all replacements**

Replacements:
1. `"Error: specify shell type (zsh or bash)\nUsage: didyoumean init <zsh|bash>"` → `"Error: specify shell type (zsh or bash)\nUsage: oops init <zsh|bash>"`
2. `"# >>> didyoumean initialize >>>"` → `"# >>> oops-fix initialize >>>"` (all occurrences)
3. `"# <<< didyoumean initialize <<<"` → `"# <<< oops-fix initialize <<<"`
4. `"{}/.local/bin/didyoumean"` → `"{}/.local/bin/oops"`
5. `"(didyoumean block)"` → `"(oops-fix block)"`
6. `"Removed didyoumean block"` → `"Removed oops-fix block"`
7. `"No didyoumean block"` → `"No oops-fix block"`
8. `"didyoumean uninstalled"` → `"oops uninstalled"`
9. `"didyoumean {}"` (version) → `"oops {}"`
10. `".dym.bak"` → `".oops.bak"`
11. All `[dym]` → `[oops]` in format strings
12. `DYM_AUTO_CORRECT` → `OOPS_AUTO_CORRECT`
13. `DYM_MAX_DISTANCE` → `OOPS_MAX_DISTANCE`
14. `DYM_MAX_SUGGESTIONS` → `OOPS_MAX_SUGGESTIONS`
15. Help text: all `didyoumean` → `oops` (in command examples)
16. Test strings: `"didyoumean"` → `"oops"` in test vec args
17. Test assertions: `"/home/user/.local/bin/didyoumean"` → `"/home/user/.local/bin/oops"`
18. Test assertions: `"/usr/local/bin/didyoumean"` → `"/usr/local/bin/oops"`

**Note:** The help_text() function must be fully rewritten with new naming. The `OOPS_AUTO_CORRECT`, `OOPS_MAX_DISTANCE`, `OOPS_MAX_SUGGESTIONS` env var names replace all `DYM_*` variants throughout.

---

### Task 4: src/update.rs

**Files:**
- Modify: `src/update.rs`

**Step 1: Apply all replacements**

Replacements:
1. `"https://api.github.com/repos/yhzion/didyoumean/releases/latest"` → `"https://api.github.com/repos/yhzion/oops-fix/releases/latest"`
2. `.join("didyoumean")` (cache path) → `.join("oops-fix")`
3. `".cache/didyoumean"` → `".cache/oops-fix"`
4. `"[dym]"` → `"[oops]"`
5. `"Run 'didyoumean update'"` → `"Run 'oops update'"`
6. `"didyoumean update [--check]"` (doc comment) → `"oops update [--check]"`
7. `"https://raw.githubusercontent.com/yhzion/didyoumean/main/install.sh"` → `"https://raw.githubusercontent.com/yhzion/oops-fix/main/install.sh"`
8. `"https://github.com/yhzion/didyoumean/releases/download/{}"` → `"https://github.com/yhzion/oops-fix/releases/download/{}"`
9. `"didyoumean-{}.tar.gz"` → `"oops-fix-{}.tar.gz"`
10. `"Updating didyoumean"` → `"Updating oops"`
11. `"dym_update_{}"` → `"oops_update_{}"`
12. `tmpdir.join("didyoumean")` (extracted binary) → `tmpdir.join("oops")`
13. Test strings: `"didyoumean-aarch64-apple-darwin.tar.gz"` → `"oops-fix-aarch64-apple-darwin.tar.gz"`
14. Test strings: `"didyoumean-x86_64-unknown-linux-musl.tar.gz"` → `"oops-fix-x86_64-unknown-linux-musl.tar.gz"`
15. Test assertions: `.contains("didyoumean")` → `.contains("oops-fix")`
16. Test temp dirs: `"dym_test_*"` → `"oops_test_*"`

---

### Task 5: src/suggest.rs

**Files:**
- Modify: `src/suggest.rs`

**Step 1: Rename test temp dirs only**

Replacements (test code only):
1. `"dym_test_executables"` → `"oops_test_executables"`
2. `"dym_test_12345"` → `"oops_test_12345"`
3. `"dym_test_empty"` → `"oops_test_empty"`
4. `"dym_test_skipdir"` → `"oops_test_skipdir"`
5. `"dym_test_sorted"` → `"oops_test_sorted"`
6. `"dym_test_dedup"` → `"oops_test_dedup"`

No logic changes.

---

### Task 6: tests/integration.rs

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Apply all replacements**

Replacements:
1. `CARGO_BIN_EXE_didyoumean` → `CARGO_BIN_EXE_oops`
2. `fn dym_bin()` → `fn oops_bin()`
3. All calls `dym_bin()` → `oops_bin()`
4. `"didyoumean "` (version output assertion) → `"oops "`
5. `"# >>> didyoumean initialize >>>"` → `"# >>> oops-fix initialize >>>"`
6. `DYM_AUTO_CORRECT` → `OOPS_AUTO_CORRECT`
7. `DYM_MAX_DISTANCE` → `OOPS_MAX_DISTANCE`
8. `"dym_uninstall_test"` → `"oops_uninstall_test"`
9. `"dym_uninstall_noblock"` → `"oops_uninstall_noblock"`
10. `"dym_uninstall_yflag"` → `"oops_uninstall_yflag"`
11. `tmp.join("didyoumean")` → `tmp.join("oops")`
12. `"didyoumean init zsh"` (in RC file content) → `"oops init zsh"`
13. `"# >>> didyoumean initialize >>>"` in RC file → `"# >>> oops-fix initialize >>>"`
14. `"# <<< didyoumean initialize <<<"` in RC file → `"# <<< oops-fix initialize <<<"`
15. `.contains("didyoumean")` → `.contains("oops")`
16. `"Removed didyoumean block"` → `"Removed oops-fix block"`
17. `".zshrc.dym.bak"` → `".zshrc.oops.bak"`
18. `"[dym]"` → `"[oops]"`

---

### Task 7: install.sh

**Files:**
- Modify: `install.sh`

**Step 1: Apply all replacements**

Replacements:
1. `DYM_TMPDIR` → `OOPS_TMPDIR`
2. `DYM_VERSION` → `OOPS_VERSION`
3. `DYM_INSTALL_DIR` → `OOPS_INSTALL_DIR`
4. `"didyoumean-${target}.tar.gz"` → `"oops-fix-${target}.tar.gz"`
5. `"https://api.github.com/repos/yhzion/didyoumean/releases/latest"` → `"https://api.github.com/repos/yhzion/oops-fix/releases/latest"`
6. `"https://github.com/yhzion/didyoumean/releases/download/v${version}"` → `"https://github.com/yhzion/oops-fix/releases/download/v${version}"`
7. `"Downloading didyoumean"` → `"Downloading oops"`
8. `install -m 755 didyoumean` → `install -m 755 oops`  (note: tar extracts the binary as "oops" now)
9. `"$install_dir/didyoumean"` → `"$install_dir/oops"`
10. `"Installed didyoumean"` → `"Installed oops"`
11. `"didyoumean init $shell_name"` → `"oops init $shell_name"`
12. `"# >>> didyoumean initialize >>>"` → `"# >>> oops-fix initialize >>>"`
13. `"# <<< didyoumean initialize <<<"` → `"# <<< oops-fix initialize <<<"`
14. `"didyoumean is already configured"` → `"oops is already configured"`
15. `"Added didyoumean to"` → `"Added oops to"`
16. `"${rc_file}.dym.bak"` → `"${rc_file}.oops.bak"`

---

### Task 8: .github/workflows/release.yml

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Apply all replacements**

Replacements:
1. `didyoumean-${{ matrix.target }}.tar.gz didyoumean` → `oops-fix-${{ matrix.target }}.tar.gz oops`
2. `name: didyoumean-${{ matrix.target }}` → `name: oops-fix-${{ matrix.target }}`
3. `path: didyoumean-${{ matrix.target }}.tar.gz` → `path: oops-fix-${{ matrix.target }}.tar.gz`
4. `sha256sum didyoumean-*.tar.gz` → `sha256sum oops-fix-*.tar.gz`
5. `didyoumean-*.tar.gz` (in files) → `oops-fix-*.tar.gz`

---

### Task 9: README.md

**Files:**
- Modify: `README.md`

**Step 1: Full rewrite**

Replace entire README with updated naming. Key changes:
- Title: `# oops`
- All `didyoumean` → `oops` (in commands/examples)
- `[dym]` → `[oops]` in console examples
- `DYM_*` → `OOPS_*` in env vars table
- GitHub URLs → `yhzion/oops-fix`
- Install command → updated URL
- Comparison table → `oops` column name
- Badge URLs → `yhzion/oops-fix`

---

### Task 10: CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

**Step 1: Apply all replacements**

Replacements:
1. `didyoumean update` → `oops update`
2. `~/.cache/didyoumean/` → `~/.cache/oops-fix/`
3. `DYM_AUTO_CORRECT` → `OOPS_AUTO_CORRECT`
4. `DYM_MAX_DISTANCE` → `OOPS_MAX_DISTANCE`
5. `DYM_MAX_SUGGESTIONS` → `OOPS_MAX_SUGGESTIONS`
6. GitHub URLs: `yhzion/didyoumean` → `yhzion/oops-fix`
7. Add a new entry in `[Unreleased]` section documenting the rename

---

### Task 11: Verify Everything

**Step 1: Build**

Run: `cargo build`
Expected: success, binary at `target/debug/oops`

**Step 2: Run all tests**

Run: `cargo test`
Expected: all pass

**Step 3: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

**Step 4: Format check**

Run: `cargo fmt --check`
Expected: no issues

**Step 5: Verify binary name**

Run: `ls target/debug/oops && ./target/debug/oops --version`
Expected: `oops 0.2.0`

**Step 6: Commit all changes**

```bash
git add -A
git commit -m "feat: rename project from didyoumean to oops-fix (binary: oops)"
```

---

## Post-Code: Manual Steps (not part of commit)
1. Rename GitHub repo in Settings: `didyoumean` → `oops-fix`
2. GitHub auto-creates redirect from old URL
3. Update any external links/bookmarks
