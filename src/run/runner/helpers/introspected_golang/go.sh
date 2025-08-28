#!/usr/bin/env bash

set -euo pipefail

debug_log() {
    if [ "${CODSPEED_LOG:-}" = "debug" ]; then
        echo "[DEBUG go.sh] $*" >&2
    fi
}

debug_log "Called with arguments: $*"
debug_log "Number of arguments: $#"


# Currently only walltime is supported
if [ "${CODSPEED_RUNNER_MODE:-}" != "walltime" ]; then
    echo "CRITICAL: Go benchmarks can only be run with the walltime instrument"
    exit 1
fi

# Find the real go binary, so that we don't end up in infinite recursion
REAL_GO=$(which -a go | grep -v "$(realpath "$0")" | head -1)
if [ -z "$REAL_GO" ]; then
    echo "ERROR: Could not find real go binary" >&2
    exit 1
fi

# Check if we have any arguments
if [ $# -eq 0 ]; then
    debug_log "No arguments provided, using standard go binary"
    "$REAL_GO"
    exit $?
fi

# Check if first argument is "test"
if [ "$1" = "test" ]; then
    debug_log "Detected 'test' command, routing to go-runner"

    # Find go-runner or install if not found
    GO_RUNNER=$(which go-runner 2>/dev/null || true)
    if [ -z "$GO_RUNNER" ]; then
        curl -fsSL http://github.com/CodSpeedHQ/runner/releases/latest/download/go-runner-installer.sh | bash -s -- --quiet
        GO_RUNNER=$(which go-runner 2>/dev/null || true)
    fi

    debug_log "Using go-runner at: $GO_RUNNER"
    debug_log "Full command: RUST_LOG=info $GO_RUNNER $*"

    "$GO_RUNNER" "$@"
else
    debug_log "Detected non-test command ('$1'), routing to standard go binary"
    debug_log "Full command: $REAL_GO $*"
    "$REAL_GO" "$@"
fi
