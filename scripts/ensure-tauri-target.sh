#!/bin/zsh
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
PROJECT_TARGET="$ROOT/app/src-tauri/target"
CACHE_TARGET="${REPAIR_RSV_TARGET_CACHE:-$HOME/Library/Caches/repair-rsv/tauri-target}"

if [[ -d "$PROJECT_TARGET" ]]; then
  exit 0
fi
if [[ -e "$PROJECT_TARGET" || -L "$PROJECT_TARGET" ]]; then
  echo "Build target exists but is not a usable directory: $PROJECT_TARGET" >&2
  exit 1
fi
mkdir -p "$CACHE_TARGET"
ln -s "$CACHE_TARGET" "$PROJECT_TARGET"
