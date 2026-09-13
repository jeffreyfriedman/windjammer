#!/usr/bin/env bash
# Atomic install of the `wj` compiler binary.
# Never truncate an existing inode in place (wedges live macOS processes: state UE).
#
# Usage:
#   scripts/atomic_install_wj.sh <src> <dest> [--allow-busy]
#
# Prefer isolated tips for concurrent agents:
#   export WJ_COMPILER=/path/to/windjammer-game/.cargo-target-wj/release/wj
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <src> <dest> [--allow-busy]" >&2
  exit 2
fi

SRC=$1
DEST=$2
ALLOW_BUSY=0
if [[ "${3:-}" == "--allow-busy" ]]; then
  ALLOW_BUSY=1
fi

if [[ ! -f "$SRC" ]]; then
  echo "error: source is not a file: $SRC" >&2
  exit 1
fi

mkdir -p "$(dirname "$DEST")"

USERS=()
if command -v lsof >/dev/null 2>&1 && [[ -e "$DEST" ]]; then
  while IFS= read -r pid; do
    [[ -n "$pid" ]] && USERS+=("$pid")
  done < <(lsof -a -d txt -F p "$DEST" 2>/dev/null | sed -n 's/^p//p' || true)
fi

if ((${#USERS[@]} > 0)) && [[ "$ALLOW_BUSY" -eq 0 ]]; then
  echo "error: refusing to replace $DEST — still executing in PIDs: ${USERS[*]}" >&2
  echo "Prefer: export WJ_COMPILER=<isolated tip wj>" >&2
  echo "Or rebuild into windjammer-game/.cargo-target-wj and leave dogfood alone." >&2
  echo "Atomic rename is safe for old PIDs; re-run with --allow-busy to publish dogfood anyway." >&2
  exit 1
fi

if ((${#USERS[@]} > 0)); then
  echo "warning: $DEST in use by PIDs ${USERS[*]} — installing via rename; those processes keep the old binary" >&2
fi

TMP="${DEST}.$$.new"
cp -f "$SRC" "$TMP"
chmod +x "$TMP"
mv -f "$TMP" "$DEST"
echo "installed $DEST (atomic rename)"
