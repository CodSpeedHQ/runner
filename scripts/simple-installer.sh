#!/usr/bin/env bash
# Simple installer for codspeed-runner
# This script clones the repository (or uses the current directory), builds the
# `codspeed` binary with cargo in release mode, and installs it to the target
# directory (default: /usr/local/bin). It intentionally avoids cargo-dist and
# the GitHub release flow so you can build and install locally or from CI.

set -euo pipefail

REPO_URL="https://github.com/jzombie/codspeed-runner.git"
REF="main"
INSTALL_DIR="/usr/local/bin"
TMP_DIR=""
NO_RUSTUP="false"
QUIET="false"

usage() {
  cat <<EOF
Usage: $0 [--repo <git-url>] [--ref <branch-or-tag>] [--install-dir <path>] [--no-rustup] [--quiet]

Options:
  --repo        Git repository URL (default: ${REPO_URL})
  --ref         Git ref to checkout (branch, tag, or commit). Default: ${REF}
  --install-dir Where to install the built binary. Default: ${INSTALL_DIR}
  --no-rustup   Do not attempt to install rustup if cargo is missing
  --quiet       Minimize output
  -h, --help    Show this help message

Example:
  curl -fsSL https://example.com/codspeed-runner-installer.sh | bash -s -- --ref feature/my-branch

This script will clone the repository to a temporary directory, build the
`codspeed` binary with `cargo build --release`, and copy it to
${INSTALL_DIR}. Sudo may be used to write to the install directory.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo)
      REPO_URL="$2"; shift 2;;
    --ref)
      REF="$2"; shift 2;;
    --install-dir)
      INSTALL_DIR="$2"; shift 2;;
    --no-rustup)
      NO_RUSTUP="true"; shift 1;;
    --quiet)
      QUIET="true"; shift 1;;
    -h|--help)
      usage; exit 0;;
    --)
      shift; break;;
    *)
      echo "Unknown argument: $1" >&2; usage; exit 1;;
  esac
done

log() {
  if [ "$QUIET" != "true" ]; then
    echo "$@"
  fi
}

fail() {
  echo "Error: $@" >&2
  exit 1
}

cleanup() {
  if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT

check_command() {
  command -v "$1" >/dev/null 2>&1
}

ensure_rust() {
  if check_command cargo; then
    log "Found cargo"
    return 0
  fi

  if [ "$NO_RUSTUP" = "true" ]; then
    fail "cargo is not installed and --no-rustup was passed. Install Rust toolchain first.";
  fi

  log "Rust toolchain not found. Installing rustup (non-interactive)..."
  # Install rustup non-interactively
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || fail "failed to install rustup"
  export PATH="$HOME/.cargo/bin:$PATH"
  check_command cargo || fail "cargo still not available after rustup install"
}

main() {
  ensure_rust

  # Create temp dir
  TMP_DIR=$(mktemp -d -t codspeed-installer-XXXX)
  log "Using temporary directory: $TMP_DIR"

  # Clone the requested ref
  log "Cloning ${REPO_URL} (ref: ${REF})..."
  git clone --depth 1 --branch "$REF" "$REPO_URL" "$TMP_DIR" || {
    # Try cloning default branch and then checking out ref (for commit-ish refs)
    log "Shallow clone failed for ref $REF, attempting full clone and checkout"
    rm -rf "$TMP_DIR"
    TMP_DIR=$(mktemp -d -t codspeed-installer-XXXX)
    git clone "$REPO_URL" "$TMP_DIR" || fail "failed to clone repo"
    (cd "$TMP_DIR" && git fetch --all --tags && git checkout "$REF") || fail "failed to checkout ref $REF"
  }

  # Build
  log "Building codspeed (release)..."
  (cd "$TMP_DIR" && cargo build --release) || fail "cargo build failed"

  # Locate built binary
  BIN_PATH="$TMP_DIR/target/release/codspeed"
  if [ ! -x "$BIN_PATH" ]; then
    fail "Built binary not found at $BIN_PATH"
  fi

  # Ensure install dir exists
  if [ ! -d "$INSTALL_DIR" ]; then
    log "Creating install directory $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR" || fail "failed to create install dir"
  fi

  # Copy binary (use sudo if required)
  DEST="$INSTALL_DIR/codspeed"
  if [ -w "$INSTALL_DIR" ]; then
    cp "$BIN_PATH" "$DEST" || fail "failed to copy binary to $DEST"
  else
    log "Installing to $DEST with sudo"
    sudo cp "$BIN_PATH" "$DEST" || fail "sudo copy failed"
  fi

  log "Installed codspeed to $DEST"
  log "Run 'codspeed --help' to verify"
}

main "$@"
