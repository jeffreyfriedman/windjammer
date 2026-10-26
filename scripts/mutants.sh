#!/usr/bin/env bash
# Mutation testing of the user-facing command layer (docs/testing/INVARIANTS.md, "Heavy, opt-in tools").
#
#   scripts/mutants.sh [FILE...]        default: src/cli/*.rs, src/bin/wj_cli_cmds.rs, src/linter/*.rs, src/test_runner/*.rs
#   MUTANTS_RE='regex' scripts/mutants.sh   only mutants whose name matches (small scopes on shared machines)
#
# Each mutant rebuilds the compiler and runs the acceptance suites (promise registry, fmt/fix semantics, check vs
# build, smoke) against it. Surviving mutants (tests stayed green) are written to docs/generated/mutants.txt:
# each one is a behavior no test pins down. Nightly / CI-optional; needs `cargo install cargo-mutants`.
# Runs in a scratch copy (cargo-mutants default) unless MUTANTS_IN_PLACE=1 (faster: reuses the target dir;
# never edit the tree while it runs).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
files=("$@")
[ ${#files[@]} -eq 0 ] && files=(src/cli/*.rs src/bin/wj_cli_cmds.rs src/linter/*.rs src/test_runner/*.rs)
args=(--no-shuffle --baseline skip --timeout 900 -j 1)
for f in "${files[@]}"; do args+=(--file "$f"); done
[ -n "${MUTANTS_RE:-}" ] && args+=(--re "$MUTANTS_RE")
[ "${MUTANTS_IN_PLACE:-0}" = 1 ] && args+=(--in-place)
out="$(mktemp -d)"
cargo mutants "${args[@]}" --output "$out" -- --release --test all -- \
  promise_registry_test:: fmt_fix_semantics_test:: check_build_differential_test:: installed_binary_smoke_test:: || true
mkdir -p docs/generated
{
  echo "# Surviving mutants (scripts/mutants.sh, $(date -u +%Y-%m-%d)): behavior no acceptance test pins down."
  echo "# scope: ${files[*]} ${MUTANTS_RE:+(re: $MUTANTS_RE)}"
  cat "$out/mutants.out/missed.txt" 2>/dev/null || true
} > docs/generated/mutants.txt
echo "caught: $(wc -l < "$out/mutants.out/caught.txt" 2>/dev/null || echo 0)  missed: $(wc -l < "$out/mutants.out/missed.txt" 2>/dev/null || echo 0)"
rm -rf "$out"
