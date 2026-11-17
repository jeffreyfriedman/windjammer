#!/bin/sh
# Build-slot gate with disk hygiene: `slot_gate.sh <command...>` waits for one of N slots (default 3), checks the disk
# floor, points cargo at the one shared target, runs the command, frees the slot. See docs/dev/disk-hygiene.md.
#
# Shell fallback (recorded, per the tooling policy): the Windjammer version of the slot gate hangs under
# `wj run --interpret` (no output, timeout), repro: wj run --interpret slot.wj -- echo hello
#
# Environment (nothing about the clone path or OS is assumed):
#   WJ_BUILD_SLOTS      slots, default 3
#   WJ_SLOT_ROOT        slot dir, default $HOME/.wj-build-slots
#   WJ_SHARED_TARGET    the shared cargo target; default <cache>/windjammer/cargo-target where <cache> is
#                       ~/Library/Caches on macOS and ${XDG_CACHE_HOME:-~/.cache} elsewhere
#   WJ_WORKTREE_ROOT    when set, a CARGO_TARGET_DIR inside it is refused (private target)
#   WJ_DISK_FLOOR_GB    free-space floor before a build starts, default 28
#   WJ_DISK_WAIT_SECS   how long to wait for space to appear before refusing, default 300 (0 = refuse at once)
#   WJ_JANITOR_CMD      command that runs the janitor gate, default: wj run ... scripts/wj/disk_janitor.wj -- gate
#                       (exit 0 ok, 1 below the floor; any other status = janitor unavailable, df is used instead)
# Exit codes: 4 CARGO_TARGET_DIR points elsewhere inside a worktree, 5 disk still below the floor, else the command's.
MAX=${WJ_BUILD_SLOTS:-3}
ROOT=${WJ_SLOT_ROOT:-$HOME/.wj-build-slots}
FLOOR=${WJ_DISK_FLOOR_GB:-28}
WAIT=${WJ_DISK_WAIT_SECS:-300}
HERE=$(cd "$(dirname "$0")" && pwd)

if [ -n "${WJ_SHARED_TARGET:-}" ]; then
  SHARED=$WJ_SHARED_TARGET
elif [ "$(uname -s)" = Darwin ]; then
  SHARED=$HOME/Library/Caches/windjammer/cargo-target
else
  SHARED=${XDG_CACHE_HOME:-$HOME/.cache}/windjammer/cargo-target
fi

# One shared target: unset or already the shared one is fine; elsewhere inside a worktree is a policy violation.
if [ -n "${CARGO_TARGET_DIR:-}" ] && [ "$CARGO_TARGET_DIR" != "$SHARED" ]; then
  inside=0
  case "$CARGO_TARGET_DIR" in
    "$PWD"/*|"$PWD") inside=1 ;;
  esac
  if [ -n "${WJ_WORKTREE_ROOT:-}" ]; then
    case "$CARGO_TARGET_DIR" in
      "$WJ_WORKTREE_ROOT"/*) inside=1 ;;
    esac
  fi
  top=$(git rev-parse --show-toplevel 2>/dev/null)
  if [ -n "$top" ]; then
    case "$CARGO_TARGET_DIR" in
      "$top"/*|"$top") inside=1 ;;
    esac
  fi
  if [ "$inside" = 1 ]; then
    echo "slot_gate: refusing: CARGO_TARGET_DIR=$CARGO_TARGET_DIR is a private target inside a worktree." >&2
    echo "slot_gate: use the one shared target ($SHARED): unset CARGO_TARGET_DIR or set it to that path." >&2
    exit 4
  fi
fi
CARGO_TARGET_DIR=$SHARED
export CARGO_TARGET_DIR
mkdir -p "$SHARED" 2>/dev/null

free_gb() { # POSIX df: available 1K blocks of the filesystem holding the shared target
  df -Pk "$SHARED" 2>/dev/null | awk 'NR==2{print int($4/1048576)}'
}

disk_ok() {
  if [ -n "${WJ_JANITOR_CMD:-}" ]; then
    # shellcheck disable=SC2086
    $WJ_JANITOR_CMD gate --floor-gb "$FLOOR" >/dev/null 2>&1
    rc=$?
  elif command -v wj >/dev/null 2>&1 && [ -f "$HERE/wj/disk_janitor.wj" ]; then
    (cd "$HERE/wj" && wj run --allow process --allow env.read disk_janitor.wj -- gate --floor-gb "$FLOOR" >/dev/null 2>&1)
    rc=$?
  else
    rc=127
  fi
  case $rc in
    0) return 0 ;;
    1) return 1 ;;
  esac
  # janitor unavailable: the same question answered with df
  g=$(free_gb)
  [ -z "$g" ] && return 0
  [ "$g" -ge "$FLOOR" ]
}

waited=0
while ! disk_ok; do
  if [ "$waited" -ge "$WAIT" ]; then
    echo "slot_gate: refusing to build: about $(free_gb) GB free, floor is $FLOOR GB." >&2
    echo "slot_gate: free space (janitor: wj run scripts/wj/disk_janitor.wj -- clean --apply) and try again." >&2
    exit 5
  fi
  echo "slot_gate: below the $FLOOR GB floor ($(free_gb) GB free), waiting for space..." >&2
  sleep 15
  waited=$((waited + 15))
done

mkdir -p "$ROOT"
HELD=""
while [ -z "$HELD" ]; do
  i=0
  while [ $i -lt "$MAX" ]; do
    d="$ROOT/$i"
    if mkdir "$d" 2>/dev/null; then echo $$ > "$d/pid"; HELD="$d"; break; fi
    # reclaim a slot whose owner died
    if [ -f "$d/pid" ] && ! kill -0 "$(cat "$d/pid")" 2>/dev/null; then rm -rf "$d"; fi
    i=$((i+1))
  done
  [ -z "$HELD" ] && sleep 15
done
trap 'rm -rf "$HELD"' EXIT INT TERM
"$@"
