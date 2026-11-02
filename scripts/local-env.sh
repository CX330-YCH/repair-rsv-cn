#!/bin/sh

# Source this file before local builds. All writable tool caches stay in the
# repository and no global Cargo, Rustup, npm, or FFmpeg paths are changed.

REPAIR_RSV_ROOT="${REPAIR_RSV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
REPAIR_RSV_TOOLING="$REPAIR_RSV_ROOT/.tooling"
REPAIR_RSV_SYSTEM="$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"

export REPAIR_RSV_ROOT
export REPAIR_RSV_TOOLING
export CARGO_HOME="${CARGO_HOME:-$REPAIR_RSV_TOOLING/cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$REPAIR_RSV_TOOLING/rustup}"
export NPM_CONFIG_CACHE="${NPM_CONFIG_CACHE:-$REPAIR_RSV_TOOLING/npm-cache}"
export REPAIR_RSV_DEPS_PREFIX="${REPAIR_RSV_DEPS_PREFIX:-$REPAIR_RSV_TOOLING/deps/$REPAIR_RSV_SYSTEM}"
export REPAIR_RSV_FFMPEG_VERSION="${REPAIR_RSV_FFMPEG_VERSION:-9.0.2}"
export PATH="$REPAIR_RSV_TOOLING/bin:$CARGO_HOME/bin:$PATH"

mkdir -p "$REPAIR_RSV_TOOLING/bin" "$CARGO_HOME" "$RUSTUP_HOME" "$NPM_CONFIG_CACHE" "$REPAIR_RSV_DEPS_PREFIX"

echo "Repair RSV local environment:"
echo "  tooling: $REPAIR_RSV_TOOLING"
echo "  deps:    $REPAIR_RSV_DEPS_PREFIX"
echo "  cargo:   $CARGO_HOME"
echo "  rustup:  $RUSTUP_HOME"
