#!/usr/bin/env bash
# Run examples/verified through a portable backend and compare stdout/exit with the `// expect:` /
# `// exit:` oracle in each file.
#
#   scripts/verified_backends.sh <path-to-wj> go|js|ts [out.tsv]
#
# Result per example: PASS, FAIL (built, wrong output or status), BUILD (wj or the toolchain rejected it).
# Files named *_test.wj (`wj test` programs) are skipped. VERIFIED_WORK=<dir> keeps the build dirs.
set -uo pipefail
WJ="${1:?usage: verified_backends.sh <wj> go|js|ts [out.tsv]}"; BACKEND="${2:?backend}"; OUT="${3:-/dev/stdout}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -n "${VERIFIED_WORK:-}" ]; then W="$VERIFIED_WORK"; mkdir -p "$W"; else W="$(mktemp -d)"; trap 'rm -rf "$W"' EXIT; fi
: > "$OUT"
pass=0; fail=0; build=0
for f in "$ROOT"/examples/verified/*.wj; do
  n=$(basename "$f" .wj)
  case "$n" in *_test) continue;; esac
  d="$W/$n"; mkdir -p "$d"; cp "$f" "$d/$n.wj"
  sed -n 's#^// expect: \{0,1\}##p' "$f" > "$d/want.txt"
  want_exit=$(sed -n 's#^// exit: *##p' "$f" | head -1); want_exit="${want_exit:-0}"
  status=BUILD; got_exit=-1
  case "$BACKEND" in
    go) tgt=go;; js) tgt=javascript;; ts) tgt=typescript;;
  esac
  if "$WJ" build "$d/$n.wj" -t $tgt -o "$d/out" --no-lint > "$d/build.log" 2>&1; then
    case "$BACKEND" in
      go) (cd "$d/out" && go vet ./... && go build -o "$d/prog" .) >> "$d/build.log" 2>&1 && { "$d/prog" > "$d/got.txt" 2> "$d/err.txt"; got_exit=$?; status=RAN; } ;;
      js) (cd "$d/out" && node main.mjs > "$d/got.txt" 2> "$d/err.txt"); got_exit=$?; status=RAN ;;
      ts) (cd "$d/out" && node --experimental-strip-types --no-warnings main.ts > "$d/got.txt" 2> "$d/err.txt"); got_exit=$?; status=RAN ;;
    esac
  fi
  if [ $status = RAN ]; then
    if cmp -s "$d/want.txt" "$d/got.txt" && [ "$got_exit" = "$want_exit" ]; then status=PASS; pass=$((pass+1)); else status=FAIL; fail=$((fail+1)); fi
  else build=$((build+1)); fi
  printf '%s\t%s\n' "$n" "$status" >> "$OUT"
done
echo "backend=$BACKEND pass=$pass fail=$fail build-error=$build" >&2
