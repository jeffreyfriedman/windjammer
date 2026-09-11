#!/usr/bin/env bash
# CLI emit gate for P3.237 — tip `wj` when cargo test is blocked on tip bin WIP.
# Exit 0 = GREEN (no row.clone around get_string). Exit 1 = RED (expected today).
set -euo pipefail

# Prefer in-tree tip release over older `~/.cargo/bin/wj`.
if [[ -z "${WJ:-}" ]]; then
  if [[ -x "/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj" ]]; then
    WJ="/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj"
  else
    WJ="$HOME/.cargo/bin/wj"
  fi
fi
FIXTURE="$(cd "$(dirname "$0")" && pwd)/fixtures/library_multipass/db_row_get_string_match_arms.wj"
TMP=$(mktemp -d /tmp/wj-p3237-cli.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/m.wj"
"$WJ" build "$TMP/m.wj" -o "$TMP/build" --library --module-file --no-cargo >/dev/null
RS="$TMP/build/m.rs"

fail() { echo "RED: $*"; echo "---- emit ----"; cat "$RS"; exit 1; }

rg -q 'get_string' "$RS" || fail "must emit get_string"
if rg -q 'row\.clone\(\)\.get_string' "$RS"; then
  fail "get_string must not force row.clone() before the call"
fi
if rg -q '\(row\.clone\(\), (value|id|name)\)' "$RS"; then
  fail "(Row, string) return must move row, not row.clone()"
fi

echo "GREEN: db_row_get_string_no_row_clone (CLI)"
