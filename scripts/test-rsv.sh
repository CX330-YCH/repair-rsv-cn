#!/bin/zsh
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
TEST_DIR="$(mktemp -d "${TMPDIR:-/tmp}/repair-rsv-tests.XXXXXX")"
trap 'rm -rf -- "$TEST_DIR"' EXIT
"${CXX:-c++}" -std=c++17 -Wall -Wextra -Werror \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  -I "$ROOT/source/untrunc-rsv/src" "$ROOT/tests/rsv_parser_test.cpp" \
  -o "$TEST_DIR/rsv-parser-test"
"$TEST_DIR/rsv-parser-test"
if [[ $# -eq 2 ]]; then
  python3 "$ROOT/tests/rsv_recovery_test.py" --tool "$ROOT/bin/untrunc-rsv" \
    --donor "$1" --rsv "$2"
elif [[ $# -ne 0 ]]; then
  echo "Usage: zsh scripts/test-rsv.sh [donor.mp4 broken.RSV]" >&2
  exit 64
fi
