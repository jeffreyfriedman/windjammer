#!/usr/bin/env bash
# Reap test and build leftovers in the temp dir that nothing can still be using, and report free disk space.
#
#   scripts/reap-tmp.sh [--dry-run]   remove stale leftovers in ${TMPDIR:-/tmp}; prints each path and a summary
#   scripts/reap-tmp.sh --disk        print free GB of the temp dir's filesystem; exit 0 ok, 10 ALARM (< 30 GB),
#                                     20 HARD STOP for new heavy jobs (< 20 GB); WJ_DISK_ALARM_GB / WJ_DISK_STOP_GB override
#   scripts/reap-tmp.sh --selftest    build fake leftovers in a private temp base and check what is reaped and kept
#
# What is stale (only entries owned by the current user, only directories):
#  1. pid-named  wj-<tag>-<pid>-<n> (includes wj-*-gate-<pid>-<n>, wj-cache-active-<pid>-<n>), windjammer-test-<pid>-<n>:
#     the owner process is gone AND nothing in the tree was touched for 10 minutes (a recycled pid is then not mistaken
#     for the owner, and a live one is never touched);
#  2. pid-less   .tmp* (tempfile crate), wj-drift-live.*: nothing in the tree touched for 6 hours.
# Killed test runs (timeouts, kill -9, restarts) never run their cleanup; tests/common/scratch.rs reaps rule 1 itself
# on the first Scratch::new of a process, this script covers everything else and runs before heavy jobs.
#
# Why bash and not interpreted Windjammer (recorded per the tooling policy, see docs/design/python-replacement-audit.md):
# std::fs::Metadata has no modification time (only size, is_file, is_dir, is_readonly), std::process has no liveness
# check for another pid (only pid(), run, exit), and there is no free-space query for a filesystem. A port would fork
# find, kill and df for every entry, and the reaper must also run when no wj binary is built yet (it frees the disk the
# build needs). Revisit when std::fs::Metadata::modified and a std::process liveness function exist.
#
# Portability (Linux and macOS): bash 3.2 features only, ${TMPDIR:-/tmp}, find -mmin (no -newermt), du -sk, df -Pk,
# ps -p / kill -0 (no /proc), touch -t, no stat, no GNU-only sed/date flags.
set -u

ME="$0"
TMP_BASE=${TMPDIR:-/tmp}
TMP_BASE=${TMP_BASE%/}
[ -n "$TMP_BASE" ] || TMP_BASE=/

# Anything in the tree (including the directory itself) modified within the last $2 minutes?
touched_within() {
  [ -n "$(find "$1" -mmin -"$2" -print 2>/dev/null | head -n 1)" ]
}

# Is a process with this pid running? ps -p and kill -0 are both portable; kill -0 alone reports "not permitted" for
# another user's live process, so either one succeeding counts as alive.
pid_alive() {
  ps -p "$1" >/dev/null 2>&1 || kill -0 "$1" 2>/dev/null
}

reap_main() {
  local dry=$1 n=0 kb=0 d b pid sz
  reap_one() {
    sz=$(du -sk "$1" 2>/dev/null | cut -f1)
    sz=${sz:-0}
    echo "reap $((sz / 1024))M $1"
    n=$((n + 1)); kb=$((kb + sz))
    [ "$dry" = 1 ] || rm -rf "$1"
  }
  for d in "$TMP_BASE"/wj-* "$TMP_BASE"/windjammer-test-*; do
    [ -d "$d" ] && [ ! -L "$d" ] && [ -O "$d" ] || continue
    b=${d##*/}
    [[ $b =~ -([0-9]+)-[0-9]+$ ]] || continue
    pid=${BASH_REMATCH[1]}
    [ "$pid" = "$$" ] && continue
    pid_alive "$pid" && continue
    touched_within "$d" 10 && continue
    reap_one "$d"
  done
  for d in "$TMP_BASE"/.tmp* "$TMP_BASE"/wj-drift-live.*; do
    [ -d "$d" ] && [ ! -L "$d" ] && [ -O "$d" ] || continue
    touched_within "$d" 360 && continue
    reap_one "$d"
  done
  echo "reaped $n dirs, $((kb / 1024)) MB in $TMP_BASE$([ "$dry" = 1 ] && echo ' (dry run, nothing removed)')"
}

disk_main() {
  local alarm=${WJ_DISK_ALARM_GB:-30} stop=${WJ_DISK_STOP_GB:-20} free
  free=$(df -Pk "$TMP_BASE" 2>/dev/null | awk 'NR == 2 { print int($4 / 1048576) }')
  if [ -z "$free" ]; then echo "disk: cannot read free space of $TMP_BASE" >&2; return 2; fi
  if [ "$free" -lt "$stop" ]; then
    echo "disk: ${free} GB free on $TMP_BASE: HARD STOP (< ${stop} GB), start no new heavy jobs"; return 20
  elif [ "$free" -lt "$alarm" ]; then
    echo "disk: ${free} GB free on $TMP_BASE: ALARM (< ${alarm} GB)"; return 10
  fi
  echo "disk: ${free} GB free on $TMP_BASE: ok"
}

selftest_main() {
  local base dead fails=0 checks=0 out code old=202001010000
  base=$(mktemp -d "$TMP_BASE/reap-selftest.XXXXXX") || { echo "selftest: mktemp failed" >&2; return 1; }
  SELFTEST_BASE=$base
  trap 'rm -rf "$SELFTEST_BASE"' EXIT
  # A pid that is certainly not running: a child we started and already waited for.
  sh -c 'exit 0' & dead=$!; wait "$dead"
  mk() { mkdir -p "$base/$1/sub" && echo x >"$base/$1/sub/f"; }
  age() { find "$base/$1" -exec touch -t "$old" {} +; }
  mk "wj-cache-active-$dead-0"; age "wj-cache-active-$dead-0"      # dead owner, idle: reaped
  mk "windjammer-test-$dead-3"; age "windjammer-test-$dead-3"      # dead owner, idle: reaped
  mk "wj-live-$$-0";            age "wj-live-$$-0"                 # live owner (this script): kept
  mk "wj-recent-$dead-1"                                           # dead owner, touched now: kept
  mk ".tmpOLD123";              age ".tmpOLD123"                   # pid-less, idle 6 h: reaped
  mk ".tmpNEW456"                                                  # pid-less, recent: kept
  mk "other-tool-$dead-0";      age "other-tool-$dead-0"           # foreign name: kept
  mk "wj-nopid";                age "wj-nopid"                     # wj- without a pid: kept
  check() { checks=$((checks + 1)); if ! eval "$2"; then echo "selftest FAIL: $1" >&2; fails=$((fails + 1)); fi; }

  out=$(TMPDIR="$base" bash "$ME" --dry-run); code=$?
  check "dry run exits 0" '[ $code = 0 ]'
  check "dry run names 3 dirs" 'echo "$out" | grep -q "reaped 3 dirs"'
  check "dry run removes nothing" '[ -d "$base/wj-cache-active-$dead-0" ] && [ -d "$base/.tmpOLD123" ]'

  out=$(TMPDIR="$base" bash "$ME"); code=$?
  check "reap exits 0" '[ $code = 0 ]'
  check "reap prints each path" '[ "$(echo "$out" | grep -c "^reap ")" = 3 ]'
  check "dead idle wj- dir reaped" '[ ! -e "$base/wj-cache-active-$dead-0" ]'
  check "dead idle windjammer-test dir reaped" '[ ! -e "$base/windjammer-test-$dead-3" ]'
  check "old .tmp dir reaped" '[ ! -e "$base/.tmpOLD123" ]'
  check "live-pid dir kept" '[ -f "$base/wj-live-$$-0/sub/f" ]'
  check "recent dead-pid dir kept" '[ -f "$base/wj-recent-$dead-1/sub/f" ]'
  check "new .tmp dir kept" '[ -f "$base/.tmpNEW456/sub/f" ]'
  check "foreign name kept" '[ -f "$base/other-tool-$dead-0/sub/f" ]'
  check "wj- without pid kept" '[ -f "$base/wj-nopid/sub/f" ]'

  TMPDIR="$base" WJ_DISK_ALARM_GB=0 WJ_DISK_STOP_GB=0 bash "$ME" --disk >/dev/null; code=$?
  check "disk ok exits 0" '[ $code = 0 ]'
  TMPDIR="$base" WJ_DISK_ALARM_GB=99999999 WJ_DISK_STOP_GB=0 bash "$ME" --disk >/dev/null; code=$?
  check "disk alarm exits 10" '[ $code = 10 ]'
  TMPDIR="$base" WJ_DISK_ALARM_GB=99999999 WJ_DISK_STOP_GB=99999999 bash "$ME" --disk >/dev/null; code=$?
  check "disk hard stop exits 20" '[ $code = 20 ]'

  if [ "$fails" = 0 ]; then echo "selftest: $checks checks passed"; return 0; fi
  echo "selftest: $fails of $checks checks failed" >&2
  return 1
}

case "${1:-}" in
  "") reap_main 0 ;;
  --dry-run) reap_main 1 ;;
  --disk) disk_main ;;
  --selftest) selftest_main ;;
  -h|--help) sed -n '2,8p' "$ME" ;;
  *) echo "usage: $ME [--dry-run | --disk | --selftest]" >&2; exit 2 ;;
esac
