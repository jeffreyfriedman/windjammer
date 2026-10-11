#!/usr/bin/env bash
# Fails when a test file exists that no runner reaches.
#
# Candidates (relative to the repo root):
#   - tests/**/*.rs         (build.rs wires top-level files; subdir files need a reference)
#   - tests/**/*.wj         (and *_test.wj, *_tests.wj, test_*.wj anywhere else)
#   - tests/**/*.sh         (runner scripts)
#   - tests/**/*.skip, *.disabled (disabled tests are never allowed)
#
# A candidate is "reached" when any of these holds:
#   1. top-level tests/*.rs (build.rs includes it in the `all` binary)
#   2. a `// consumes: <glob>` line in a tests/*.rs file matches it
#   3. its repo-relative path, or its basename, is mentioned in a reference file
#      (tests/*.rs, tests/common/*.rs, src/**/*.rs, scripts/*, Makefile, .github/**)
#   4. it matches an entry in tests/ORPHAN_ALLOWLIST.txt ("<glob> | <reason>")
#
# Usage: scripts/check-orphan-tests.sh [--list-allowed]
set -o pipefail  # portable to bash 3.2 (macOS): no mapfile/globstar/set -u on arrays

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
ALLOWLIST="tests/ORPHAN_ALLOWLIST.txt"

allow_globs=()
if [ -f "$ALLOWLIST" ]; then
    while IFS= read -r line; do
        case "$line" in ''|'#'*) continue ;; esac
        glob="${line%%|*}"; reason="${line#*|}"
        glob="$(echo "$glob" | sed 's/[[:space:]]*$//')"
        if [ "$reason" = "$line" ] || [ -z "$(echo "$reason" | tr -d '[:space:]')" ]; then
            echo "ERROR: $ALLOWLIST entry without a reason: $line" >&2
            exit 2
        fi
        allow_globs+=("$glob")
    done < "$ALLOWLIST"
fi

# Candidate files (tracked files only, so scratch outputs never count).
candidates=()
while IFS= read -r f; do candidates+=("$f"); done < <(git ls-files -- 'tests/**' 'crates/*/tests/**' '*_test.wj' '*_tests.wj' '*/test_*.wj' 'test_*.wj' 2>/dev/null \
    | grep -E '\.(rs|wj|sh|skip|disabled|bak)$' \
    | grep -vE '^tests/(ORPHAN_ALLOWLIST\.txt)$' | sort -u)

# Reference corpus: text in which a candidate may be mentioned.
corpus="$(mktemp)"; trap 'rm -f "$corpus"' EXIT
{
    cat tests/*.rs tests/common/*.rs 2>/dev/null
    git ls-files -- 'src/**/*.rs' 'scripts/*' 'Makefile' '.github/**' 'build.rs' | xargs cat 2>/dev/null
} > "$corpus"

# `// consumes: <glob>` declarations.
consumed=()
while IFS= read -r g; do
    consumed+=("$g")
done < <(grep -h 'consumes:' tests/*.rs 2>/dev/null \
    | sed -n -E 's|^[[:space:]]*//[/!]?[[:space:]]*consumes:[[:space:]]*([^[:space:]]+).*|\1|p')

# Glob match where a single `*` does not cross `/` (use `**` to cross directories).
matches_glob() { # path glob
    local p="$1" g="$2"
    # shellcheck disable=SC2254
    case "$p" in $g) ;; *) return 1 ;; esac
    case "$g" in *'**'*) return 0 ;; esac
    local ps="${p//[^\/]/}" gs="${g//[^\/]/}"
    [ "${#ps}" -eq "${#gs}" ]
}

# One pass: every file-like token mentioned in the corpus (path-ish words ending in a test extension).
mentioned="$(mktemp)"; trap 'rm -f "$corpus" "$mentioned"' EXIT
grep -oE '[A-Za-z0-9_./-]+\.(wj|rs|sh|skip|disabled)' "$corpus" | sort -u > "$mentioned"
# A candidate is mentioned when any of its path suffixes ("a/b/c.wj", "b/c.wj", "c.wj") is a token.
mentioned_cands="$(mktemp)"; trap 'rm -f "$corpus" "$mentioned" "$mentioned_cands"' EXIT
printf '%s\n' "${candidates[@]}" | awk '{
    n = split($0, p, "/"); s = "";
    for (i = n; i >= 1; i--) { s = (s == "" ? p[i] : p[i] "/" s); print s "\t" $0 }
}' | sort > "$mentioned_cands"
join -t "$(printf '\t')" "$mentioned" "$mentioned_cands" | cut -f2 | sort -u > "$mentioned_cands.hit"
mv "$mentioned_cands.hit" "$mentioned"

orphans=()
for f in "${candidates[@]}"; do
    case "$f" in
        *.skip|*.disabled|*.bak) orphans+=("$f  (disabled test file)"); continue ;;
    esac
    # 1. top-level rs
    if [[ "$f" =~ ^(crates/[^/]+/)?tests/[^/]+\.rs$ ]]; then continue; fi
    # 2. consumed
    hit=0
    for g in "${consumed[@]}"; do matches_glob "$f" "$g" && { hit=1; break; }; done
    [ $hit = 1 ] && continue
    # 4. allow-list
    for g in "${allow_globs[@]}"; do matches_glob "$f" "$g" && { hit=1; break; }; done
    [ $hit = 1 ] && continue
    # 3. mentioned
    if grep -qxF -- "$f" "$mentioned"; then continue; fi
    orphans+=("$f")
done

if [ "${#orphans[@]}" -gt 0 ]; then
    echo "ERROR: ${#orphans[@]} test file(s) are not reached by any runner:" >&2
    printf '  %s\n' "${orphans[@]}" >&2
    echo >&2
    echo "Fix: wire the file (top-level tests/*.rs, a '// consumes: <glob>' line in a tests/*.rs" >&2
    echo "runner, or a reference from a runner), delete it, or add '<glob> | <reason>' to $ALLOWLIST" >&2
    echo "(allow-list is for fixtures/helpers only)." >&2
    exit 1
fi
echo "OK: all ${#candidates[@]} candidate test files are reached by a runner."
