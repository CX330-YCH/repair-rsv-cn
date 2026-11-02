#!/bin/sh
set -eu

ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
exec python3 "$ROOT/scripts/tauri-bundle.py"
