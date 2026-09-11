#!/usr/bin/env bash
# CLI emit gate for P3.238 — owned Row chain call sites must move, not clone.
# Exit 0 = GREEN. Exit 1 = RED (expected while tip emits col_string(row.clone(), …)).
set -euo pipefail

if [[ -z "${WJ:-}" ]]; then
  if [[ -x "/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj" ]]; then
    WJ="/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj"
  else
    WJ="$HOME/.cargo/bin/wj"
  fi
fi
FIXTURE="$(cd "$(dirname "$0")" && pwd)/fixtures/library_multipass/db_row_get_string_match_arms.wj"
TMP=$(mktemp -d /tmp/wj-p3238-cli.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/m.wj"
"$WJ" build "$TMP/m.wj" -o "$TMP/build" --library --module-file --no-cargo >/dev/null
RS="$TMP/build/m.rs"

fail() { echo "RED: $*"; echo "---- emit ----"; cat "$RS"; exit 1; }

rg -q 'two_columns|fn two_columns' "$RS" || fail "must emit two_columns"
if rg -q 'col_string\(row\.clone\(' "$RS"; then
  fail "owned Row chain must move into col_string(row, …), not row.clone()"
fi

echo "GREEN: db_row_col_chain_call_site_move (CLI)"
