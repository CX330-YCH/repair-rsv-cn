#!/bin/zsh
set -euo pipefail

# Keep the historical entry point on the current, versioned build pipeline.
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT/app"
exec npm run bundle
