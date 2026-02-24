#!/bin/bash
set -euo pipefail

OOPS_TMPDIR=""
cleanup() { [ -n "$OOPS_TMPDIR" ] && rm -rf "$OOPS_TMPDIR"; }
trap cleanup EXIT

main() {
    local version="${OOPS_VERSION:-latest}"
    local install_dir="${OOPS_INSTALL_DIR:-$HOME/.local/bin}"

    # Detect OS
    local os
    case "$(uname -s)" in
        Linux*)  os="unknown-linux-musl" ;;
        Darwin*) os="apple-darwin" ;;
        *) echo "Error: unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac

    # Detect arch
    local arch
    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "Error: unsupported architecture: $(uname -m)" >&2; exit 1 ;;
    esac

    # Rosetta 2 detection on macOS
    if [ "$os" = "apple-darwin" ] && [ "$arch" = "x86_64" ]; then
        if sysctl -n sysctl.proc_translated 2>/dev/null | grep -q 1; then
            arch="aarch64"
        fi
    fi

    local target="${arch}-${os}"
    local filename="oops-fix-${target}.tar.gz"

    # Resolve version
    if [ "$version" = "latest" ]; then
        if command -v curl >/dev/null 2>&1; then
            version=$(curl -sSf "https://api.github.com/repos/yhzion/oops-fix/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
        elif command -v wget >/dev/null 2>&1; then
            version=$(wget -qO- "https://api.github.com/repos/yhzion/oops-fix/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
        else
            echo "Error: curl or wget is required" >&2
            exit 1
        fi
    fi

    local base_url="https://github.com/yhzion/oops-fix/releases/download/v${version}"
    local url="${base_url}/${filename}"
    local checksum_url="${base_url}/SHA256SUMS"

    # Create install dir
    mkdir -p "$install_dir"

    # Download to temp dir
    OOPS_TMPDIR=$(mktemp -d)
    local tmpdir="$OOPS_TMPDIR"

    echo "Downloading oops v${version} for ${target}..."
    if command -v curl >/dev/null 2>&1; then
        curl -sSfL "$url" -o "$tmpdir/$filename"
        curl -sSfL "$checksum_url" -o "$tmpdir/SHA256SUMS"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$tmpdir/$filename" "$url"
        wget -qO "$tmpdir/SHA256SUMS" "$checksum_url"
    fi

    # Verify checksum
    echo "Verifying checksum..."
    cd "$tmpdir"
    if command -v sha256sum >/dev/null 2>&1; then
        grep "$filename" SHA256SUMS | sha256sum -c - >/dev/null 2>&1
    elif command -v shasum >/dev/null 2>&1; then
        grep "$filename" SHA256SUMS | shasum -a 256 -c - >/dev/null 2>&1
    else
        echo "Warning: cannot verify checksum (sha256sum/shasum not found)" >&2
    fi

    # Extract and install
    tar xzf "$filename"
    install -m 755 oops "$install_dir/oops"
    echo "Installed oops to $install_dir/oops"

    # Check if install_dir is in PATH
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$install_dir"; then
        echo "Warning: $install_dir is not in your PATH" >&2
        echo "  Add this to your shell config: export PATH=\"$install_dir:\$PATH\"" >&2
    fi

    # Detect shell and RC file
    local shell_name rc_file
    shell_name=$(basename "$SHELL")
    case "$shell_name" in
        zsh)  rc_file="$HOME/.zshrc" ;;
        bash) rc_file="$HOME/.bashrc" ;;
        *)
            echo "Unsupported shell: $shell_name" >&2
            echo "Add manually: eval \"\$(oops init $shell_name)\"" >&2
            return 0
            ;;
    esac

    # Check for existing installation (idempotent)
    if grep -q "# >>> oops-fix initialize >>>" "$rc_file" 2>/dev/null; then
        echo "oops is already configured in $rc_file"
        return 0
    fi

    # Warn about existing command_not_found handler
    if grep -q "command_not_found_handle" "$rc_file" 2>/dev/null; then
        echo "Warning: existing command_not_found handler found in $rc_file" >&2
        echo "  Add manually: eval \"\$(oops init $shell_name)\"" >&2
        return 0
    fi

    # Backup RC file
    if [ -f "$rc_file" ]; then
        cp "$rc_file" "${rc_file}.oops.bak"
    fi

    # Add eval line
    cat >> "$rc_file" <<SHELLEOF

# >>> oops-fix initialize >>>
eval "\$(oops init $shell_name)"
# <<< oops-fix initialize <<<
SHELLEOF

    echo "Added oops to $rc_file (backup: ${rc_file}.oops.bak)"
    echo "Run 'source $rc_file' or restart your shell to activate."
}

main "$@"
