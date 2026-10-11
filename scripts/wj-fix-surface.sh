#!/usr/bin/env bash
# Migrate a directory of .wj sources to the Windjammer-native surface syntax (ADR-0014 Phase 1):
#   println!("a {}", x) -> println("a ${x}"),  vec![..] -> [..],  format! -> "..${x}..",
#   eprintln! -> stderr.println,  dbg! -> debug,  assert!/assert_eq!/panic!/todo! -> plain calls.
#
# Usage:  scripts/wj-fix-surface.sh [--check] <dir-or-file> [more paths...]
#
#   --check   do not write; exit non-zero if any file would change (CI gate)
#
# The `wj` binary is taken from $WJ, else `wj` on PATH, else a local cargo build of this repo.
# Idempotent: running it twice is a no-op. Files whose rewrite would no longer parse are left
# unchanged, and occurrences that cannot be rewritten safely (type-dependent `vec![..]` in call
# arguments, dynamic `{:width$}` specs) are reported as "need manual migration".
set -euo pipefail

CHECK=()
if [[ "${1:-}" == "--check" ]]; then
  CHECK=(--check)
  shift
fi
if [[ $# -lt 1 ]]; then
  echo "usage: $0 [--check] <dir-or-file> [more paths...]" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -n "${WJ:-}" ]]; then
  WJ_BIN="$WJ"
elif command -v wj >/dev/null 2>&1; then
  WJ_BIN="$(command -v wj)"
else
  echo "wj not found on PATH; building it from $REPO_ROOT" >&2
  (cd "$REPO_ROOT" && cargo build --release --bin wj)
  WJ_BIN="${CARGO_TARGET_DIR:-$REPO_ROOT/target}/release/wj"
fi

status=0
for path in "$@"; do
  "$WJ_BIN" fix ${CHECK[@]+"${CHECK[@]}"} "$path" || status=$?
done
exit $status
