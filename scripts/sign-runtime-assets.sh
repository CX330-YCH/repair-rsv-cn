#!/bin/zsh
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
SIGNING_IDENTITY="${SIGNING_IDENTITY:--}"
ENTITLEMENTS="$ROOT/app/src-tauri/Entitlements.plist"
TIMESTAMP_ARG=()
if [[ "$SIGNING_IDENTITY" != "-" ]]; then
  TIMESTAMP_ARG=(--timestamp)
fi

for lib in "$ROOT/lib"/*.dylib(N); do
  codesign --force --options runtime "${TIMESTAMP_ARG[@]}" --sign "$SIGNING_IDENTITY" "$lib"
done

codesign --force --options runtime "${TIMESTAMP_ARG[@]}" --entitlements "$ENTITLEMENTS" --sign "$SIGNING_IDENTITY" "$ROOT/bin/untrunc-rsv"
codesign --verify --deep --strict --verbose=2 "$ROOT/bin/untrunc-rsv"

echo "Signed runtime helper and bundled dylibs"
