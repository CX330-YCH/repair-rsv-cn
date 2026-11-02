#!/bin/zsh
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
APP_DIR="$ROOT/app"
export SIGNING_IDENTITY="${SIGNING_IDENTITY:--}"

"$ROOT/scripts/build-from-source.sh"
"$ROOT/scripts/bundle-ffmpeg-dylibs.sh"
"$ROOT/scripts/sign-runtime-assets.sh"

cd "$APP_DIR"
CONFIG="$(python3 -c 'import json, os; print(json.dumps({"bundle": {"macOS": {"signingIdentity": os.environ["SIGNING_IDENTITY"], "providerShortName": None}}}))')"
"$ROOT/scripts/with-appledouble-cleaner.sh" tauri build --config "$CONFIG" --bundles dmg
