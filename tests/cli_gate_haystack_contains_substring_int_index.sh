#!/usr/bin/env bash
# CLI emit gate for P3.242 — substring int indices must not mix i64 + 1_usize.
# Exit 0 = GREEN. Exit 1 = RED (expected on tip today).
set -euo pipefail

if [[ -z "${WJ:-}" ]]; then
  if [[ -x "/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj" ]]; then
    WJ="/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj"
  else
    WJ="$HOME/.cargo/bin/wj"
  fi
fi
FIXTURE="$(cd "$(dirname "$0")" && pwd)/fixtures/library_multipass/haystack_contains_int_index.wj"
TMP=$(mktemp -d /tmp/wj-p3242-cli.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/m.wj"
"$WJ" build "$TMP/m.wj" -o "$TMP/build" --library --module-file --no-cargo >/dev/null
RS="$TMP/build/m.rs"

fail() { echo "RED: $*"; echo "---- emit ----"; cat "$RS"; exit 1; }

rg -q 'substring' "$RS" || fail "must emit substring"
if rg -q '\+ 1_usize|\+1_usize' "$RS"; then
  fail "substring end must not mix i64 + 1_usize"
fi

echo "GREEN: haystack_contains_substring_int_indices (CLI)"
