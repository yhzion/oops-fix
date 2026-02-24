# didyoumean

**Typo → Fix → Run. Instantly.**

[![CI](https://github.com/yhzion/didyoumean/actions/workflows/ci.yml/badge.svg)](https://github.com/yhzion/didyoumean/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Single binary. No daemon. No config. ~300KB.

---

You mistype commands. Everyone does. **didyoumean** catches it before the shell gives up, finds the right command, and runs it — with your arguments intact.

```console
$ gti stash pop
[dym] 'gti stash pop' → 'git stash pop'          # runs automatically

$ dcoker compose up -d
[dym] 'dcoker compose up -d' → 'docker compose up -d'

$ kubeclt get pods -n production
[dym] 'kubeclt get pods -n production' → 'kubectl get pods -n production'

$ brwe install ripgrep
[dym] 'brwe install ripgrep' → 'brew install ripgrep'
```

That's it. No confirmation prompt. No extra keypress. It just works.

## Install

```bash
curl -sSfL https://raw.githubusercontent.com/yhzion/didyoumean/main/install.sh | bash
```

Or build from source:

```bash
cargo install --git https://github.com/yhzion/didyoumean
```

Then add to your shell config:

```bash
# ~/.zshrc
eval "$(didyoumean init zsh)"

# ~/.bashrc
eval "$(didyoumean init bash)"
```

## How it works

```
You type: kubeclt get pods
                │
Shell can't find "kubeclt"
                │
        ┌───────▼────────┐
        │  didyoumean     │  Receives all known commands
        │                 │  (builtins + PATH executables)
        │  Computes       │
        │  Damerau-       │  "kubectl" = distance 1
        │  Levenshtein    │  (transposition of l,t)
        │  distance       │
        └───────┬────────┘
                │
    Confident? (distance 1, unique, length ≥ 3)
                │
          ┌─────┴─────┐
         YES          NO
          │            │
     Exit 0        Exit 1
     "kubectl"     Show suggestions
          │
  Shell runs: kubectl get pods
```

**No daemon.** Runs only when you mistype — not a background process.

**No network.** Pure local string comparison. Works offline, always.

**No config.** Works out of the box. Env vars available for tuning.

## More examples

### Auto-correction (high confidence)

When the match is unambiguous (distance 1, unique, length ≥ 3), didyoumean auto-corrects and runs:

```console
$ gti stash pop           →  git stash pop
$ dcoker ps               →  docker ps
$ pyhton main.py          →  python main.py
$ kubeclt get pods        →  kubectl get pods
$ claer                   →  clear
$ brwe install ffmpeg     →  brew install ffmpeg
$ ndoe index.js           →  node index.js
$ carg build --release    →  cargo build --release
```

### Suggestions (multiple matches or low confidence)

When there are multiple candidates or the command is too short:

```console
$ gt
[dym] Did you mean one of these? (gt)
  git
  gd

$ nde
[dym] Did you mean 'node'?
```

### No match

```console
$ xyzabc123
[dym] Command 'xyzabc123' not found, no similar commands
```

### Opt-in auto-correct for lower confidence

```bash
export DYM_AUTO_CORRECT=on
```

This also auto-executes matches that don't meet the high-confidence threshold (e.g., short commands).

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DYM_AUTO_CORRECT` | Auto-execute lower-confidence matches (`on`/`1`/`true`) | `off` |
| `DYM_MAX_DISTANCE` | Maximum Damerau-Levenshtein distance | `2` |
| `DYM_MAX_SUGGESTIONS` | Maximum number of suggestions | `5` |
| `NO_COLOR` | Disable colored output (any value) | unset |

## Why didyoumean?

| | didyoumean | thefuck | pay-respects |
|---|---|---|---|
| **When** | Before execution | After execution | Before execution |
| **Action** | Auto-runs if confident | Type `fuck` to correct | Press `F` to correct |
| **Runtime** | Native binary (~300KB) | Python 3 required | Native binary (<1MB) |
| **Config** | Zero | Rule files | TOML rules |
| **Approach** | Edit distance only | Rule matching + AI | Rule matching + AI |
| **Memory** | None (runs on demand) | Python process | None (runs on demand) |

**thefuck** runs your wrong command first, then offers a fix after you type `fuck`. You wait for the wrong command to fail, then wait for Python to start, then confirm.

**didyoumean** intercepts *before* execution. If it's confident, your corrected command runs immediately. One mistype, zero extra keystrokes.

## Supported platforms

| OS | Shell | Architecture |
|---|---|---|
| Linux | zsh, bash | x86_64, aarch64 |
| macOS | zsh, bash | x86_64 (Intel), aarch64 (Apple Silicon) |

## Uninstall

```bash
didyoumean uninstall
```

Removes the binary and shell config block. Backs up your RC file first.

## License

[MIT](LICENSE)
