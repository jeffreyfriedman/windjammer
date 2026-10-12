#!/usr/bin/env bash
# Run the tests/spec examples through a compiled backend and compare with their `// expect:` oracle.
#
#   scripts/spec_backends.sh <path-to-wj> rust|go|js|ts [out.tsv]
#
# Needs rustc (rust), go (go: `go vet` must be clean too) or node >= 22.6 (js, ts: type stripping). Sequential on purpose (shared machines). Examples in
# `parse-ok` / `parse-error` mode are skipped: the compiled backends have no front-end-only mode.
# Result per example: PASS, FAIL (built, wrong output or status), BUILD (wj or the toolchain rejected it).
set -uo pipefail
WJ="${1:?usage: spec_backends.sh <wj> rust|go|js|ts [out.tsv]}"; BACKEND="${2:?backend}"; OUT="${3:-/dev/stdout}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# SPEC_WORK=<dir> keeps the per-example build directories for inspection.
if [ -n "${SPEC_WORK:-}" ]; then W="$SPEC_WORK"; mkdir -p "$W"; else W="$(mktemp -d)"; trap 'rm -rf "$W"' EXIT; fi
: > "$OUT"
pass=0; fail=0; build=0
while IFS= read -r f; do
  mode=$(sed -n 's#^// mode: *##p' "$f" | head -1); mode="${mode:-run}"
  case "$mode" in parse-ok|parse-error) continue;; esac
  id=$(sed -n 's#^// spec: *##p' "$f" | head -1)
  xf=$(sed -n 's#^// xfail: *##p' "$f" | head -1)
  want_exit=0; [ "$mode" = panic ] && { want_exit=$(sed -n 's#^// expect-exit: *##p' "$f" | head -1); want_exit="${want_exit:-101}"; }
  sed -n 's#^// expect: \{0,1\}##p' "$f" > "$W/want.txt"
  n=$(echo "$id" | tr -c 'A-Za-z0-9\n' '_'); d="$W/$n"; mkdir -p "$d"; cp "$f" "$d/$n.wj"
  status=BUILD; got_exit=-1
  if [ "$BACKEND" = rust ]; then
    if "$WJ" build "$d/$n.wj" -o "$d/out" --no-cargo > "$d/build.log" 2>&1 && [ -f "$d/out/$n.rs" ] \
       && rustc --edition 2021 -A warnings --crate-name spec -o "$d/prog" "$d/out/$n.rs" >> "$d/build.log" 2>&1; then
      "$d/prog" > "$d/got.txt" 2> "$d/err.txt"; got_exit=$?; status=RAN
    fi
  elif [ "$BACKEND" = go ]; then
    if "$WJ" build "$d/$n.wj" -t go -o "$d/out" > "$d/build.log" 2>&1 \
       && (cd "$d/out" && go vet ./... && go build -o "$d/prog" .) >> "$d/build.log" 2>&1; then
      "$d/prog" > "$d/got.txt" 2> "$d/err.txt"; got_exit=$?; status=RAN
    fi
  elif [ "$BACKEND" = ts ]; then
    if "$WJ" build "$d/$n.wj" -t typescript -o "$d/out" > "$d/build.log" 2>&1 && [ -f "$d/out/main.ts" ]; then
      (cd "$d/out" && node --experimental-strip-types --no-warnings main.ts > "$d/got.txt" 2> "$d/err.txt"); got_exit=$?; status=RAN
    fi
  else
    if "$WJ" build "$d/$n.wj" -t javascript -o "$d/out" > "$d/build.log" 2>&1 && [ -f "$d/out/main.mjs" ]; then
      (cd "$d/out" && node main.mjs > "$d/got.txt" 2> "$d/err.txt"); got_exit=$?; status=RAN
    fi
  fi
  if [ $status = RAN ]; then
    if cmp -s "$W/want.txt" "$d/got.txt" && [ "$got_exit" = "$want_exit" ]; then status=PASS; pass=$((pass+1)); else status=FAIL; fail=$((fail+1)); fi
  else build=$((build+1)); fi
  printf '%s\t%s\t%s\n' "$id" "$status" "${xf:--}" >> "$OUT"
done < <(find "$ROOT/tests/spec" -name '*.wj' | sort)
echo "backend=$BACKEND pass=$pass fail=$fail build-error=$build" >&2
